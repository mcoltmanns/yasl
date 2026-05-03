use std::fmt::Display;
use crate::util::{FilePos, Positionable};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TokenPayload {
    Unknown(String),
    Eof,

    Const,
    Push,
    Pop,
    Dup,
    Swap,
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
    Load,
    Store,
    Label,
    Jump,
    Jumpif,
    Call,
    Ret,
    Cast,
    Conv,
    Proc,
    In,
    Out,
    Def,

    // these are all literals
    // tokenizer does not parse them, only extract
    Name(String),
    Literal(String),

    // these are types
    // payload denotes width
    IType(u8),
    UType(u8),
    FType(u8),
    PtrType
}
impl From<&str> for TokenPayload {
    fn from(value: &str) -> Self {
        match value {
            "" => Self::Eof,
            "const" => Self::Const,
            "push" => Self::Push,
            "pop" => Self::Pop,
            "dup" => Self::Dup,
            "swap" => Self::Swap,
            "add" => Self::Add,
            "sub" => Self::Sub,
            "mult" => Self::Mult,
            "div" => Self::Div,
            "mod" => Self::Mod,
            "inc" => Self::Inc,
            "dec" => Self::Dec,
            "and" => Self::And,
            "or" => Self::Or,
            "xor" => Self::Xor,
            "not" => Self::Not,
            "bsl" => Self::Bsl,
            "bsr" => Self::Bsr,
            "rol" => Self::Rol,
            "ror" => Self::Ror,
            "eq" => Self::Eq,
            "neq" => Self::Neq,
            "lt" => Self::Lt,
            "gt" => Self::Gt,
            "leq" => Self::Leq,
            "geq" => Self::Geq,
            "load" => Self::Load,
            "store" => Self::Store,
            "label" => Self::Label,
            "jump" => Self::Jump,
            "jumpif" => Self::Jumpif,
            "call" => Self::Call,
            "ret" => Self::Ret,
            "cast" => Self::Cast,
            "conv" => Self::Conv,
            "i8" => Self::IType(8),
            "i16" => Self::IType(16),
            "i32" => Self::IType(32),
            "i64" => Self::IType(64),
            "u8" => Self::UType(8),
            "u16" => Self::UType(16),
            "u32" => Self::UType(32),
            "u64" => Self::UType(64),
            "f16" => Self::FType(16),
            "f32" => Self::FType(32),
            "f64" => Self::FType(64),
            "ptr" => Self::PtrType,
            "proc" => Self::Proc,
            "in" => Self::In,
            "out" => Self::Out,
            "def" => Self::Def,
            word => {
                // if we didn't match a keyword, this must be a literal or a name
                // literals all start with - or any digit
                // so if the first letter of the word is alphabetical, it is a name
                let first = word.chars().next();
                if first.is_some_and(|c| c.is_alphabetic() || c == '_') {
                    return TokenPayload::Name(word.to_string());
                }
                // if the first letter is not numeric, we don't know what this token is
                else if first.is_some_and(|c| !c.is_numeric() && c != '-' ) {
                    return TokenPayload::Unknown(word.to_string());
                }
                // otherwise, it is a number
                TokenPayload::Literal(word.to_string())
            }
        }
    }
}

pub struct Token {
    payload: TokenPayload,
    pos: FilePos,
}
impl Token {
    pub fn new(pos: FilePos, source: &str) -> Self {
        Token { payload: source.into(), pos }
    }

    pub fn payload(&self) -> &TokenPayload {
        &self.payload
    }

    pub fn is_eof(&self) -> bool {
        self.payload == TokenPayload::Eof
    }
}
impl Positionable for Token {
    fn pos(&self) -> &FilePos {
        &self.pos    
    }
    fn line(&self) -> usize {
        self.pos.line
    }
    fn col(&self) -> usize {
        self.pos.col
    }
}
impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match &self.payload {
            TokenPayload::Name(s) => format!("\"{}\"", s),
            TokenPayload::Literal(s) => format!("literal \"{}\"", s),
            TokenPayload::Unknown(s) => format!("\"{}\"", s),
            _ => format!("{:?}", self.payload)
        };
        write!(f, "{}", str)
    }
}
