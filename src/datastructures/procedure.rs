use std::collections::HashMap;
use std::fmt::Display;

use crate::datastructures::basicblock::BasicBlock;
use crate::datastructures::statement::DType;
use crate::datastructures::statement::StatementPayload;
use crate::datastructures::statement::Statement;
use crate::logger::Logger;
use crate::util::Positionable;

#[derive(Debug)]
pub struct Procedure {
    name: String,
    types_in: Vec<DType>,
    types_out: Vec<DType>,

    jump_table: HashMap<String, usize>,
    blocks: Vec<BasicBlock>,
    // (from, to)
    block_links: Vec<Vec<bool>>,
    statements: Vec<Statement>,
}

impl Procedure {
    pub fn empty(name: String, types_in: Vec<DType>, types_out: Vec<DType>) -> Self {
        Procedure { name, types_in, types_out, jump_table: HashMap::new(), blocks: vec![], block_links: vec![], statements: vec![] }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn blocks(&self) -> &[BasicBlock] {
        &self.blocks
    }

    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    pub fn set_statements(&mut self, statements: Vec<Statement>) {
        self.statements = statements
    }

    pub fn types_in(&self) -> &[DType] {
        &self.types_in
    }

    pub fn types_out(&self) -> &[DType] {
        &self.types_out
    }

    pub fn view_block(&self, block_i: usize) -> Option<&[Statement]> {
        let block = self.blocks().get(block_i)?;
        Some(&self.statements()[block.start()..block.start() + block.length()])
    }

    pub fn build_blocks_and_jumps(&mut self, logger: &mut dyn Logger) {
        let mut current_block: Option<BasicBlock> = None;

        for (i, s) in self.statements.iter().enumerate() {
            match s.payload() {
                // labels start a new block and end the last one
                // labels are included in the new block
                StatementPayload::Label { name } => {
                    if let Some(mut last_block) = current_block.take() {
                        last_block.set_length(i - last_block.start());
                        self.blocks.push(last_block);
                    }
                    current_block = Some(BasicBlock::new(i, 0, s.pos().clone()));
                    // also add to the jump table
                    if self.jump_table.insert(name.clone(), self.blocks().len()).is_some() {
                        logger.error("duplicate label", s.pos().clone());
                    }
                }
                // control flow changes start a new block and end the last one
                // control flow changes are included in the old block
                StatementPayload::Jump { .. }
                | StatementPayload::Jumpif { .. }
                | StatementPayload::Ret => {
                    if let Some(mut last_block) = current_block.take() {
                        last_block.set_length(i + 1 - last_block.start());
                        self.blocks.push(last_block);
                    }
                    else {
                        self.blocks.push(BasicBlock::new(i, 1, s.pos().clone()));
                    }
                }
                // anything else starts a new block if we aren't already on one
                _ => {
                    if current_block.is_none() {
                        current_block = Some(BasicBlock::new(i, 0, s.pos().clone()))
                    }
                }
            }
        }
        // close last block
        if let Some(mut last_block) = current_block.take() {
            last_block.set_length(self.statements.len() - last_block.start());
            self.blocks.push(last_block);
        }

        self.block_links = vec![vec![false; self.blocks.len()]; self.blocks.len()];
    }

    pub fn link_blocks(&mut self, logger: &mut dyn Logger) {
        for b_i in 0..self.blocks.len() {
            // last statement in a block determines successors
            let s = self.view_block(b_i).expect("block index out of range").last().expect("no statements in block").clone(); // this clone is ok, it's only one statement
            match s.payload() {
                // jumps are succeeded by block they call
                StatementPayload::Jump { dest } => {
                    if let Some(successor_id) = self.jump_table.get(dest) {
                        self.block_links[b_i][*successor_id] = true;
                    }
                    else {
                        logger.error("invalid jump destination", s.pos().clone());
                    }
                }
                // jumpifs are succeeded by block they call, and block after them
                StatementPayload::Jumpif { dest } => {
                    if let Some(successor_id) = self.jump_table.get(dest) {
                        self.block_links[b_i][*successor_id] = true;
                    }
                    else {
                        logger.error("invalid jump destination", s.pos().clone());
                    }
                    let next_id = b_i + 1;
                    if next_id < self.blocks.len() {
                        self.block_links[b_i][next_id] = true;
                    }
                    else {
                        logger.error("no fallthrough block after conditional jump", s.pos().clone());
                    }
                }
                // rets have no successors
                StatementPayload::Ret => {}
                // anything else is succeeded by the block after if
                _ => {
                    let next_id = b_i + 1;
                    if next_id < self.blocks.len() {
                        self.block_links[b_i][next_id] = true;
                    }
                }
            }
        }
    }

    pub fn check_block_reachability(&self, logger: &mut dyn Logger) {
        println!("{:?}", self.block_links);
        for b_i in 1..self.blocks.len() {
            // first block is always reachable, so we skip it
            let mut reachable = false;
            // you are reachable if there is a link from any other to you
            for other_i in 0..self.blocks.len() {
                // skip self links
                if b_i == other_i { continue; }
                reachable |= self.block_links[other_i][b_i];
            }
            if !reachable {
                logger.warning("unreachable code", self.blocks[b_i].pos().clone());
            }
        }
    }
}

impl Display for Procedure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = format!("Procedure {}\nIn: {:?}\nOut: {:?}\n", self.name, self.types_in, self.types_out);
        for statement in &self.statements {
            s.push_str(&format!("  {}\n", statement));
        }
        write!(f, "{}", s)
    }
}
