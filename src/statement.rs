use crate::util::FilePos;
use half::f16;
use std::fmt::Display;

// this file contains the struct definitions for internal representations of statements and
// datatypes

#[derive(PartialEq, Debug, Clone)]
pub enum DType {
    Pointer,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F16,
    F32,
    F64,
}

#[derive(Debug, Clone)]
pub struct Literal {
    value: LiteralValue,
    pos: FilePos,
}
impl Literal {
    pub fn new(value: LiteralValue, pos: FilePos) -> Literal {
        Literal { value, pos }
    }

    pub fn value(&self) -> &LiteralValue {
        &self.value
    }

    pub fn pos(&self) -> &FilePos {
        &self.pos
    }
}
impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "literal {:?}", self.value)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum LiteralValue {
    Pointer(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F16(f16),
    F32(f32),
    F64(f64),
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Push { value: Literal },
    Pop,
    Dup,
    Swap,
    Load { kind: DType },
    Store { kind: DType },
    Label { name: String },
    Jump { dest: String },
    Jumpif { dest: String },
    Call { dest: String },
    Ret,
    Cast { to: DType },
    Conv { to: DType },
    Proc { name: String, t_in: Vec<DType>, t_out: Vec<DType> },

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

#[derive(Clone)]
pub struct Statement {
    kind: StatementKind,
    pos: FilePos,
}
impl Statement {
    pub fn new(kind: StatementKind, pos: FilePos) -> Statement {
        Statement { kind, pos }
    }

    pub fn kind(&self) -> &StatementKind {
        &self.kind
    }

    pub fn pos(&self) -> &FilePos {
        &self.pos
    }

    // typing information methods
    // these give type information that can be known at compile time
    pub fn inputs(&self) -> Vec<Option<DType>> {
        unimplemented!()
    }
    pub fn outputs(&self) -> Vec<Option<DType>> {
        unimplemented!()
    }
    pub fn equals(&self) -> &[usize] {
        unimplemented!()
    }
}
impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let opt = match &self.kind {
            StatementKind::Push { value } => format!("Push {}", value),
            StatementKind::Load { kind } => format!("Load {:?}", kind),
            StatementKind::Store { kind } => format!("Store {:?}", kind),
            StatementKind::Label { name } => format!("Label {:?}", name),
            StatementKind::Jump { dest } => format!("Jump {:?}", dest),
            StatementKind::Jumpif { dest } => format!("Jumpif {:?}", dest),
            StatementKind::Call { dest } => format!("Call {:?}", dest),
            StatementKind::Cast { to } => format!("Cast {:?}", to),
            StatementKind::Conv { to } => format!("Conv {:?}", to),
            _ => format!("{:?}", self.kind)
        };
        write!(f, "{}", opt)
    }
}
