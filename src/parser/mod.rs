// the parser takes a vector/stream of tokens and makes sure they make grammatic sense
// makes sure statements are well-formed
// converts number literals into actual number values
// makes sure literals are not too wide for the types of the constants they are associated with

use std::num::{ParseFloatError, ParseIntError};

use crate::tokenizer::{Token, TokenKind};
use crate::logger::Logger;

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
    let mut parser = Parser::new(tokens.iter().peekable(), logger);
    parser.run()
}

struct Parser<'a> {
    tokens: std::iter::Peekable<std::slice::Iter<'a, Token>>,
    logger: &'a mut dyn Logger
}

impl<'a> Parser<'a> {
    fn new(tokens: std::iter::Peekable<std::slice::Iter<'a, Token>>, logger: &'a mut impl Logger) -> Self {
        Parser { tokens, logger }
    }
    
    fn run(&mut self) -> Vec<Statement> {
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
                        if let Some(s) = self.parse_statement() {
                            statements.push(s);
                        }
                    }
                }
            }
        }

        // once we hit EOF, peek again and make sure there's nothing left
        if let Some(t) = self.tokens.peek() {
            self.logger.error("more tokens after EOF (how did you manage that?)", t.line, t.col);
        }

        statements
    }

    fn parse_statement(&mut self) -> Option<Statement> {
        // all statements start with a control word, so check what that is
        if let Some(t) = self.tokens.next() {
            match &t.kind {
                // implicit args are one word, and can be parsed directly
                TokenKind::Pop => Some(Statement::Pop),
                TokenKind::Dup => Some(Statement::Dup),
                TokenKind::Swap => Some(Statement::Swap),
                TokenKind::Add => Some(Statement::Add),
                TokenKind::Sub => Some(Statement::Sub),
                TokenKind::Div => Some(Statement::Div),
                TokenKind::Mult => Some(Statement::Mult),
                TokenKind::Mod => Some(Statement::Mod),
                TokenKind::Inc => Some(Statement::Inc),
                TokenKind::Dec => Some(Statement::Dec),
                TokenKind::And => Some(Statement::And),
                TokenKind::Or => Some(Statement::Or),
                TokenKind::Xor => Some(Statement::Xor),
                TokenKind::Bsl => Some(Statement::Bsl),
                TokenKind::Bsr => Some(Statement::Bsr),
                TokenKind::Rol => Some(Statement::Rol),
                TokenKind::Ror => Some(Statement::Ror),
                TokenKind::Eq => Some(Statement::Eq),
                TokenKind::Neq => Some(Statement::Neq),
                TokenKind::Lt => Some(Statement::Lt),
                TokenKind::Gt => Some(Statement::Gt),
                TokenKind::Leq => Some(Statement::Leq),
                TokenKind::Geq => Some(Statement::Geq),
                TokenKind::Ret => Some(Statement::Ret),

                // explicit args need some more processing
                TokenKind::Const => {
                    if let Some(t) = self.tokens.next() {
                        match &t.kind {
                            TokenKind::Name(s) => {
                                if let Some(value) = self.parse_literal_typed() {
                                    Some(Statement::Const { name: s.to_string(), value })
                                }
                                else {
                                    self.logger.error("constant definition missing value", t.line, t.col);
                                    None
                                }
                            }
                            _ => {
                                self.logger.error("constant definition missing name", t.line, t.col);
                                None
                            }
                        }
                    }
                    else {
                        self.logger.error("expected constant definition", t.line, t.col);
                        None
                    }
                }

                TokenKind::Push => {
                    // next token can be either a name or a literal
                    if let Some(t) = self.tokens.peek() {
                        match &t.kind {
                            // if it is a name, extract the name string and build that token
                            TokenKind::Name(s) => { 
                                self.tokens.next();
                                Some(Statement::PushConst { name: s.to_string() })
                            }
                            _ => {
                                // if it isn't a name but we were able to parse a literal out of
                                // it, build that literal token
                                self.parse_literal_typed().map(|value| Statement::PushLiteral { value })
                            }
                        }
                    }
                    // if there is no next token something is really wrong
                    else {
                        self.logger.error("expected operand for push", t.line, t.col);
                        None
                    }
                }

                TokenKind::Unknown(s) => {
                    self.logger.error(format!("unknown token \"{}\"", &s).as_str(), t.line, t.col);
                    None
                }
                _ => {
                    self.logger.error("expected a statement", t.line, t.col);
                    None
                }
            }
        }
        else {
            None
        }
    }

    fn parse_literal_typed(&mut self) -> Option<Literal> {
        if let Some(t) = self.tokens.next() {
            match &t.kind {
                TokenKind::IType(w) => {
                    self.parse_int(*w)
                }
                TokenKind::FType(w) => {
                    Some(Literal { bits: 0, kind: DType::F16 })
                }
                TokenKind::UType(w) => {
                    Some(Literal { bits: 0, kind: DType::U8 })
                }
                TokenKind::PtrType => {
                    Some(Literal { bits: 0, kind: DType::Pointer })
                }
                _ => {
                    self.logger.error("illegal type specification in literal", t.line, t.col);
                    None
                }
            }
        }
        else {
            None
        }
    }

    fn parse_int(&mut self, width: u8) -> Option<Literal> {
        if let Some(t) = self.tokens.next() {
            match &t.kind {
                TokenKind::Literal(s) => {
                    let value = Parser::str_to_int(s).map_err(|_e| {
                        self.logger.error("malformed integer literal", t.line, t.col)
                    }).ok()?;

                    let dtype = match width {
                        8 => {
                            if !(i8::MIN as i64..i8::MAX as i64).contains(&value) {
                                self.logger.error("literal out of bounds for type", t.line, t.col);
                            }
                            DType::I8
                        }
                        16 => {
                            if !(i16::MIN as i64..i16::MAX as i64).contains(&value) {
                                self.logger.error("literal out of bounds for type", t.line, t.col);
                            }
                            DType::I16
                        }
                        32 => {
                            if !(i32::MIN as i64..i32::MAX as i64).contains(&value) {
                                self.logger.error("literal out of bounds for type", t.line, t.col);
                            }
                            DType::I32
                        }
                        64 => {
                            if !(i64::MIN..i64::MAX).contains(&value) {
                                self.logger.error("literal out of bounds for type", t.line, t.col);
                            }
                            DType::I64
                        }
                        _ => panic!()
                    };

                    Some(Literal { bits: value as u64, kind: dtype })
                }
                _ => {
                    self.logger.error("expected integer literal", t.line, t.col);
                    None
                }
            }
        }
        else {
            None
        }
    }

    fn str_to_int(s: &str) -> Result<i64, ParseIntError> {
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
