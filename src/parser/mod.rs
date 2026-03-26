// the parser takes a vector/stream of tokens and makes sure they make grammatic sense
// makes sure statements are well-formed
// converts number literals into actual number values
// makes sure literals are not too wide for the types of the constants they are associated with
// does not do typechecking

use std::num::{ParseFloatError, ParseIntError};

use crate::tokenizer::{Token, TokenKind};
use crate::logger::{Logger, LogEvent, EventKind};

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

#[derive(Debug)]
pub struct Literal {
    bits: u64,
    kind: DType
}

// because the language is so flat, we really don't need much of a tree
// we can get away with a single enum
#[derive(Debug)]
pub enum Statement {
    Const { name: String, value: Literal },
    PushConst { name: String },
    PushLiteral { value: Literal },
    Pop,
    Dup,
    Swap,
    Load { kind: Literal },
    Store { kind: Literal },
    Label { name: String },
    Jump { dest_label: Box<Statement> },
    Jumpif { dest_label: Box<Statement> },
    Call { dest_label: Box<Statement> },
    Ret,
    Cast { from: Literal, to: Literal },
    Conv { from: Literal, to: Literal },

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
                        let value = self.parse_literal_typed()?;
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
                        let value = self.parse_literal_typed()?;
                        Ok(Statement::PushLiteral { value })
                    }
                }
            }

            TokenKind::Unknown(s) => Err(LogEvent { kind: EventKind::Error, msg: format!("unknown token \"{}\"", s), line: t.line, col: t.col }),

            _ => {
                Err(LogEvent { kind: EventKind::Error, msg: format!("unsupported token \"{:?}\"", t.kind), line: t.line, col: t.col})
            }
        }
    }

    fn parse_literal_typed(&mut self) -> Result<Literal, LogEvent> {
        let t = self.tokens.next().unwrap();
        match &t.kind {
            TokenKind::IType(w) => {
                self.parse_int(w)
            }
            TokenKind::FType(w) => {
                self.parse_float(w)
            }
            //TokenKind::UType(w) => {
            //    Ok(Literal { bits: 0, kind: DType::U8 })
            //}
            //TokenKind::PtrType => {
            //    Ok(Literal { bits: 0, kind: DType::Pointer })
            //}
            _ => Err(LogEvent { kind: EventKind::Error, msg: format!("unknown type \"{}\"", t), line: t.line, col: t.col})
        }
    }

    fn parse_float(&mut self, width: &u8) -> Result<Literal, LogEvent> {
        let t = self.tokens.next().unwrap();
        match &t.kind {
            TokenKind::Literal(s) => {
                let value = Parser::str_to_float(s).map_err(
                    |e| {
                        LogEvent { kind: EventKind::Error, msg: format!("unable to parse float literal \"{}\" ({})", s, e.to_string()), line: t.line, col: t.col }
                    }
                )?;

                let dtype = match width {
                    16 => {
                        DType::F16
                    }
                    32 => {
                        DType::F32
                    }
                    64 => {
                        DType::F64
                    }
                    _ => return Err(LogEvent { kind: EventKind::Error, msg: format!("floats of width {} are not supported", width), line: t.line, col: t.col })
                };

                Ok(Literal { bits: value as u64, kind: dtype })
            }

            _ => Err(LogEvent { kind: EventKind::Error, msg: "expected float literal".to_string(), line: t.line, col: t.col })
        }
    }

    fn parse_int(&mut self, width: &u8) -> Result<Literal, LogEvent> {
        let t = self.tokens.next().unwrap();
        match &t.kind {
            TokenKind::Literal(s) => {
                let value = Parser::str_to_int(s).map_err(
                    |e| {
                        LogEvent { kind: EventKind::Error, msg: format!("unable to parse integer literal \"{}\" ({})", s, e.to_string()), line: t.line, col: t.col }
                    }
                )?;

                let dtype = match width {
                    8 => {
                        if value.abs() > i8::MAX.into() {
                            return Err(LogEvent { kind: EventKind::Error, msg: format!("literal out of bounds for i{}", width), line: t.line, col: t.col });
                        }
                        DType::I8
                    }
                    16 => {
                        if value.abs() > i16::MAX.into() {
                            return Err(LogEvent { kind: EventKind::Error, msg: format!("literal out of bounds for i{}", width), line: t.line, col: t.col });
                        }
                        DType::I16
                    }
                    32 => {
                        if value.abs() > i32::MAX.into() {
                            return Err(LogEvent { kind: EventKind::Error, msg: format!("literal out of bounds for i{}", width), line: t.line, col: t.col });
                        }
                        DType::I32
                    }
                    64 => {
                        // here we can take the datatype correctly because if the string was too
                        // wide it will have been caught in parsing
                        DType::I64
                    }
                    _ => panic!()
                };

                Ok(Literal { bits: value as u64, kind: dtype })
            }

            _ => Err(LogEvent { kind: EventKind::Error, msg: "expected integer literal".to_string(), line: t.line, col: t.col })
        }
    }

    fn str_to_int(s: &str) -> Result<i64, ParseIntError> {
        // have to do own sign logic because rust doesn't do that when parsing ints
        let (sign, s) = if let Some(st) = s.strip_prefix("-") {
            (-1i64, st)
        }
        else {
            (1i64, s)
        };

        if let Some(st) = s.strip_prefix("0x") {
            i64::from_str_radix(st, 16).map(|v| v * sign)
        }
        else if let Some(st) = s.strip_prefix("0b") {
            i64::from_str_radix(st, 2).map(|v| v * sign)
        }
        else {
            s.parse::<i64>().map(|v| v * sign)
        }
    }

    fn str_to_float(s: &str) -> Result<f64, ParseFloatError> {
        s.parse::<f64>()
    }
}
