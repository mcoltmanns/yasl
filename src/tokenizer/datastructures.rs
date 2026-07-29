use std::cmp::PartialEq;

/// Data held by a token
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Token {
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
    //Trap,
    Nmi,
    Swi,
    Hwi,

    // names are function or constant identifiers
    Name(String),
    // literals are numbers (unparsed)
    Literal(String),

    // these are types
    // value denotes width of the type
    IType(u8),
    UType(u8),
    FType(u8),
    PtrType
}
impl From<&str> for Token {
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
            "tokenizer" => Self::Mod,
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
            "nmi" => Self::Nmi,
            "swi" => Self::Swi,
            "hwi" => Self::Hwi,
            word => {
                // if we didn't match a keyword, this must be a literal or a name
                // literals all start with - or any digit
                // so if the first letter of the word is alphabetical, it is a name
                let first = word.chars().next();
                if first.is_some_and(|c| c.is_alphabetic() || c == '_') {
                    return Token::Name(word.to_string());
                }
                // if the first letter is not numeric, we don't know what this token is
                else if first.is_some_and(|c| !c.is_numeric() && c != '-' ) {
                    return Token::Unknown(word.to_string());
                }
                // otherwise, it is a number
                Token::Literal(word.to_string())
            }
        }
    }
}
impl Token {
    pub fn is_eof(&self) -> bool {
        // double deref gets the data stored in this token
        *self == Token::Eof
    }
}
