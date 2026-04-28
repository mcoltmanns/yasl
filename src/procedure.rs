use std::collections::HashMap;
use crate::statement::DType;
use crate::statement::Statement;
use crate::statement::StatementKind;
use crate::logger::Logger;
use crate::basicblock::BasicBlock;

// global procedure table
// struct ProcedureTable {
//     procedures: HashMap<String, Procedure>
// }
// impl ProcedureTable {
//     pub fn new() -> ProcedureTable {
//         ProcedureTable { procedures: HashMap::new() }
//     }
// 
//     pub fn get_mut(&mut self, name: &str) -> Option<&mut Procedure> {
//         self.procedures.get_mut(name)
//     }
// 
//     pub fn insert(&mut self, proc: Procedure) -> Option<Procedure> {
//         self.procedures.insert(proc.name.clone(), proc)
//     }
// }
pub type ProcedureTable = HashMap<String, Procedure>;

pub struct Procedure {
    name: String,
    inputs: Vec<DType>,
    outputs: Vec<DType>,
    blocks: Vec<BasicBlock>,
    statements: Vec<Statement>,
    // local table for jump labels
    jump_table: HashMap<String, usize>,
}

impl Procedure {
    pub fn new(name: String, inputs: Vec<DType>, outputs: Vec<DType>, statements: Vec<Statement>) -> Procedure {
        Procedure { name, inputs, outputs, blocks: vec![], statements, jump_table: HashMap::new() }
    }

    pub fn get_blocks(&self) -> &Vec<BasicBlock> {
        &self.blocks
    }

    pub fn get_jump(&self, label: &String) -> Option<&usize> {
        self.jump_table.get(label)
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn get_intypes(&self) -> &Vec<DType> {
        &self.inputs
    }

    pub fn get_outtypes(&self) -> &Vec<DType> {
        &self.outputs
    }

    pub fn set_statements(&mut self, sts: Vec<Statement>) {
        self.statements = sts
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn build_jumps_and_blocks(&mut self, logger: &mut dyn Logger) {
        // this is the first procedural definition pass
        // it builds the jump table and blocks out the statements into basic blocks
        // first we have to build the basic blocks, since the jump table maps label names to
        // basic block vector indices
        let mut current_block: Option<BasicBlock> = None;

        for (i, s) in self.statements.iter().enumerate() {
            match s.kind() {
                StatementKind::Label { name: _ } => {
                    // labels terminate the last block and start a new one
                    // labels are always at the start of a block
                    // if we were already working on a block, terminate it
                    if let Some(mut last_block) = current_block.take() {
                        last_block.length = i - last_block.start;
                        self.blocks.push(last_block);
                    }
                    // and start a new one
                    current_block = Some(BasicBlock { start: i, length: 0, predecessors: vec![], successors: vec![] });
                }
                StatementKind::Jump { dest: _ }
                | StatementKind::Jumpif { dest: _ }
                | StatementKind::Ret => {
                    // jumps and rets terminate the current block
                    // jumps and rets are always at the end of the block they terminate, so the
                    // indexing is a little different
                    // if we were already working on a block, terminate it and include this
                    // statement
                    // leave starting the next block open
                    if let Some(mut last_block) = current_block.take() {
                        last_block.length = (i + 1) - last_block.start;
                        self.blocks.push(last_block);
                    }
                    // if there was no previous block, create a block containing only this
                    // statement
                    else {
                        self.blocks.push(BasicBlock { start: i, length: 1, predecessors: vec![], successors: vec![] })
                    }
                }
                _ => {
                    // anything else starts a new block if we aren't already working on one
                    if current_block.is_none() {
                        current_block = Some(BasicBlock { start: i, length: 0, predecessors: vec![], successors: vec![] })
                    }
                }
            };
        }

        // when we're done, if we're still working on a block, close it
        // a procedure ending in an open block is an error, but we will catch this later
        if let Some(mut last_block) = current_block.take() {
            last_block.length = self.statements.len() - last_block.start;
            self.blocks.push(last_block);
        }

    }
}

