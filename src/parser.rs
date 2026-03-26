// the parser takes a vector/stream of tokens and makes sure they make grammatic sense
// makes sure statements are well-formed
// converts number literals into actual number values
// makes sure literals are not too wide for the types of the constants they are associated with
// does not do typechecking
// does not build any symbol tables
use std::fmt::Display;

use half::f16;
use crate::tokenizer::{Token, TokenKind};
use crate::logger::{Logger, LogEvent, EventKind};

#[derive(PartialEq, Debug)]
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

#[derive(PartialEq, Debug)]
pub struct Literal {
    pub bits: u64,
    pub kind: DType
}

// because the language is so flat, we really don't need much of a tree
// we can get away with a single enum
#[derive(PartialEq, Debug)]
pub enum Statement {
    Const { name: String, value: Literal },
    PushConst { name: String },
    PushLiteral { value: Literal },
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

    Add,
    Sub,
    Div,
    Mult,
    Mod,
    Inc,
    Dec,
    And,
    Or,
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

pub fn parse_program(tokens: &[Token], logger: &mut impl Logger) -> Vec<Statement> {
    let mut parser = Parser::new(tokens.iter().peekable());
    parser.run(logger)
}

struct Parser<'a> {
    tokens: std::iter::Peekable<std::slice::Iter<'a, Token>>,
}

impl<'a> Parser<'a> {
    fn new(tokens: std::iter::Peekable<std::slice::Iter<'a, Token>>) -> Self {
        Parser { tokens }
    }
    
    fn run(&mut self, logger: &mut impl Logger) -> Vec<Statement> {
        let mut statements = vec![];

        loop {
            // look at the next token
            if let Some(t) = self.tokens.peek() {
                match t.kind {
                    // if it's the eof, consume it and stop processing
                    TokenKind::Eof => {
                        self.tokens.next();
                        break;
                    }
                    // if it isn't, try to process
                    // processing consumes
                    _ => {
                        match self.parse_statement() {
                            Ok(s) => statements.push(s),
                            Err(e) => logger.log(e),
                        }
                    }
                }
            }
            else {
                self.tokens.next();
                break;
            }
        }

        // once we hit EOF, peek again and make sure there's nothing left
        if let Some(t) = self.tokens.peek() {
            logger.error("more tokens after EOF (how did you manage that?)".to_string(), t.line, t.col);
        }

        statements
    }

    // this can panic in some places
    // a panic should never happen - it means the token stream is malformed somehow
    // in cases where a statement could not be parsed but it was programmer error, this just
    // returns None
    fn parse_statement(&mut self) -> Result<Statement, LogEvent> {
        // we unwrap when getting a next token and expecting one to be there because if the token stream
        // is out of tokens already it's a bug, not a normal error case
        let t = self.tokens.next().unwrap();
        // all statements start with a control word, so check what that is
        match &t.kind {
            // implicit args are one word, and can be parsed directly
            TokenKind::Pop => Ok(Statement::Pop),
            TokenKind::Dup => Ok(Statement::Dup),
            TokenKind::Swap => Ok(Statement::Swap),
            TokenKind::Add => Ok(Statement::Add),
            TokenKind::Sub => Ok(Statement::Sub),
            TokenKind::Div => Ok(Statement::Div),
            TokenKind::Mult => Ok(Statement::Mult),
            TokenKind::Mod => Ok(Statement::Mod),
            TokenKind::Inc => Ok(Statement::Inc),
            TokenKind::Dec => Ok(Statement::Dec),
            TokenKind::And => Ok(Statement::And),
            TokenKind::Or => Ok(Statement::Or),
            TokenKind::Xor => Ok(Statement::Xor),
            TokenKind::Bsl => Ok(Statement::Bsl),
            TokenKind::Bsr => Ok(Statement::Bsr),
            TokenKind::Rol => Ok(Statement::Rol),
            TokenKind::Ror => Ok(Statement::Ror),
            TokenKind::Eq => Ok(Statement::Eq),
            TokenKind::Neq => Ok(Statement::Neq),
            TokenKind::Lt => Ok(Statement::Lt),
            TokenKind::Gt => Ok(Statement::Gt),
            TokenKind::Leq => Ok(Statement::Leq),
            TokenKind::Geq => Ok(Statement::Geq),
            TokenKind::Ret => Ok(Statement::Ret),

            // explicit args need some more processing
            TokenKind::Const => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => {
                        let kind = self.parse_type()?;
                        let value = self.parse_literal(kind)?;
                        Ok(Statement::Const { name: s.to_string(), value })
                    }
                    _ => {
                        Err(LogEvent { kind: EventKind::Error, msg: "expected a constant definition".to_string(), line: t.line, col: t.col })
                    }
                }
            }

            TokenKind::Push => {
                // next token can be either a name or a literal
                let t = self.tokens.peek().unwrap();
                match &t.kind {
                    // if it is a name, consume the token, extract the name string and build the statement
                    TokenKind::Name(s) => { 
                        self.tokens.next();
                        Ok(Statement::PushConst { name: s.to_string() })
                    }
                    _ => {
                        // if it isn't a name but we were able to parse a literal out of
                        // it, build that literal token
                        let kind = self.parse_type()?;
                        let value = self.parse_literal(kind)?;
                        Ok(Statement::PushLiteral { value })
                    }
                }
            }

            TokenKind::Load => {
                let kind = self.parse_type()?;
                Ok(Statement::Load { kind })
            }

            TokenKind::Store => {
                let kind = self.parse_type()?;
                Ok(Statement::Store { kind })
            }

            TokenKind::Label => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => Ok(Statement::Label { name: s.to_string() }),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: "expected name after label".to_string(), line: t.line, col: t.col })
                }
            }

            TokenKind::Jump => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => Ok(Statement::Jump { dest: s.to_string() }),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: "expected name after jump".to_string(), line: t.line, col: t.col })
                }
            }
            
            TokenKind::Jumpif => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => Ok(Statement::Jumpif { dest: s.to_string() }),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: "expected name after jump".to_string(), line: t.line, col: t.col })
                }
            }

            TokenKind::Call => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => Ok(Statement::Call { dest: s.to_string() }),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: "expected name after call".to_string(), line: t.line, col: t.col })
                }
            }

            TokenKind::Cast => {
                let to = self.parse_type()?;
                Ok(Statement::Cast { to })
            }

            TokenKind::Conv => {
                let to = self.parse_type()?;
                Ok(Statement::Conv { to })
            }

            TokenKind::Unknown(s) => Err(LogEvent { kind: EventKind::Error, msg: format!("unknown token \"{}\"", s), line: t.line, col: t.col }),

            _ => {
                Err(LogEvent { kind: EventKind::Error, msg: format!("unexpected token \"{}\"", t), line: t.line, col: t.col})
            }
        }
    }

    fn parse_literal(&mut self, into: DType) -> Result<Literal, LogEvent> {
        let t = self.tokens.next().unwrap();

        match &t.kind {
            TokenKind::Literal(st) => {
                let (repr, radix) =
                    if let Some(s) = st.strip_prefix("0x") {
                        (s, 16)
                    } else if let Some(s) = st.strip_prefix("0b") {
                        (s, 2)
                    } else {
                        (st.as_str(), 10)
                    };

                let err = |e: &dyn Display| LogEvent {
                    kind: EventKind::Error,
                    msg: format!("Unable to parse literal ({})", e),
                    line: t.line,
                    col: t.col,
                };

                let value: Result<u64, LogEvent> = match into {
                    DType::I8  => i8::from_str_radix(repr, radix).map(|v| v as u64).map_err(|e| err(&e)),
                    DType::I16 => i16::from_str_radix(repr, radix).map(|v| v as u64).map_err(|e| err(&e)),
                    DType::I32 => i32::from_str_radix(repr, radix).map(|v| v as u64).map_err(|e| err(&e)),
                    DType::I64 => i64::from_str_radix(repr, radix).map(|v| v as u64).map_err(|e| err(&e)),
                    DType::U8  => u8::from_str_radix(repr, radix).map(|v| v as u64).map_err(|e| err(&e)),
                    DType::U16 => u16::from_str_radix(repr, radix).map(|v| v as u64).map_err(|e| err(&e)),
                    DType::U32 => u32::from_str_radix(repr, radix).map(|v| v as u64).map_err(|e| err(&e)),
                    DType::U64 => u64::from_str_radix(repr, radix).map_err(|e| err(&e)),
                    DType::F16 => repr.parse::<f16>().map(|v| v.to_bits() as u64).map_err(|e| err(&e)),
                    DType::F32 => repr.parse::<f32>().map(|v| v.to_bits() as u64).map_err(|e| err(&e)),
                    DType::F64 => repr.parse::<f64>().map(|v| v.to_bits()).map_err(|e| err(&e)),
                    DType::Pointer => u64::from_str_radix(repr, radix).map(|v| v as u64).map_err(|e| err(&e)),
                };

                Ok(Literal { bits: value?, kind: into })
            }
            _ => Err(LogEvent { kind: EventKind::Error, msg: "expected a literal".to_string(), line: t.line, col: t.col })
        }
    }

    fn parse_type(&mut self) -> Result<DType, LogEvent> {
        let t = self.tokens.next().unwrap();
        match &t.kind {
            TokenKind::IType(w) => {
                match w {
                    8 => Ok(DType::I8),
                    16 => Ok(DType::I16),
                    32 => Ok(DType::I32),
                    64 => Ok(DType::I64),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: format!("integers of width {} are not supported", w), line: t.line, col: t.col })
                }
            }
            TokenKind::FType(w) => {
                match w {
                    16 => Ok(DType::F16),
                    32 => Ok(DType::F32),
                    64 => Ok(DType::F64),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: format!("floats of width {} are not supported", w), line: t.line, col: t.col })
                }
            }
            TokenKind::UType(w) => {
                match w {
                    8 => Ok(DType::U8),
                    16 => Ok(DType::U16),
                    32 => Ok(DType::U32),
                    64 => Ok(DType::U64),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: format!("unsigned integers of width {} are not supported", w), line: t.line, col: t.col })
                }
            }
            TokenKind::PtrType => Ok(DType::Pointer),
            _ => Err(LogEvent { kind: EventKind::Error, msg: format!("unknown type \"{}\"", t), line: t.line, col: t.col})
        }
    }
}
