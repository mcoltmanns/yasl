// the parser takes a vector/stream of tokens and makes sure they make grammatic sense
// makes sure statements are well-formed
// converts number literals into actual number values
// makes sure literals are not too wide for the types of the constants they are associated with
// does not do typechecking
// does not build any symbol tables
use std::fmt::{Display};
use std::collections::HashMap;

use half::f16;
use crate::tokenizer::{Token, TokenKind};
use crate::logger::Logger;
use crate::util::Positioned;

// these are type annotations for memory instructions
#[derive(PartialEq, Debug, Clone, Copy)]
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

// these are typed values for literals
pub type Literal = Positioned<LiteralKind>;
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum LiteralKind {
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
impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "literal {:?}", self.content)
    }
}

// because the language is so flat, we really don't need much of a tree
// we can get away with a single enum
pub type Statement = Positioned<StatementKind>;
#[derive(PartialEq, Debug, Clone)]
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
impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let opt = match &self.content {
            StatementKind::Push { value } => format!("Push {}", value),
            StatementKind::Load { kind } => format!("Load {:?}", kind),
            StatementKind::Store { kind } => format!("Store {:?}", kind),
            StatementKind::Label { name } => format!("Label {:?}", name),
            StatementKind::Jump { dest } => format!("Jump {:?}", dest),
            StatementKind::Jumpif { dest } => format!("Jumpif {:?}", dest),
            StatementKind::Call { dest } => format!("Call {:?}", dest),
            StatementKind::Cast { to } => format!("Cast {:?}", to),
            StatementKind::Conv { to } => format!("Conv {:?}", to),
            _ => format!("{:?}", self.content)
        };
        write!(f, "{}", opt)
    }
}

pub struct Parser<'a>{
    tokens: std::iter::Peekable<std::vec::IntoIter<Token>>,
    logger: &'a mut dyn Logger,
    constants: HashMap<String, Literal>,
    pub statements: Vec<Statement>
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, logger: &'a mut impl Logger) -> Self {
        Parser { tokens: tokens.into_iter().peekable(), logger, constants: HashMap::new(), statements: Vec::new() }
    }

    pub fn parse_tokens(&mut self) {
        loop {
            if let Some(t) = self.tokens.peek() {
                match t.content {
                    TokenKind::Eof => {
                        self.tokens.next();
                        break;
                    }
                    _ => self.parse_statement(),
                };
            }
            else {
                self.tokens.next();
                break;
            }
        }

        if let Some(t) = self.tokens.peek() {
            self.logger.error("more tokens after EOF (how did you manage that?)".to_string(), t.pos.line, t.pos.col);
        }
    }

    // this can panic in some places
    // a panic should never happen - it means the token stream is malformed somehow
    // in cases where a statement could not be parsed but it was programmer error, this just
    // returns None
    fn parse_statement(&mut self) {
        // we unwrap when getting a next token and expecting one to be there because if the token stream
        // is out of tokens already it's a bug, not a normal error case
        let t = self.tokens.next().unwrap();
        // all statements start with a control word, so check what that is
        match t.content {
            // implicit args are one word, and can be parsed directly
            TokenKind::Pop => self.statements.push(Statement { content: StatementKind::Pop, pos: t.pos }),
            TokenKind::Dup => self.statements.push(Statement { content: StatementKind::Dup, pos: t.pos }),
            TokenKind::Swap => self.statements.push(Statement { content: StatementKind::Swap, pos: t.pos }),
            TokenKind::Add => self.statements.push(Statement { content: StatementKind::Add, pos: t.pos }),
            TokenKind::Sub => self.statements.push(Statement { content: StatementKind::Sub, pos: t.pos }),
            TokenKind::Div => self.statements.push(Statement { content: StatementKind::Div, pos: t.pos }),
            TokenKind::Mult => self.statements.push(Statement { content: StatementKind::Mult, pos: t.pos }),
            TokenKind::Mod => self.statements.push(Statement { content: StatementKind::Mod, pos: t.pos }),
            TokenKind::Inc => self.statements.push(Statement { content: StatementKind::Inc, pos: t.pos }),
            TokenKind::Dec => self.statements.push(Statement { content: StatementKind::Dec, pos: t.pos }),
            TokenKind::And => self.statements.push(Statement { content: StatementKind::And, pos: t.pos }),
            TokenKind::Or => self.statements.push(Statement { content: StatementKind::Or, pos: t.pos }),
            TokenKind::Xor => self.statements.push(Statement { content: StatementKind::Xor, pos: t.pos }),
            TokenKind::Bsl => self.statements.push(Statement { content: StatementKind::Bsl, pos: t.pos }),
            TokenKind::Bsr => self.statements.push(Statement { content: StatementKind::Bsr, pos: t.pos }),
            TokenKind::Rol => self.statements.push(Statement { content: StatementKind::Rol, pos: t.pos }),
            TokenKind::Ror => self.statements.push(Statement { content: StatementKind::Ror, pos: t.pos }),
            TokenKind::Eq => self.statements.push(Statement { content: StatementKind::Eq, pos: t.pos }),
            TokenKind::Neq => self.statements.push(Statement { content: StatementKind::Neq, pos: t.pos }),
            TokenKind::Lt => self.statements.push(Statement { content: StatementKind::Lt, pos: t.pos }),
            TokenKind::Gt => self.statements.push(Statement { content: StatementKind::Gt, pos: t.pos }),
            TokenKind::Leq => self.statements.push(Statement { content: StatementKind::Leq, pos: t.pos }),
            TokenKind::Geq => self.statements.push(Statement { content: StatementKind::Geq, pos: t.pos }),
            TokenKind::Ret => self.statements.push(Statement { content: StatementKind::Ret, pos: t.pos }),

            // explicit args need some more processing
            TokenKind::Const => {
                let t = self.tokens.next().unwrap();
                match t.content {
                    TokenKind::Name(s) => {
                        let kind = self.parse_type();
                        if kind.is_none() {
                            return;
                        }
                        let literal = self.parse_literal(kind.unwrap());
                        if literal.is_none() {
                            return;
                        }
                        if self.constants.insert(s, literal.unwrap()).is_some() {
                            self.logger.error("illegal constant redefinition".to_string(), t.pos.line, t.pos.col);
                        }
                    }
                    _ => {
                        self.logger.error("expected a constant definition".to_string(), t.pos.line, t.pos.col);
                    }
                }
            }

            TokenKind::Push => {
                // next token can be either a name or a literal
                let t = self.tokens.peek().unwrap();
                let pos = t.pos.clone();
                match &t.content {
                    // if it is a name, consume the token, find the constant in your table, and generate the push command
                    TokenKind::Name(s) => { 
                        match self.constants.get(s) {
                            Some(value) => {
                                self.statements.push(Statement { content: StatementKind::Push { value: value.clone() }, pos  });
                            }
                            None => {
                                self.logger.error(format!("constant \"{}\" was not defined", s), pos.line, pos.col );
                                return;
                            }
                        };
                        self.tokens.next();
                    }
                    _ => {
                        // if it isn't a name but we were able to parse a literal out of
                        // it, build that literal token
                        let kind = self.parse_type();
                        if kind.is_none() {
                            return;
                        }
                        let value = self.parse_literal(kind.unwrap());
                        if value.is_none() {
                            return;
                        }
                        self.statements.push(Statement {content: StatementKind::Push { value: value.unwrap() }, pos});
                    }
                }
            }

            TokenKind::Load => {
                let kind = self.parse_type();
                if kind.is_none () {
                    return;
                }
                self.statements.push(Statement { content: StatementKind::Load { kind: kind.unwrap() }, pos: t.pos });
            }

            TokenKind::Store => {
                let kind = self.parse_type();
                if kind.is_none () {
                    return;
                }
                self.statements.push(Statement { content: StatementKind::Store { kind: kind.unwrap() }, pos: t.pos });
            }

            TokenKind::Label => {
                let t = self.tokens.next().unwrap();
                match t.content {
                    TokenKind::Name(s) => {
                        self.statements.push(Statement { content: StatementKind::Label { name: s.to_string() }, pos: t.pos });
                    }
                    _ => {
                        self.logger.error("expected name after label".to_string(), t.pos.line, t.pos.col);
                    }
                }
            }

            TokenKind::Jump => {
                let t = self.tokens.next().unwrap();
                match t.content {
                    TokenKind::Name(s) => {
                        self.statements.push(Statement { content: StatementKind::Jump { dest: s.to_string() }, pos: t.pos });
                    }
                    _ => {
                        self.logger.error("expected label after jump".to_string(), t.pos.line, t.pos.col);
                    }
                }
            }
            
            TokenKind::Jumpif => {
                let t = self.tokens.next().unwrap();
                match t.content {
                    TokenKind::Name(s) => {
                        self.statements.push(Statement { content: StatementKind::Jumpif { dest: s.to_string() }, pos: t.pos });
                    }
                    _ => {
                        self.logger.error("expected label after jump".to_string(), t.pos.line, t.pos.col);
                    }
                }
            }

            TokenKind::Call => {
                let t = self.tokens.next().unwrap();
                match t.content {
                    TokenKind::Name(s) => {
                        self.statements.push(Statement { content: StatementKind::Call { dest: s.to_string() }, pos: t.pos });
                    }
                    _ => {
                        self.logger.error("expected label after call".to_string(), t.pos.line, t.pos.col);
                    }
                }
            }

            TokenKind::Cast => {
                let to = self.parse_type();
                if to.is_none() {
                    return;
                }
                self.statements.push(Statement { content: StatementKind::Cast { to: to.unwrap() }, pos: t.pos });
            }

            TokenKind::Conv => {
                let to = self.parse_type();
                if to.is_none() {
                    return;
                }
                self.statements.push(Statement { content: StatementKind::Conv { to: to.unwrap() }, pos: t.pos });
            }

            TokenKind::Unknown(s) => {
                self.logger.error(format!("unknown token \"{:?}\"", s), t.pos.line, t.pos.col);
            },

            _ => {
                //panic!("missing token implementation")
            }
        };
    }

    fn parse_literal(&mut self, into: DType) -> Option<Literal> {
        let t = self.tokens.next().unwrap();

        match t.content {
            TokenKind::Literal(st) => {
                let (repr, radix) =
                    if let Some(s) = st.strip_prefix("0x") {
                        (s, 16)
                    } else if let Some(s) = st.strip_prefix("0b") {
                        (s, 2)
                    } else {
                        (st.as_str(), 10)
                    };

                let res = match into {
                    DType::I8 => i8::from_str_radix(repr, radix).map(LiteralKind::I8).map_err(|e| e.to_string()),
                    DType::I16 => i16::from_str_radix(repr, radix).map(LiteralKind::I16).map_err(|e| e.to_string()),
                    DType::I32 => i32::from_str_radix(repr, radix).map(LiteralKind::I32).map_err(|e| e.to_string()),
                    DType::I64 => i64::from_str_radix(repr, radix).map(LiteralKind::I64).map_err(|e| e.to_string()),
                    DType::U8 => u8::from_str_radix(repr, radix).map(LiteralKind::U8).map_err(|e| e.to_string()),
                    DType::U16 => u16::from_str_radix(repr, radix).map(LiteralKind::U16).map_err(|e| e.to_string()),
                    DType::U32 => u32::from_str_radix(repr, radix).map(LiteralKind::U32).map_err(|e| e.to_string()),
                    DType::U64 => u64::from_str_radix(repr, radix).map(LiteralKind::U64).map_err(|e| e.to_string()),
                    DType::F16 => repr.parse::<f16>().map(LiteralKind::F16).map_err(|e| e.to_string()),
                    DType::F32 => repr.parse::<f32>().map(LiteralKind::F32).map_err(|e| e.to_string()),
                    DType::F64 => repr.parse::<f64>().map(LiteralKind::F64).map_err(|e| e.to_string()),
                    DType::Pointer => u64::from_str_radix(repr, radix).map(LiteralKind::Pointer).map_err(|e| e.to_string()),
                };
                if let Err(msg) = res {
                    self.logger.error(msg, t.pos.line, t.pos.col);
                    None
                }
                else {
                    Some(Literal { content: res.unwrap(), pos: t.pos })
                }
            }
            _ => {
                self.logger.error("expected a literal".to_string(), t.pos.line, t.pos.col);
                None
            }
        }
    }

    fn parse_type(&mut self) -> Option<DType> {
        let t = self.tokens.next().unwrap();
        match t.content {
            TokenKind::IType(w) => {
                match w {
                    8 => Some(DType::I8),
                    16 => Some(DType::I16),
                    32 => Some(DType::I32),
                    64 => Some(DType::I64),
                    _ => {
                        self.logger.error(format!("integers of width {} are not supported", w), t.pos.line, t.pos.col);
                        None
                    }
                }
            }
            TokenKind::FType(w) => {
                match w {
                    16 => Some(DType::F16),
                    32 => Some(DType::F32),
                    64 => Some(DType::F64),
                    _ => {
                        self.logger.error(format!("floats of width {} are not supported", w), t.pos.line, t.pos.col);
                        None
                    }
                }
            }
            TokenKind::UType(w) => {
                match w {
                    8 => Some(DType::U8),
                    16 => Some(DType::U16),
                    32 => Some(DType::U32),
                    64 => Some(DType::U64),
                    _ => {
                        self.logger.error(format!("unsigned ints of width {} are not supported", w), t.pos.line, t.pos.col);
                        None
                    }
                }
            }
            TokenKind::PtrType => Some(DType::Pointer),
            _ => {
                self.logger.error(format!("unknown type {}", t), t.pos.line, t.pos.col);
                None
            }
        }
    }
}
