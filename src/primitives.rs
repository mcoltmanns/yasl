/// File for general primitive datatypes
use half::f16;
use crate::tokenizer::datastructures::Token;

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
        match t {
            Token::Literal(st) => {
                let (repr, radix) =
                    if let Some(s) = st.strip_prefix("0x") {
                        (s, 16)
                    } else if let Some(s) = st.strip_prefix("0b") {
                        (s, 2)
                    } else {
                        (st.as_str(), 10)
                    };

                match dtype {
                    DType::I8 => i8::from_str_radix(repr, radix).map(Literal::I8).map_err(|e| format!("could not parse i8: {}", e.to_string())),
                    DType::I16 => i16::from_str_radix(repr, radix).map(Literal::I16).map_err(|e| format!("could not parse i16: {}", e.to_string())),
                    DType::I32 => i32::from_str_radix(repr, radix).map(Literal::I32).map_err(|e| format!("could not parse i32: {}", e.to_string())),
                    DType::I64 => i64::from_str_radix(repr, radix).map(Literal::I64).map_err(|e| format!("could not parse i64: {}", e.to_string())),
                    DType::U8 => u8::from_str_radix(repr, radix).map(Literal::U8).map_err(|e| format!("could not parse u8: {}", e.to_string())),
                    DType::U16 => u16::from_str_radix(repr, radix).map(Literal::U16).map_err(|e| format!("could not parse u16: {}", e.to_string())),
                    DType::U32 => u32::from_str_radix(repr, radix).map(Literal::U32).map_err(|e| format!("could not parse u32: {}", e.to_string())),
                    DType::U64 => u64::from_str_radix(repr, radix).map(Literal::U64).map_err(|e| format!("could not parse u64: {}", e.to_string())),
                    DType::F16 => repr.parse::<f16>().map(Literal::F16).map_err(|e| format!("could not parse f16: {}", e.to_string())),
                    DType::F32 => repr.parse::<f32>().map(Literal::F32).map_err(|e| format!("could not parse f32: {}", e.to_string())),
                    DType::F64 => repr.parse::<f64>().map(Literal::F64).map_err(|e| format!("could not parse f64: {}", e.to_string())),
                    DType::Pointer => u64::from_str_radix(repr, radix).map(Literal::Pointer).map_err(|e| format!("could not parse pointer: {}", e.to_string())),
                }
            }
            _ => {
                Err("cannot parse literal from nonliteral token".to_string())
            }
        }
    }

    /// return the value in this literal as a vector of bytes in little-endian order
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::U8(v) => v.to_le_bytes().to_vec(),
            Self::U16(v) => v.to_le_bytes().to_vec(),
            Self::U32(v) => v.to_le_bytes().to_vec(),
            Self::U64(v) => v.to_le_bytes().to_vec(),
            Self::I8(v) => v.to_le_bytes().to_vec(),
            Self::I16(v) => v.to_le_bytes().to_vec(),
            Self::I32(v) => v.to_le_bytes().to_vec(),
            Self::I64(v) => v.to_le_bytes().to_vec(),
            Self::F16(v) => v.to_le_bytes().to_vec(),
            Self::F32(v) => v.to_le_bytes().to_vec(),
            Self::F64(v) => v.to_le_bytes().to_vec(),
            Self::Pointer(v) => v.to_le_bytes().to_vec()
        }
    }
}

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
        match t {
            Token::IType(w) => {
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
            Token::UType(w) => {
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
            Token::FType(w) => {
                match w {
                    16 => Ok(Self::F16),
                    32 => Ok(Self::F32),
                    64 => Ok(Self::F64),
                    _ => {
                        Err(format!("invalid float width {}", w))
                    }
                }
            }
            Token::PtrType => Ok(Self::Pointer),
            _ => {
                Err(format!("unknown type {:?}", t))
            }
        }
    }
}
impl From<&Literal> for DType {
    /// Do enum-unwrapping to strip type information from a literal
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicBlock {
    /// All programs are a map of procedures, and each procedure is a vec of statements
    /// So every basic block in a program can be defined by its parent procedure's id, its starting index, and its length
    procedure_id: usize,
    start: usize,
    length: usize,
}
impl BasicBlock {
    pub fn empty(procedure_id: usize, start: usize) -> Self {
        Self { procedure_id, start, length: 0 }
    }

    pub fn procedure_id(&self) -> usize {
        self.procedure_id
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn set_length(&mut self, length: usize) {
        self.length = length;
    }

    pub fn length(&self) -> usize {
        self.length
    }
}
