use half::f16;
use std::fmt::Display;

use crate::util::{FilePos, Positionable};

#[derive(Debug)]
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
impl DType {
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::U8 | Self::U16 | Self::U32 | Self::U64)
    }
}
impl From<&Literal> for DType {
    fn from(value: &Literal) -> Self {
        match value {
            Literal::Pointer(_) => Self::Pointer,
            Literal::I8(_) => Self::I8,
            Literal::I16(_) => Self::I16,
            Literal::I32(_) => Self::I32,
            Literal::I64(_) => Self::I64,
            Literal::U8(_) => Self::U8,
            Literal::U16(_) => Self::U16,
            Literal::U32(_) => Self::U32,
            Literal::U64(_) => Self::U64,
            Literal::F16(_) => Self::F16,
            Literal::F32(_) => Self::F32,
            Literal::F64(_) => Self::F64,
        }
    }
}

#[derive(Debug)]
pub enum Literal {
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

#[derive(Debug)]
pub enum StatementPayload {
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

pub struct Statement {
    payload: StatementPayload,
    pos: FilePos,
}
impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let opt = match &self.payload {
            StatementPayload::Push { value } => format!("Push {:?}", value),
            StatementPayload::Load { kind } => format!("Load {:?}", kind),
            StatementPayload::Store { kind } => format!("Store {:?}", kind),
            StatementPayload::Label { name } => format!("Label {:?}", name),
            StatementPayload::Jump { dest } => format!("Jump {:?}", dest),
            StatementPayload::Jumpif { dest } => format!("Jumpif {:?}", dest),
            StatementPayload::Call { dest } => format!("Call {:?}", dest),
            StatementPayload::Cast { to } => format!("Cast {:?}", to),
            StatementPayload::Conv { to } => format!("Conv {:?}", to),
            _ => format!("{:?}", self.payload)
        };
        write!(f, "{}", opt)
    }
}
impl Positionable for Statement {
    fn pos(&self) -> &FilePos {
        &self.pos
    }
    fn col(&self) -> usize {
        self.pos.col
    }
    fn line(&self) -> usize {
        self.pos.line
    }
}
