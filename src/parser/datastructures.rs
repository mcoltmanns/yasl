use std::collections::HashMap;
use crate::datastructures::statement::{DType, Literal};
use crate::util::Positioned;

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

pub struct StackProcedure {
    pub name: String,
    /// The types this procedure expects
    pub types_in: Vec<DType>,
    /// The types this procedure leaves
    pub types_out: Vec<DType>,
    /// Map labels in this procedure to indices into the basic block array
    pub jump_table: HashMap<String, usize>,
    /// Basic blocks in this procedure
    pub blocks: Vec<BasicBlock>,
    /// Which blocks follow which?
    /// block_links[a, b] == true -> b follows a
    pub block_links: Vec<Vec<bool>>,
    /// Statements in this procedure
    pub statements: Vec<StackStatement>
}

pub struct StackProgram {
    /// Entry point
    pub(crate) entry: Positioned<StackProcedure>,
    /// Non-maskable hardware interrupt handler
    pub(crate) nmi: Option<Positioned<StackProcedure>>,
    /// Maskable hardware interrupt handler
    pub(crate) hwi: Option<Positioned<StackProcedure>>,
    /// Table of maskable software interrupt handlers
    pub(crate) swi_table: [Option<Positioned<StackProcedure>>; 256],
    /// Non-interrupt-handling procedure table
    pub(crate) proc_table: HashMap<String, Positioned<StackProcedure>>,
    /// Constant table
    pub(crate) constants: HashMap<String, Literal>
}
