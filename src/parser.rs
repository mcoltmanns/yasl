// the parser takes a vector/stream of tokens and makes sure they make grammatic sense
// makes sure statements are well-formed
// converts number literals into actual number values
// makes sure literals are not too wide for the types of the constants they are associated with
// does not do typechecking
// does not build any symbol tables
use std::fmt::Display;
use std::collections::HashMap;

use half::f16;
use crate::tokenizer::{Token, TokenKind};
use crate::logger::{Logger, LogEvent, EventKind};

// these are type annotations for memory instructions
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

// these are typed values for literals
#[derive(PartialEq, Debug, Clone, Copy)]
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

// because the language is so flat, we really don't need much of a tree
// we can get away with a single enum
#[derive(PartialEq, Debug)]
pub enum Statement {
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

pub fn parse_program(tokens: &[Token], logger: &mut impl Logger) -> Vec<Statement> {
    let mut parser = Parser::new(tokens.iter().peekable());
    parser.run(logger)
}

struct Parser<'a> {
    tokens: std::iter::Peekable<std::slice::Iter<'a, Token>>,
    constants: HashMap<String, Literal>,
}

impl<'a> Parser<'a> {
    fn new(tokens: std::iter::Peekable<std::slice::Iter<'a, Token>>) -> Self {
        Parser { tokens, constants: HashMap::new() }
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
                            Ok(Some(s)) => statements.push(s),
                            Ok(None) => {},
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
    fn parse_statement(&mut self) -> Result<Option<Statement>, LogEvent> {
        // we unwrap when getting a next token and expecting one to be there because if the token stream
        // is out of tokens already it's a bug, not a normal error case
        let t = self.tokens.next().unwrap();
        // all statements start with a control word, so check what that is
        match &t.kind {
            // implicit args are one word, and can be parsed directly
            TokenKind::Pop => Ok(Some(Statement::Pop)),
            TokenKind::Dup => Ok(Some(Statement::Dup)),
            TokenKind::Swap => Ok(Some(Statement::Swap)),
            TokenKind::Add => Ok(Some(Statement::Add)),
            TokenKind::Sub => Ok(Some(Statement::Sub)),
            TokenKind::Div => Ok(Some(Statement::Div)),
            TokenKind::Mult => Ok(Some(Statement::Mult)),
            TokenKind::Mod => Ok(Some(Statement::Mod)),
            TokenKind::Inc => Ok(Some(Statement::Inc)),
            TokenKind::Dec => Ok(Some(Statement::Dec)),
            TokenKind::And => Ok(Some(Statement::And)),
            TokenKind::Or => Ok(Some(Statement::Or)),
            TokenKind::Xor => Ok(Some(Statement::Xor)),
            TokenKind::Bsl => Ok(Some(Statement::Bsl)),
            TokenKind::Bsr => Ok(Some(Statement::Bsr)),
            TokenKind::Rol => Ok(Some(Statement::Rol)),
            TokenKind::Ror => Ok(Some(Statement::Ror)),
            TokenKind::Eq => Ok(Some(Statement::Eq)),
            TokenKind::Neq => Ok(Some(Statement::Neq)),
            TokenKind::Lt => Ok(Some(Statement::Lt)),
            TokenKind::Gt => Ok(Some(Statement::Gt)),
            TokenKind::Leq => Ok(Some(Statement::Leq)),
            TokenKind::Geq => Ok(Some(Statement::Geq)),
            TokenKind::Ret => Ok(Some(Statement::Ret)),

            // explicit args need some more processing
            TokenKind::Const => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => {
                        let kind = self.parse_type()?;
                        let value = self.parse_literal(kind)?;
                        match self.constants.insert(s.to_string(), value) {
                            None => Ok(None),
                            Some(_) => Err(LogEvent { kind: EventKind::Error, msg: "illegal constant redefinition".to_string(), line: t.line, col: t.col })
                        }
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
                    // if it is a name, consume the token, find the constant in your table, and generate the push command
                    TokenKind::Name(s) => { 
                        let line = t.line;
                        let col = t.col;
                        self.tokens.next();
                        match self.constants.get(s) {
                            Some(value) => Ok(Some(Statement::Push { value: *value })),
                            None => Err(LogEvent { kind: EventKind::Error, msg: format!("constant \"{}\" was not defined", s), line, col })
                        }
                    }
                    _ => {
                        // if it isn't a name but we were able to parse a literal out of
                        // it, build that literal token
                        let kind = self.parse_type()?;
                        let value = self.parse_literal(kind)?;
                        Ok(Some(Statement::Push { value } ))
                    }
                }
            }

            TokenKind::Load => {
                let kind = self.parse_type()?;
                Ok(Some(Statement::Load { kind }))
            }

            TokenKind::Store => {
                let kind = self.parse_type()?;
                Ok(Some(Statement::Store { kind }))
            }

            TokenKind::Label => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => Ok(Some(Statement::Label { name: s.to_string() })),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: "expected name after label".to_string(), line: t.line, col: t.col })
                }
            }

            TokenKind::Jump => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => Ok(Some(Statement::Jump { dest: s.to_string() })),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: "expected name after jump".to_string(), line: t.line, col: t.col })
                }
            }
            
            TokenKind::Jumpif => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => Ok(Some(Statement::Jumpif { dest: s.to_string() })),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: "expected name after jump".to_string(), line: t.line, col: t.col })
                }
            }

            TokenKind::Call => {
                let t = self.tokens.next().unwrap();
                match &t.kind {
                    TokenKind::Name(s) => Ok(Some(Statement::Call { dest: s.to_string() })),
                    _ => Err(LogEvent { kind: EventKind::Error, msg: "expected name after call".to_string(), line: t.line, col: t.col })
                }
            }

            TokenKind::Cast => {
                let to = self.parse_type()?;
                Ok(Some(Statement::Cast { to }))
            }

            TokenKind::Conv => {
                let to = self.parse_type()?;
                Ok(Some(Statement::Conv { to }))
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

                match into {
                    DType::I8 => i8::from_str_radix(repr, radix).map(Literal::I8).map_err(|e| err(&e)),
                    DType::I16 => i16::from_str_radix(repr, radix).map(Literal::I16).map_err(|e| err(&e)),
                    DType::I32 => i32::from_str_radix(repr, radix).map(Literal::I32).map_err(|e| err(&e)),
                    DType::I64 => i64::from_str_radix(repr, radix).map(Literal::I64).map_err(|e| err(&e)),
                    DType::U8 => u8::from_str_radix(repr, radix).map(Literal::U8).map_err(|e| err(&e)),
                    DType::U16 => u16::from_str_radix(repr, radix).map(Literal::U16).map_err(|e| err(&e)),
                    DType::U32 => u32::from_str_radix(repr, radix).map(Literal::U32).map_err(|e| err(&e)),
                    DType::U64 => u64::from_str_radix(repr, radix).map(Literal::U64).map_err(|e| err(&e)),
                    DType::F16 => repr.parse::<f16>().map(Literal::F16).map_err(|e| err(&e)),
                    DType::F32 => repr.parse::<f32>().map(Literal::F32).map_err(|e| err(&e)),
                    DType::F64 => repr.parse::<f64>().map(Literal::F64).map_err(|e| err(&e)),
                    DType::Pointer => u64::from_str_radix(repr, radix).map(Literal::Pointer).map_err(|e| err(&e)),
                }
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
