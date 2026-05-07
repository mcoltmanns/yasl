use half::f16;
use std::{fmt::Display};

use crate::{datastructures::token::{Token, TokenPayload}, regmachine::VReg, util::{FilePos, Positionable}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn from_token(t: &Token) -> Result<Self, String> {
        match t.payload() {
            TokenPayload::IType(w) => {
                match w {
                    8 => Ok(Self::I8),
                    16 => Ok(Self::I16),
                    32 => Ok(Self::I32),
                    64 => Ok(Self::I64),
                    _ => {
                        Err(format!("invalid integer width {}", w))
                    }
                }
            }
            TokenPayload::UType(w) => {
                match w {
                    8 => Ok(Self::U8),
                    16 => Ok(Self::U16),
                    32 => Ok(Self::U32),
                    64 => Ok(Self::U64),
                    _ => {
                        Err(format!("invalid unsigned integer width {}", w))
                    }
                }
            }
            TokenPayload::FType(w) => {
                match w {
                    16 => Ok(Self::F16),
                    32 => Ok(Self::F32),
                    64 => Ok(Self::F64),
                    _ => {
                        Err(format!("invalid float width {}", w))
                    }
                }
            }
            TokenPayload::PtrType => Ok(Self::Pointer),
            _ => {
                Err(format!("unknown type {}", t))
            }
        }
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

#[derive(Debug, Clone)]
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
impl Literal {
    pub fn from_token(t: &Token, dtype: &DType) -> Result<Self, String> {
        match t.payload() {
            TokenPayload::Literal(st) => {
                let (repr, radix) =
                if let Some(s) = st.strip_prefix("0x") {
                    (s, 16)
                } else if let Some(s) = st.strip_prefix("0b") {
                    (s, 2)
                } else {
                    (st.as_str(), 10)
                };

                match dtype {
                    DType::I8 => i8::from_str_radix(repr, radix).map(Literal::I8).map_err(|e| e.to_string()),
                    DType::I16 => i16::from_str_radix(repr, radix).map(Literal::I16).map_err(|e| e.to_string()),
                    DType::I32 => i32::from_str_radix(repr, radix).map(Literal::I32).map_err(|e| e.to_string()),
                    DType::I64 => i64::from_str_radix(repr, radix).map(Literal::I64).map_err(|e| e.to_string()),
                    DType::U8 => u8::from_str_radix(repr, radix).map(Literal::U8).map_err(|e| e.to_string()),
                    DType::U16 => u16::from_str_radix(repr, radix).map(Literal::U16).map_err(|e| e.to_string()),
                    DType::U32 => u32::from_str_radix(repr, radix).map(Literal::U32).map_err(|e| e.to_string()),
                    DType::U64 => u64::from_str_radix(repr, radix).map(Literal::U64).map_err(|e| e.to_string()),
                    DType::F16 => repr.parse::<f16>().map(Literal::F16).map_err(|e| e.to_string()),
                    DType::F32 => repr.parse::<f32>().map(Literal::F32).map_err(|e| e.to_string()),
                    DType::F64 => repr.parse::<f64>().map(Literal::F64).map_err(|e| e.to_string()),
                    DType::Pointer => u64::from_str_radix(repr, radix).map(Literal::Pointer).map_err(|e| e.to_string()),
                }
            }
            _ => {
                Err("cannot parse literal from nonliteral token".to_string())
            }
        }
    }
}

#[derive(Debug, Clone)]
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
    Trap { vec: Literal, t_in: Vec<DType>, t_out: Vec<DType> },

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

#[derive(Debug, Clone)]
pub struct VirtualStatement {
    payload: StatementPayload,
    pos: FilePos,
    math_type: Option<DType>,
}
impl VirtualStatement {
    pub fn new(payload: StatementPayload, pos: FilePos) -> Self {
        Self { payload, pos, math_type: None }
    }

    pub fn payload(&self) -> &StatementPayload {
        &self.payload
    }

    pub fn set_type(&mut self, t: DType) {
        self.math_type = Some(t);
    }

    pub fn math_type(&self) -> &Option<DType> {
        &self.math_type
    }
}
impl Display for VirtualStatement {
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
impl Positionable for VirtualStatement {
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

// args are always dest arg1 arg2
#[derive(Debug)]
pub enum VRegInstruction {
    // load a literal to a register
    LoadImm { dest: VReg, val: Literal },
    // load a register from memory
    LoadMem { dest: VReg, addr: VReg },
    // store a register to memory
    Store { addr: VReg, src: VReg },

    // copy a value from one register to another
    Move { dest: VReg, src: VReg },

    // cast a value in a register
    // this might change the bits stored
    Cast { dest: VReg, src: VReg, to: DType },
    // conv does not need an instruction, we just move the value into a new register of the
    // destination type
    
    Label { name: String },
    // jump to a label
    Jump { dest: String },
    // jump to a label if the value in cmp is not 0
    Jumpif { dest: String, cmp: VReg },
    // jump to a label and push a return address to the stack
    Call { dest: String, inputs: Vec<VReg>, outputs: Vec<VReg> },
    // pop the stack and jump to that address to continue execution
    Ret { regs: Vec<VReg> },

    // we don't need to type operations, because their type can be determined from the registers
    // they operate on
    Add { dest: VReg, a: VReg, b: VReg },
    Sub { dest: VReg, a: VReg, b: VReg },
    Div { dest: VReg, a: VReg, b: VReg },
    Mul { dest: VReg, a: VReg, b: VReg }, 
    Mod { dest: VReg, a: VReg, b: VReg },
    Inc { dest: VReg, a: VReg },
    Dec { dest: VReg, a: VReg },
    And { dest: VReg, a: VReg, b: VReg },
    Or  { dest: VReg, a: VReg, b: VReg },
    Not { dest: VReg, a: VReg },
    Xor { dest: VReg, a: VReg, b: VReg },
    Bsl { dest: VReg, a: VReg },
    Bsr { dest: VReg, a: VReg },
    Rol { dest: VReg, a: VReg },
    Ror { dest: VReg, a: VReg },
    Eq  { dest: VReg, a: VReg, b: VReg },
    Neq { dest: VReg, a: VReg, b: VReg },
    Lt  { dest: VReg, a: VReg, b: VReg },
    Leq { dest: VReg, a: VReg, b: VReg },
    Gt  { dest: VReg, a: VReg, b: VReg },
    Geq { dest: VReg, a: VReg, b: VReg },
}
impl VRegInstruction {
    pub fn registers(&self) -> Vec<VReg> {
        match self {
            Self::Add { dest, a, b }
            | Self::Sub { dest, a, b }
            | Self::Mul { dest, a, b }
            | Self::Div { dest, a, b }
            | Self::Mod { dest, a, b }
            | Self::And { dest, a, b }
            | Self::Or { dest, a, b }
            | Self::Xor { dest, a, b }
            | Self::Eq { dest, a, b }
            | Self::Neq { dest, a, b }
            | Self::Lt { dest, a, b }
            | Self::Leq { dest, a, b }
            | Self::Gt { dest, a, b }
            | Self::Geq { dest, a, b } => vec![*dest, *a, *b],
            Self::Inc { dest, a }
            | Self::Dec { dest, a }
            | Self::Not { dest, a }
            | Self::Bsl { dest, a }
            | Self::Bsr { dest, a }
            | Self::Rol { dest, a }
            | Self::Ror { dest, a } => vec![*dest, *a],
            Self::Cast { dest, src, .. }
            | Self::Move { src, dest } => vec![*src, *dest],
            Self::LoadMem { addr, dest } => vec![*addr, *dest],
            Self::Store { addr, src, .. } => vec![*addr, *src],
            Self::Jumpif { dest: _, cmp } => vec![*cmp],
            Self::Ret { regs } => regs.clone(),
            Self::Call { inputs, .. } => inputs.clone(),
            Self::LoadImm { dest, .. } => vec![*dest],
            | Self::Jump { .. }
            | Self::Label { .. } => vec![],
        }
    }
}
