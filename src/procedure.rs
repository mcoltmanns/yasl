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

    pub fn set_blocks(&mut self, blocks: Vec<BasicBlock>) {
        self.blocks = blocks
    }

    pub fn get_blocks(&self) -> &Vec<BasicBlock> {
        &self.blocks
    }

    pub fn insert_jump(&mut self, label: String, target_block_index: usize) -> Option<usize> {
        self.jump_table.insert(label, target_block_index)
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

    pub fn statements(&self) -> &Vec<Statement> {
        &self.statements
    }
}

