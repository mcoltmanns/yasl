use std::collections::HashMap;
use petgraph::Direction;
use petgraph::prelude::StableGraph;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::Visitable;
use crate::logger::LogEvent;
use crate::primitives::{BasicBlock, DType};
use crate::util::Positioned;
use crate::irs::stack::statement::StackStatement;

#[derive(Debug)]
pub enum ProcedureKind { Main, Default, Swi(u8), Hwi, Nmi }
impl PartialEq for ProcedureKind {
    fn eq(&self, other: &Self) -> bool {
        match (self,other) {
            (&ProcedureKind::Main, &ProcedureKind::Main) => true,
            (&ProcedureKind::Default, &ProcedureKind::Default) => true,
            (&ProcedureKind::Hwi, &ProcedureKind::Hwi) => true,
            (&ProcedureKind::Nmi, &ProcedureKind::Nmi) => true,
            (&ProcedureKind::Swi(a), &ProcedureKind::Swi(b)) => a == b,
            _ => false,
        }
    }
}

pub struct StackProcedure {
    name: String,
    kind: ProcedureKind,
    id: usize,
    external: bool,
    /// The types this procedure expects
    types_in: Vec<DType>,
    /// The types this procedure leaves
    types_out: Vec<DType>,
    /// Statements in this procedure
    statements: Vec<Positioned<StackStatement>>,
    /// Basic blocks in this procedure
    blocks: StableGraph<BasicBlock, ()>, // use stablegraph so block ids aren't invalidated upon delete
    entry_block_id: Option<NodeIndex>,
    /// Map label names to node indices in the block graph
    label_map: HashMap<String, NodeIndex>
}
impl StackProcedure {
    pub fn new(name: String, external: bool, types_in: Vec<DType>, types_out: Vec<DType>) -> Result<Self, String> { // yes this return type breaks with convention but an empty procedure can be created before we know its position, so we can't return a LogEvent
        let mut kind: ProcedureKind = ProcedureKind::Default;
        match name.as_str() {
            "main" => {
                kind = ProcedureKind::Main;
                if external {
                    return Err("main procedure may not be marked external".to_string())
                }
            }
            "hwi" => {
                kind = ProcedureKind::Hwi;
                if external {
                    return Err("hwi procedure may not be marked external".to_string())
                }
            }
            "nmi" => {
                kind = ProcedureKind::Nmi;
                if external {
                    return Err("nmi procedure may not be marked external".to_string())
                }
            }
            _ => {
                if name.starts_with("swi") {
                    let id = name.trim_start_matches("swi").parse::<u8>().map_err(
                        |_| format!("invalid swi handler id: {}", name.trim_start_matches("swi"))
                    )?;
                    kind = ProcedureKind::Swi(id);
                }
            }
        }
        Ok(StackProcedure {
            name,
            kind,
            external,
            id: usize::MAX,
            types_in,
            types_out,
            statements: Vec::new(),
            blocks: StableGraph::new(),
            entry_block_id: None,
            label_map: HashMap::new(),
        })
    }

    /// Tries to build a control flow graph for this procedure
    /// If fails, returns a LogEvent explaining why
    /// If it succeeds, flow through this procedure is well-formed
    pub fn build_flow_graph(&mut self) -> Result<&StableGraph<BasicBlock, ()>, LogEvent> {
        self.blocks.clear();
        self.label_map.clear();
        // do three passes:
        // first pass finds starts of blocks (0th statement, labels, statements after a control flow change)
        // second pass builds blocks and adds them to graph, maps labels to graph node ids
        // third pass builds edges: look at a block's last statement and decide
        // first pass
        let mut block_start_idxs = vec![0];
        for statement_idx in 1..self.statements.len() {
            let last_statement_idx = statement_idx - 1;
            let statement = &*self.statements[statement_idx];
            let last_statement = &*self.statements[last_statement_idx];
            // if you are a label, you are a block start
            if let StackStatement::Label { .. } = statement {
                block_start_idxs.push(statement_idx);
                continue;
            }
            // if the last statement was a jump, jumpif, or ret (control flow change), you are a block start
            match last_statement {
                StackStatement::Jump { .. } |
                StackStatement::Jumpif { .. } |
                StackStatement::Ret { .. } => {
                    block_start_idxs.push(statement_idx);
                }
                _ => {}
            }
        }
        // second pass
        // now that we know where our blocks start, building the block graph is pretty simple
        let mut block_start_iter = block_start_idxs.iter().peekable();
        while let Some(&block_start_idx) = block_start_iter.next() {
            let mut block = BasicBlock::empty(block_start_idx);
            if let Some(&&next_start_idx) = block_start_iter.peek() {
                // if there is a next block your end is its start
                block.set_length(next_start_idx - block_start_idx);
            }
            else {
                // if there isn't, your end is the last statement
                block.set_length(self.statements.len() - block_start_idx);
            }
            // save node to the graph
            let node_idx = self.blocks.add_node(block);
            // if the entry id is none, set as your entry id (we iterate in source order so this is correct)
            if self.entry_block_id.is_none() {
                self.entry_block_id = Some(node_idx);
            }
            // if your first statement is a label, save your graph id for the third pass
            let first_statement = &*self.statements[block_start_idx];
            if let StackStatement::Label { name } = first_statement {
                self.label_map.insert(name.clone(), node_idx);
            }
        }
        // third pass
        // look at the last statement of each block in the graph
        // jumpifs have edges to their target and a fallthrough edge
        // rets have no edges
        // everything else has a fallthrough edge
        let mut block_idx_iter = self.blocks.node_indices().peekable(); // this iterator is in the order we added our blocks, so in source order
        let mut edges = vec![]; // have to collect the edges into an array first because we can't modify the graph while iterating over node indices
        while let Some(block_idx) = block_idx_iter.next() {
            let block = &self.blocks[block_idx];
            let last_statement_pos = self.statements[block.start() + block.length() - 1].pos().clone(); // for error reporting
            let last_statement = &mut *self.statements[block.start() + block.length() - 1];
            match last_statement {
                StackStatement::Jump { dest_name: dest, dest_idx } => {
                    // jumps have edges to their target
                    if let Some(&dest_block_idx) = self.label_map.get(dest) {
                        *dest_idx = self.blocks[dest_block_idx].start().into(); // point the statement to the index of the first statement of the destination block
                        edges.push((block_idx, dest_block_idx));
                    }
                    else {
                        return Err(LogEvent::error(&format!("undefined jump target \"{}\"", dest), last_statement_pos.clone()))
                    }
                }
                StackStatement::Jumpif { dest_name: dest, dest_idx  } => {
                    // jumpifs have edges to their target
                    if let Some(&dest_block_idx) = self.label_map.get(dest) {
                        *dest_idx = self.blocks[dest_block_idx].start().into(); // point the statement to the index of the first statement of the destination block
                        edges.push((block_idx, dest_block_idx));
                    }
                    else {
                        return Err(LogEvent::error(&format!("undefined jump target \"{}\"", dest), last_statement_pos.clone()))
                    }
                    // and a fallthrough to the next block
                    if let Some(&next_block_idx) = block_idx_iter.peek() {
                        edges.push((block_idx, next_block_idx));
                    }
                    else {
                        return Err(LogEvent::error("no fallthrough after conditional jump", last_statement_pos))
                    }
                }
                StackStatement::Ret { .. } => {} // returns don't need aligning because they return to their calling scope, we can figure this out implicitly
                _ => {
                    if let Some(&next_block_idx) = block_idx_iter.peek() {
                        edges.push((block_idx, next_block_idx));
                    }
                    else {
                        return Err(LogEvent::error("non-terminating statement does not have a successor", last_statement_pos))
                    }
                }
            }
        }
        for e in edges.iter() {
            self.blocks.add_edge(e.0, e.1, ());
        }
        //println!("{}", self.name);
        //println!("{:?}", Dot::with_config(&self.blocks, &[Config::EdgeNoLabel]));
        Ok(&self.blocks)
    }

    /// Get all blocks that can be reached in this procedure
    pub fn reachable_blocks(&self) -> Vec<BasicBlock> {
        let mut reachable = vec![self.blocks[self.entry_block_id.expect("procedure has no entry block, did you remember to build the flow graph?")]];
        for block_idx in self.blocks.node_indices() {
            let neighbors_incoming = self.blocks.neighbors_directed(block_idx, Direction::Incoming).collect::<Vec<_>>();
            // blocks with no incoming edges are not reachable
            if neighbors_incoming.len() == 0 {
                continue;
            }
            // blocks with exactly one incoming edge which is that block are not reachable
            else if neighbors_incoming.len() == 1 && neighbors_incoming[0] == block_idx {
                continue;
            }
            // any other block is reachable
            reachable.push(self.blocks[block_idx].clone());
        }
        reachable
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> &ProcedureKind {
        &self.kind
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn external(&self) -> bool {
        self.external
    }

    pub fn types_in(&self) -> &Vec<DType> {
        &self.types_in
    }

    pub fn types_out(&self) -> &Vec<DType> {
        &self.types_out
    }

    pub fn statements(&self) -> &Vec<Positioned<StackStatement>> {
        &self.statements
    }

    pub fn statements_mut(&mut self) -> &mut Vec<Positioned<StackStatement>> {
        &mut self.statements
    }

    pub fn label_map(&self) -> &HashMap<String, NodeIndex> {
        &self.label_map
    }

    pub fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    pub fn set_statements(&mut self, statements: Vec<Positioned<StackStatement>>) {
        self.statements = statements;
    }

    pub fn statements_in_block(&self, basic_block: BasicBlock) -> &[Positioned<StackStatement>] {
        &self.statements[basic_block.start() .. basic_block.start() + basic_block.length()]
    }

    pub fn statements_in_block_mut(&mut self, basic_block: BasicBlock) -> &mut [Positioned<StackStatement>] {
        &mut self.statements[basic_block.start() .. basic_block.start() + basic_block.length()]
    }
}
