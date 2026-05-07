// the parser takes a vector/stream of tokens and makes sure they make grammatic sense
// makes sure statements are well-formed
// converts number literals into actual number values
// makes sure literals are not too wide for the types of the constants they are associated with
// does not do typechecking
// does not build any symbol tables
use std::collections::HashMap;
use crate::datastructures::statement::{DType, Literal, VirtualStatement, StatementPayload};
use crate::datastructures::token::{Token, TokenPayload};
use crate::logger::Logger;
use crate::util::Positionable;

pub struct Parser{
    tokens: std::iter::Peekable<std::vec::IntoIter<Token>>,
    constants: HashMap<String, Literal>,
    statements: Vec<VirtualStatement>
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens: tokens.into_iter().peekable(), constants: HashMap::new(), statements: Vec::new() }
    }

    pub fn statements(&self) -> &Vec<VirtualStatement> {
        &self.statements
    }

    pub fn parse_tokens(&mut self, logger: &mut dyn Logger) {
        loop {
            if let Some(t) = self.tokens.peek() {
                if t.is_eof() {
                    self.tokens.next();
                    break;
                }
                else {
                    self.parse_statement(logger);
                }
            }
            else {
                self.tokens.next();
                break;
            }
        }

        if let Some(t) = self.tokens.peek() {
            logger.error("more tokens after EOF (how did you manage that?)", t.pos().clone());
        }
    }

    // this can panic in some places
    // a panic should never happen - it means the token stream is malformed somehow
    // in cases where a statement could not be parsed but it was programmer error, this just
    // returns None
    fn parse_statement(&mut self, logger: &mut dyn Logger) {
        // we unwrap when getting a next token and expecting one to be there because if the token stream
        // is out of tokens already it's a bug, not a normal error case
        let t = self.tokens.next().unwrap();
        // all statements start with a control word, so check what that is
        match t.payload() {
            // implicit args are one word, and can be parsed directly
            TokenPayload::Pop => self.statements.push(VirtualStatement::new(StatementPayload::Pop, t.pos().clone())),
            TokenPayload::Dup => self.statements.push(VirtualStatement::new(StatementPayload::Dup, t.pos().clone())),
            TokenPayload::Swap => self.statements.push(VirtualStatement::new(StatementPayload::Swap, t.pos().clone())),
            TokenPayload::Add => self.statements.push(VirtualStatement::new(StatementPayload::Add, t.pos().clone())),
            TokenPayload::Sub => self.statements.push(VirtualStatement::new(StatementPayload::Sub, t.pos().clone())),
            TokenPayload::Div => self.statements.push(VirtualStatement::new(StatementPayload::Div, t.pos().clone())),
            TokenPayload::Mult => self.statements.push(VirtualStatement::new(StatementPayload::Mult, t.pos().clone())),
            TokenPayload::Mod => self.statements.push(VirtualStatement::new(StatementPayload::Mod, t.pos().clone())),
            TokenPayload::Inc => self.statements.push(VirtualStatement::new(StatementPayload::Inc, t.pos().clone())),
            TokenPayload::Dec => self.statements.push(VirtualStatement::new(StatementPayload::Dec, t.pos().clone())),
            TokenPayload::And => self.statements.push(VirtualStatement::new(StatementPayload::And, t.pos().clone())),
            TokenPayload::Or => self.statements.push(VirtualStatement::new(StatementPayload::Or, t.pos().clone())),
            TokenPayload::Not => self.statements.push(VirtualStatement::new(StatementPayload::Not, t.pos().clone())),
            TokenPayload::Xor => self.statements.push(VirtualStatement::new(StatementPayload::Xor, t.pos().clone())),
            TokenPayload::Bsl => self.statements.push(VirtualStatement::new(StatementPayload::Bsl, t.pos().clone())),
            TokenPayload::Bsr => self.statements.push(VirtualStatement::new(StatementPayload::Bsr, t.pos().clone())),
            TokenPayload::Rol => self.statements.push(VirtualStatement::new(StatementPayload::Rol, t.pos().clone())),
            TokenPayload::Ror => self.statements.push(VirtualStatement::new(StatementPayload::Ror, t.pos().clone())),
            TokenPayload::Eq => self.statements.push(VirtualStatement::new(StatementPayload::Eq, t.pos().clone())),
            TokenPayload::Neq => self.statements.push(VirtualStatement::new(StatementPayload::Neq, t.pos().clone())),
            TokenPayload::Lt => self.statements.push(VirtualStatement::new(StatementPayload::Lt, t.pos().clone())),
            TokenPayload::Gt => self.statements.push(VirtualStatement::new(StatementPayload::Gt, t.pos().clone())),
            TokenPayload::Leq => self.statements.push(VirtualStatement::new(StatementPayload::Leq, t.pos().clone())),
            TokenPayload::Geq => self.statements.push(VirtualStatement::new(StatementPayload::Geq, t.pos().clone())),
            TokenPayload::Ret => self.statements.push(VirtualStatement::new(StatementPayload::Ret, t.pos().clone())),

            TokenPayload::Trap => { 
                unimplemented!()
            }

            TokenPayload::Const => {
                let name_t = self.tokens.next().unwrap();
                match name_t.payload() {
                    TokenPayload::Name(s) => {
                        let type_t = self.tokens.next().unwrap();
                        match DType::from_token(&type_t) {
                            Ok(dtype) => {
                                let literal_t = self.tokens.next().unwrap();
                                match Literal::from_token(&literal_t, &dtype) {
                                    Ok(literal) => {
                                        if self.constants.insert(s.clone(), literal).is_some() {
                                            logger.error("illegal constant redefinition", t.pos().clone());
                                        }
                                    }
                                    Err(msg) => {
                                        logger.error(&msg, literal_t.pos().clone());
                                    }
                                }
                            }
                            Err(msg) => {
                                logger.error(&msg, type_t.pos().clone());
                            }
                        }
                    }
                    _ => {
                        logger.error("expected a constant definition", name_t.pos().clone());
                    }
                }
            }

            TokenPayload::Push => {
                // next token can be either a name or a literal
                let name_t = self.tokens.next().unwrap();
                match name_t.payload() {
                    // if it's a name, find the constant in your table and emit a push for that
                    // value
                    TokenPayload::Name(s) => { 
                        match self.constants.get(s) {
                            Some(value) => {
                                self.statements.push(VirtualStatement::new(StatementPayload::Push { value: value.clone() }, t.pos().clone()));
                            }
                            None => {
                                logger.error(&format!("constant \"{}\" was not defined", s), t.pos().clone() );
                                return;
                            }
                        };
                        self.tokens.next();
                    }
                    _ => {
                        // if it isn't a name, try to parse a datatype
                        match DType::from_token(&name_t) {
                            Err(s) => {
                                logger.error(&s, name_t.pos().clone());
                            }
                            Ok(to) => {
                                // and if we could parse a datatype, try to parse the next thing as
                                // a literal into that datatype, and emit a push for that
                                let val_t = self.tokens.next().unwrap();
                                match Literal::from_token(&val_t, &to) {
                                    Err(s) => {
                                        logger.error(&s, val_t.pos().clone());
                                    }
                                    Ok(literal) => {
                                        self.statements.push(VirtualStatement::new(StatementPayload::Push { value: literal }, t.pos().clone()));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            TokenPayload::Load => {
                let type_t = self.tokens.next().unwrap();
                match DType::from_token(&type_t) {
                    Err(s) => {
                        logger.error(&s, type_t.pos().clone());
                    }
                    Ok(to) => {
                        self.statements.push(VirtualStatement::new(StatementPayload::Load { kind: to }, t.pos().clone()));
                    }
                }
            }

            TokenPayload::Store => {
                let type_t = self.tokens.next().unwrap();
                match DType::from_token(&type_t) {
                    Err(s) => {
                        logger.error(&s, type_t.pos().clone());
                    }
                    Ok(to) => {
                        self.statements.push(VirtualStatement::new(StatementPayload::Store { kind: to }, t.pos().clone()));
                    }
                }
            }

            TokenPayload::Label => {
                let name_t = self.tokens.next().unwrap();
                match name_t.payload() {
                    TokenPayload::Name(s) => {
                        self.statements.push(VirtualStatement::new(StatementPayload::Label { name: s.clone() }, t.pos().clone()));
                    }
                    _ => {
                        logger.error("expected name after label", name_t.pos().clone());
                    }
                }
            }

            TokenPayload::Jump => {
                let dest_t = self.tokens.next().unwrap();
                match dest_t.payload() {
                    TokenPayload::Name(s) => {
                        self.statements.push(VirtualStatement::new(StatementPayload::Jump { dest: s.clone() }, t.pos().clone()));
                    }
                    _ => {
                        logger.error("expected label after jump", dest_t.pos().clone());
                    }
                }
            }
            
            TokenPayload::Jumpif => {
                let dest_t = self.tokens.next().unwrap();
                match dest_t.payload() {
                    TokenPayload::Name(s) => {
                        self.statements.push(VirtualStatement::new(StatementPayload::Jumpif { dest: s.clone() }, t.pos().clone()));
                    }
                    _ => {
                        logger.error("expected label after jump", dest_t.pos().clone());
                    }
                }
            }

            TokenPayload::Proc => {
                let name_t = self.tokens.next().unwrap();
                match name_t.payload() {
                    TokenPayload::Name(s) => {
                        self.expect(TokenPayload::In, logger);
                        let ins = self.parse_type_list(TokenPayload::Out);
                        self.expect(TokenPayload::Out, logger);
                        let outs = self.parse_type_list(TokenPayload::Def);
                        self.expect(TokenPayload::Def, logger);
                        match (ins, outs) {
                            (Ok(ins_ok), Ok(outs_ok)) => {
                                self.statements.push(VirtualStatement::new(StatementPayload::Proc { name: s.clone(), t_in: ins_ok, t_out: outs_ok }, t.pos().clone()));
                            }
                            (Err(err), _) | (_, Err(err))=> {
                                logger.error(&err, t.pos().clone());
                            }
                        }
                    }
                    _ => {
                        logger.error("expected procedure name", name_t.pos().clone());
                    }
                }
            }

            TokenPayload::Call => {
                let t = self.tokens.next().unwrap();
                match t.payload() {
                    TokenPayload::Name(s) => {
                        self.statements.push(VirtualStatement::new(StatementPayload::Call { dest: s.to_string() }, t.pos().clone()));
                    }
                    _ => {
                        logger.error("expected label after call", t.pos().clone());
                    }
                }
            }

            TokenPayload::Cast => {
                let t = self.tokens.next().unwrap();
                match DType::from_token(&t) {
                    Err(s) => {
                        logger.error(&s, t.pos().clone());
                    }
                    Ok(to) => {
                        self.statements.push(VirtualStatement::new(StatementPayload::Cast { to }, t.pos().clone()));
                    }
                }
            }

            TokenPayload::Conv => {
                let t = self.tokens.next().unwrap();
                match DType::from_token(&t) {
                    Err(s) => {
                        logger.error(&s, t.pos().clone());
                    }
                    Ok(to) => {
                        self.statements.push(VirtualStatement::new(StatementPayload::Conv { to }, t.pos().clone()));
                    }
                }
            }

            TokenPayload::Unknown(s) => {
                logger.error(&format!("unknown token \"{:?}\"", s), t.pos().clone());
            },

            _ => {
                logger.error("unimplemented token", t.pos().clone());
            }
        };
    }

    fn parse_type_list(&mut self, terminator: TokenPayload) -> Result<Vec<DType>, String> {
        let mut types = Vec::new();

        loop {
            if let Some(next) = self.tokens.peek() {
                if *next.payload() == terminator {
                    return Ok(types);
                }
                match DType::from_token(next) {
                    Ok(t) => types.push(t),
                    Err(msg) => return Err(msg)
                }
            }
            else {
                return Err("expected a type list".to_string())
            }
            self.tokens.next();
        }
    }

    fn expect(&mut self, kind: TokenPayload, logger: &mut dyn Logger) -> Option<Token> {
        let next = self.tokens.next()?;
        if *next.payload() == kind {
            Some(next)
        }
        else {
            logger.error(&format!("expected {:?}, got {:?}", kind, next.payload()), next.pos().clone());
            None
        }
    }
}
