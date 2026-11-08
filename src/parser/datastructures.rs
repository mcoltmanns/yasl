use std::cmp::PartialEq;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use petgraph::{graph, Graph};
use petgraph::data::Build;
use petgraph::dot::{Config, Dot};
use petgraph::stable_graph::StableGraph;
use crate::logger::{LogEvent, Logbook};
use crate::primitives::{BasicBlock, DType, Literal};
use crate::util::{FilePos, Positioned};

#[derive(Debug)]
pub enum StackStatement {
    Push { value: Literal },
    Pop,
    Dup,
    Swap,
    Load { kind: DType },
    Store { kind: DType },
    Label { name: String },
    Jump { dest: String },
    Jumpif { dest: String },
    Call { dest: String, t_in: Vec<DType>, t_out: Vec<DType> },
    Ret { t_out: Vec<DType>},
    Cast { to: DType },
    Conv { to: DType },

    Add,
    Sub,
    Div,
    Mult,
    Mod,
    Inc,
    Dec,
    And,
    Or,
    Not,
    Xor,
    Bsl,
    Bsr,
    Rol,
    Ror,
    Eq,
    Neq,
    Lt,
    Leq,
    Gt,
    Geq,
}

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
    /// Map label names to node indices in the block graph
    label_map: HashMap<String, graph::NodeIndex>
}
impl StackProcedure {
    pub fn new(name: String, external: bool, types_in: Vec<DType>, types_out: Vec<DType>) -> Result<Self, String> { // yes this return type breaks with convention but an empty procedure can be created before we know its position
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
            label_map: HashMap::new(),
        })
    }

    /// Set statements for this procedure
    /// Also recomputes the basic blocks, their flow graph, and the jump table
    pub fn set_statements(&mut self, statements: Vec<Positioned<StackStatement>>) -> Result<(), LogEvent> {
        self.statements = statements;
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
        let mut edges = vec![];
        while let Some(block_idx) = block_idx_iter.next() {
            let block = &self.blocks[block_idx];
            let last_statement = &self.statements[block.start() + block.length() - 1];
            let last_statement_pos = last_statement.pos().clone(); // for error reporting
            match &**last_statement {
                StackStatement::Jump { dest } => {
                    // jumps have edges to their target
                    let dest_block_idx = self.label_map[dest];
                    edges.push((block_idx, dest_block_idx));
                }
                StackStatement::Jumpif { dest } => {
                    // jumpifs have edges to their target
                    let dest_block_idx = self.label_map[dest];
                    edges.push((block_idx, dest_block_idx));
                    // and a fallthrough to the next block
                    if let Some(&next_block_idx) = block_idx_iter.peek() {
                        edges.push((block_idx, next_block_idx));
                    }
                    else {
                        return Err(LogEvent::error("no fallthrough after conditional jump", last_statement_pos))
                    }
                }
                StackStatement::Ret { .. } => {}
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
        println!("{}", self.name);
        println!("{:?}", Dot::with_config(&self.blocks, &[Config::EdgeNoLabel]));
        Ok(())
    }

    pub fn is_extern(&self) -> bool {
        self.external
    }
}

pub struct StackProgram {
    procedures: Vec<Positioned<StackProcedure>>,
    entry_id: Option<usize>,
    nmi_id: Option<usize>,
    hwi_id: Option<usize>,
    swi_ids: [Option<usize>; 256],
    // constants are replaced during parsing and are declare-before use, so no need to track them further
}
impl Display for StackProgram {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl StackProgram {
    pub fn empty() -> StackProgram {
        StackProgram {
            procedures: Vec::new(),
            entry_id: None,
            nmi_id: None,
            hwi_id: None,
            swi_ids: [None; 256],
        }
    }

    pub fn add_procedure(&mut self, mut procedure: Positioned<StackProcedure>) -> Result<usize, LogEvent> {
        procedure.id = self.procedures.len();

        match procedure.kind {
            ProcedureKind::Main => {
                if self.entry_id.is_some() {
                    return Err(LogEvent::error("illegal redefinition of main procedure", procedure.pos().clone()));
                }
                self.entry_id = Some(procedure.id);
            }
            ProcedureKind::Default => {}
            ProcedureKind::Swi(id) => {
                if self.swi_ids[id as usize].is_some() {
                    return Err(LogEvent::error(&format!("illegal redefinition of swi handler {}", id), procedure.pos().clone()));
                }
                self.swi_ids[id as usize] = Some(procedure.id);
            }
            ProcedureKind::Nmi => {
                if self.nmi_id.is_some() {
                    return Err(LogEvent::error("illegal redefinition of nmi handler", procedure.pos().clone()));
                }
                self.nmi_id = Some(procedure.id);
            }
            ProcedureKind::Hwi => {
                if self.hwi_id.is_some() {
                    return Err(LogEvent::error("illegal redefinition of hwi handler", procedure.pos().clone()));
                }
                self.hwi_id = Some(procedure.id);
            }
        }

        self.procedures.push(procedure);

        Ok(self.procedures.len() - 1)
    }

    pub fn get_entry_id(&self) -> Result<usize, LogEvent> {
        self.entry_id.ok_or_else(|| { LogEvent::error("no entry point in program", FilePos::default()) })
    }

    pub fn get_entry(&self) -> Option<&Positioned<StackProcedure>> {
        if let Some(entry_id) = self.entry_id {
            return self.procedures.get(entry_id)
        }
        None
    }

    pub fn get_procedures(&self) -> &[Positioned<StackProcedure>] {
        &self.procedures
    }
}
