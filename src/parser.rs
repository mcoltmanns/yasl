// the parser takes a vector/stream of tokens and makes sure they make grammatic sense
// makes sure statements are well-formed
// converts number literals into actual number values
// makes sure literals are not too wide for the types of the constants they are associated with
// does not do typechecking
// does not build any symbol tables
use std::collections::HashMap;
use half::f16;
use crate::tokenizer::Token;
use crate::tokenizer::TokenKind;
use crate::logger::Logger;
use crate::statement::Statement;
use crate::statement::DType;
use crate::statement::StatementKind;
use crate::statement::Literal;
use crate::statement::LiteralValue;

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
            TokenKind::Pop => self.statements.push(Statement::new(StatementKind::Pop, t.pos)),
            TokenKind::Dup => self.statements.push(Statement::new(StatementKind::Dup, t.pos)),
            TokenKind::Swap => self.statements.push(Statement::new(StatementKind::Swap, t.pos)),
            TokenKind::Add => self.statements.push(Statement::new(StatementKind::Add, t.pos)),
            TokenKind::Sub => self.statements.push(Statement::new(StatementKind::Sub, t.pos)),
            TokenKind::Div => self.statements.push(Statement::new(StatementKind::Div, t.pos)),
            TokenKind::Mult => self.statements.push(Statement::new(StatementKind::Mult, t.pos)),
            TokenKind::Mod => self.statements.push(Statement::new(StatementKind::Mod, t.pos)),
            TokenKind::Inc => self.statements.push(Statement::new(StatementKind::Inc, t.pos)),
            TokenKind::Dec => self.statements.push(Statement::new(StatementKind::Dec, t.pos)),
            TokenKind::And => self.statements.push(Statement::new(StatementKind::And, t.pos)),
            TokenKind::Or => self.statements.push(Statement::new(StatementKind::Or, t.pos)),
            TokenKind::Not => self.statements.push(Statement::new(StatementKind::Not, t.pos)),
            TokenKind::Xor => self.statements.push(Statement::new(StatementKind::Xor, t.pos)),
            TokenKind::Bsl => self.statements.push(Statement::new(StatementKind::Bsl, t.pos)),
            TokenKind::Bsr => self.statements.push(Statement::new(StatementKind::Bsr, t.pos)),
            TokenKind::Rol => self.statements.push(Statement::new(StatementKind::Rol, t.pos)),
            TokenKind::Ror => self.statements.push(Statement::new(StatementKind::Ror, t.pos)),
            TokenKind::Eq => self.statements.push(Statement::new(StatementKind::Eq, t.pos)),
            TokenKind::Neq => self.statements.push(Statement::new(StatementKind::Neq, t.pos)),
            TokenKind::Lt => self.statements.push(Statement::new(StatementKind::Lt, t.pos)),
            TokenKind::Gt => self.statements.push(Statement::new(StatementKind::Gt, t.pos)),
            TokenKind::Leq => self.statements.push(Statement::new(StatementKind::Leq, t.pos)),
            TokenKind::Geq => self.statements.push(Statement::new(StatementKind::Geq, t.pos)),
            TokenKind::Ret => self.statements.push(Statement::new(StatementKind::Ret, t.pos)),

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
                                self.statements.push(Statement::new(StatementKind::Push { value: value.clone() }, pos));
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
                        self.statements.push(Statement::new(StatementKind::Push { value: value.unwrap() }, pos));
                    }
                }
            }

            TokenKind::Load => {
                let kind = self.parse_type();
                if kind.is_none () {
                    return;
                }
                self.statements.push(Statement::new(StatementKind::Load { kind: kind.unwrap() }, t.pos));
            }

            TokenKind::Store => {
                let kind = self.parse_type();
                if kind.is_none () {
                    return;
                }
                self.statements.push(Statement::new(StatementKind::Store { kind: kind.unwrap() }, t.pos));
            }

            TokenKind::Label => {
                let t = self.tokens.next().unwrap();
                match t.content {
                    TokenKind::Name(s) => {
                        self.statements.push(Statement::new(StatementKind::Label { name: s.to_string() }, t.pos));
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
                        self.statements.push(Statement::new(StatementKind::Jump { dest: s.to_string() }, t.pos));
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
                        self.statements.push(Statement::new(StatementKind::Jumpif { dest: s.to_string() }, t.pos));
                    }
                    _ => {
                        self.logger.error("expected label after jump".to_string(), t.pos.line, t.pos.col);
                    }
                }
            }

            TokenKind::Proc => {
                let t = self.tokens.next().unwrap();
                match t.content {
                    TokenKind::Name(s) => {
                        self.expect(TokenKind::In);
                        let ins = self.parse_type_list(TokenKind::Out);
                        if ins.is_none() {
                            self.logger.error("expected type list".to_string(), t.pos.line, t.pos.col);
                            return;
                        }
                        self.expect(TokenKind::Out);
                        let outs = self.parse_type_list(TokenKind::Def);
                        if outs.is_none() {
                            self.logger.error("expected type list".to_string(), t.pos.line, t.pos.col);
                            return;
                        }
                        self.expect(TokenKind::Def);
                        self.statements.push(Statement::new(StatementKind::Proc { name: s, t_in: ins.unwrap(), t_out: outs.unwrap() }, t.pos));
                    }
                    _ => {
                        self.logger.error("expected procedure name".to_string(), t.pos.line, t.pos.col);
                    }
                }
            }

            TokenKind::Call => {
                let t = self.tokens.next().unwrap();
                match t.content {
                    TokenKind::Name(s) => {
                        self.statements.push(Statement::new(StatementKind::Call { dest: s.to_string() }, t.pos));
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
                self.statements.push(Statement::new(StatementKind::Cast { to: to.unwrap() }, t.pos));
            }

            TokenKind::Conv => {
                let to = self.parse_type();
                if to.is_none() {
                    return;
                }
                self.statements.push(Statement::new(StatementKind::Conv { to: to.unwrap() }, t.pos));
            }

            TokenKind::Unknown(s) => {
                self.logger.error(format!("unknown token \"{:?}\"", s), t.pos.line, t.pos.col);
            },

            _ => {
                self.logger.error("unimplemented token".to_string(), t.pos.line, t.pos.col);
            }
        };
    }

    fn parse_type_list(&mut self, terminator: TokenKind) -> Option<Vec<DType>> {
        let mut types = Vec::new();

        loop {
            let next = &self.tokens.peek()?.content;
            if next == &terminator {
                return Some(types);
            }
            types.push(self.parse_type()?);
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Option<Token> {
        let next = self.tokens.next()?;
        if next.content == kind {
            Some(next)
        }
        else {
            self.logger.error(format!("expected {:?}, got {:?}", kind, next.content).to_string(), next.pos.line, next.pos.col);
            None
        }
    }

    fn parse_literal(&mut self, into: DType) -> Option<Literal> {
        let t = self.tokens.next()?;

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
                    DType::I8 => i8::from_str_radix(repr, radix).map(LiteralValue::I8).map_err(|e| e.to_string()),
                    DType::I16 => i16::from_str_radix(repr, radix).map(LiteralValue::I16).map_err(|e| e.to_string()),
                    DType::I32 => i32::from_str_radix(repr, radix).map(LiteralValue::I32).map_err(|e| e.to_string()),
                    DType::I64 => i64::from_str_radix(repr, radix).map(LiteralValue::I64).map_err(|e| e.to_string()),
                    DType::U8 => u8::from_str_radix(repr, radix).map(LiteralValue::U8).map_err(|e| e.to_string()),
                    DType::U16 => u16::from_str_radix(repr, radix).map(LiteralValue::U16).map_err(|e| e.to_string()),
                    DType::U32 => u32::from_str_radix(repr, radix).map(LiteralValue::U32).map_err(|e| e.to_string()),
                    DType::U64 => u64::from_str_radix(repr, radix).map(LiteralValue::U64).map_err(|e| e.to_string()),
                    DType::F16 => repr.parse::<f16>().map(LiteralValue::F16).map_err(|e| e.to_string()),
                    DType::F32 => repr.parse::<f32>().map(LiteralValue::F32).map_err(|e| e.to_string()),
                    DType::F64 => repr.parse::<f64>().map(LiteralValue::F64).map_err(|e| e.to_string()),
                    DType::Pointer => u64::from_str_radix(repr, radix).map(LiteralValue::Pointer).map_err(|e| e.to_string()),
                };
                if let Err(msg) = res {
                    self.logger.error(msg, t.pos.line, t.pos.col);
                    None
                }
                else {
                    Some(Literal::new(res.unwrap(), t.pos))
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
