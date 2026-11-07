pub mod datastructures;

use std::collections::HashMap;
use datastructures::{StackProgram, StackProcedure};
use crate::primitives::{DType, Literal};
use crate::logger::Logger;
use crate::parser::datastructures::StackStatement;
use crate::util::{FilePos, Positioned};
use crate::tokenizer::datastructures::Token;

pub fn parse(tokens: Vec<Positioned<Token>>, logger: &mut dyn Logger) -> Result<StackProgram, ()> {
    let eof_pos = tokens.last().unwrap().pos().clone();
    let mut parser = Parser { tokens: tokens.into_iter().peekable(), logger, eof_pos };
    parser.parse_program()
}

struct Parser<'t> {
    tokens: std::iter::Peekable<std::vec::IntoIter<Positioned<Token>>>,
    logger: &'t mut dyn Logger,
    eof_pos: FilePos,
}
impl<'t> Parser<'t> {
    fn peek_token(&mut self) -> Result<&Positioned<Token>, ()> {
        self.tokens.peek().ok_or_else(|| self.logger.error("unexpected EOF", self.eof_pos.clone()))
    }
    fn next_token(&mut self) -> Result<Positioned<Token>, ()> {
        self.tokens.next().ok_or_else(|| self.logger.error("unexpected EOF", self.eof_pos.clone()))
    }
    /// Consumes a token!
    fn expect_token(&mut self, expected: Token) -> Result<bool, ()> {
        let have = self.next_token()?;
        let cond = *have == expected;
        if !cond {
            self.logger.error(&format!("expected {:?}, got {:?}", *have, expected), have.pos().clone());
        }
        Ok(cond)
    }

    /// Recursive descent parse an array of tokens into a stack program.
    /// This program is guaranteed to be syntax-correct.
    /// This program is not guaranteed to be type- or flow-correct - use the type- and flowcheckers for that.
    fn parse_program(&mut self) -> Result<StackProgram, ()> {
        let mut program = StackProgram::empty();
        let mut constants = HashMap::new();
        // at the top level, either parse a constant or a procedure
        while let token = self.next_token()? && !token.is_eof() {
            match *token {
                Token::Const => {
                    let (constant, name) = self.parse_constant()?;
                    // constants may not be redefined
                    if constants.insert(name, constant).is_some() {
                        self.logger.error("illegal constant redefinition", token.pos().clone());
                        return Err(())
                    }
                }
                Token::Proc => {
                    let proc = self.parse_procedure(&mut constants)?;
                    let proc_pos = proc.pos().clone();
                    program.add_procedure(proc).map_err(
                        |s| {
                            self.logger.error(&s, proc_pos);
                        }
                    )?;
                }
                _ => {
                    panic!("unexpected token")
                }
            }
        }
        program.get_entry_id().ok_or_else(|| self.logger.error("missing program entry point", self.eof_pos.clone()))?;
        Ok(program)
    }

    fn parse_constant(&mut self) -> Result<(Literal, String), ()> {
        let name_tok = self.next_token()?;
        let type_tok = self.next_token()?;
        let lit_tok = self.next_token()?;
        match &*name_tok {
            Token::Name(name) => {
                let dtype = DType::from_token(&*type_tok).map_err(|e| self.logger.error(&e, type_tok.pos().clone()))?;
                let literal = Literal::from_token(&*lit_tok, &dtype).map_err(|e| self.logger.error(&e, lit_tok.pos().clone()))?;
                Ok((literal, name.clone()))
            }
            _ => {
                self.logger.error("expected constant name", name_tok.pos().clone());
                Err(())
            }
        }
    }

    fn parse_procedure(&mut self, const_table: &mut HashMap<String, Literal>) -> Result<Positioned<StackProcedure>, ()> {
        let mut name = "".to_string();
        let mut types_in = vec![];
        let mut types_out = vec![];
        let name_tok = self.next_token()?;
        let this_pos = name_tok.pos().clone();
        let mut is_extern = false;
        // process declaration (consumes from proc up to and including def)
        match &*name_tok {
            Token::Name(n) => {
                name = n.clone();
                self.expect_token(Token::In)?;
                types_in = self.parse_type_list(Token::Out)?;
                types_out = self.parse_type_list(Token::Def)?;
            }
            _ => {
                self.logger.error("expected procedure name", name_tok.pos().clone());
            }
        }
        // check if that was an extern declaration
        if **self.peek_token()? == Token::Extern {
            self.next_token()?;
            is_extern = true;
        }
        let mut proc = StackProcedure::new(name, is_extern, types_in, types_out).map_err(
            |s| self.logger.error(&s, this_pos.clone()),
        )?;
        // process body
        let mut statements: Vec<Positioned<StackStatement>> = vec![];
        // keep going until we see the next proc or eof (since these are the only two things that can come after procedure termination)
        // TODO we also use tokens being constants as a termination condition here. while techincally this is correct, since constants cannot be defined within procedures, it produces unclear error messages
        // TODO it might be better/easier to parse to have the scope of a procedure be defined explicitly, with curly brackets or a def/fed type syntax
        while let tok = self.peek_token()? && **tok != Token::Proc && !tok.is_eof() && **tok != Token::Const {
            let tok_pos = tok.pos().clone();
            let statement = self.parse_statement(const_table)?;
            statements.push(Positioned::new(statement, tok_pos));
        }
        // empty non-extern procedures not allowed
        if statements.is_empty() && !is_extern {
            self.logger.error("procedure has no body", this_pos.clone());
            return Err(());
        }
        // and extern procedures must be empty
        if !statements.is_empty() && is_extern {
            self.logger.error("procedure declared as extern may not have a body", this_pos.clone());
            return Err(());
        }
        proc.set_statements(statements);
        // the above was "nOn-RuStLiKe" boo hoo :'(
        Ok(Positioned::new(proc, this_pos))
    }

    fn parse_type_list(&mut self, terminator: Token) -> Result<Vec<DType>, ()> {
        let mut types = vec![];
        while let next = self.next_token()? && *next != terminator {
            match DType::from_token(&next) {
                Ok(dtype) => types.push(dtype),
                Err(e) => return Err(self.logger.error(&e, next.pos().clone())),
            }
        }
        Ok(types)
    }

    fn parse_statement(&mut self, const_table: &mut HashMap<String, Literal>) -> Result<StackStatement, ()> {
        let t = self.next_token()?;
        Ok(match &*t {
            Token::Pop => StackStatement::Pop,
            Token::Dup => StackStatement::Dup,
            Token::Swap => StackStatement::Swap,
            Token::Add => StackStatement::Add,
            Token::Sub => StackStatement::Sub,
            Token::Mult => StackStatement::Mult,
            Token::Div => StackStatement::Div,
            Token::Mod => StackStatement::Mod,
            Token::Inc => StackStatement::Inc,
            Token::Dec => StackStatement::Dec,
            Token::And => StackStatement::And,
            Token::Or => StackStatement::Or,
            Token::Not => StackStatement::Not,
            Token::Xor => StackStatement::Xor,
            Token::Bsl => StackStatement::Bsl,
            Token::Bsr => StackStatement::Bsr,
            Token::Rol => StackStatement::Rol,
            Token::Ror => StackStatement::Ror,
            Token::Eq => StackStatement::Eq,
            Token::Neq => StackStatement::Neq,
            Token::Lt => StackStatement::Lt,
            Token::Leq => StackStatement::Leq,
            Token::Gt => StackStatement::Gt,
            Token::Geq => StackStatement::Geq,
            Token::Ret => StackStatement::Ret { t_out: vec![] },
            Token::Nmi => unimplemented!(),
            Token::Swi => unimplemented!(),
            Token::Hwi => unimplemented!(),
            Token::Extern => unimplemented!(),
            Token::Push => {
                let name_t = self.next_token()?;
                match &*name_t {
                    Token::Name(name) => {
                        let value = const_table.get(name).ok_or_else(|| self.logger.error("undefined constant", name_t.pos().clone()))?;
                        StackStatement::Push { value: value.clone() }
                    }
                    _ => {
                        // try to parse a literal if we couldn't find a constant
                        let dtype = DType::from_token(&*name_t).map_err(|e| self.logger.error(&e, name_t.pos().clone()))?;
                        let lit_t = self.next_token()?;
                        let literal = Literal::from_token(&*lit_t, &dtype).map_err(|e| self.logger.error(&e, lit_t.pos().clone()))?;
                        StackStatement::Push { value: literal }
                    }
                }
            }
            Token::Load => {
                let type_t = self.next_token()?;
                let dtype = DType::from_token(&*type_t).map_err(|e| self.logger.error(&e, type_t.pos().clone()))?;
                StackStatement::Load { kind: dtype }
            }
            Token::Store => {
                let type_t = self.next_token()?;
                let dtype = DType::from_token(&*type_t).map_err(|e| self.logger.error(&e, type_t.pos().clone()))?;
                StackStatement::Store { kind: dtype }
            }
            Token::Label => {
                let name_t = self.next_token()?;
                match &*name_t {
                    Token::Name(name) => {
                        StackStatement::Label { name: name.clone() }
                    }
                    _ => {
                        self.logger.error("expected label name", name_t.pos().clone());
                        return Err(());
                    }
                }
            }
            Token::Jump => {
                let dest_name_t = self.next_token()?;
                match &*dest_name_t {
                    Token::Name(name) => {
                        StackStatement::Jump { dest: name.clone() }
                    }
                    _ => {
                        self.logger.error("missing jump destination", dest_name_t.pos().clone());
                        return Err(());
                    }
                }
            }
            Token::Jumpif => {
                let dest_name_t = self.next_token()?;
                match &*dest_name_t {
                    Token::Name(name) => {
                        StackStatement::Jumpif { dest: name.clone() }
                    }
                    _ => {
                        self.logger.error("missing jump destination", dest_name_t.pos().clone());
                        return Err(());
                    }
                }
            }
            Token::Call => {
                let dest_name_t = self.next_token()?;
                match &*dest_name_t {
                    Token::Name(name) => {
                        StackStatement::Call { dest: name.clone(), t_in: vec![], t_out: vec![] } // in/out types are built later by the typechecker
                    }
                    _ => {
                        self.logger.error("missing call destination", dest_name_t.pos().clone());
                        return Err(());
                    }
                }
            }
            Token::Cast => {
                let into_t = self.next_token()?;
                let into_dt = DType::from_token(&*into_t).map_err(|e| self.logger.error(&e, into_t.pos().clone()))?;
                StackStatement::Cast { to: into_dt }
            }
            Token::Conv => {
                let into_t = self.next_token()?;
                let into_dt = DType::from_token(&*into_t).map_err(|e| self.logger.error(&e, into_t.pos().clone()))?;
                StackStatement::Conv { to: into_dt }
            }
            Token::Unknown(s) => {
                self.logger.error("unknown token", t.pos().clone());
                return Err(());
            }
            // constants may not be defined in procedures and thus cannot be statements
            Token::Const => {
                self.logger.error("constants cannot be defined in procedure scope", t.pos().clone());
                return Err(());
            },
            // all of these are grounds for panic here, they should never be parsed as statement intialization tokens no matter what the user does
            Token::Proc => panic!(),
            Token::Eof => panic!(),
            Token::In => panic!(),
            Token::Out => panic!(),
            Token::Def => panic!(),
            Token::Name(_) => panic!(),
            Token::Literal(_) => panic!(),
            Token::IType(_) => panic!(),
            Token::UType(_) => panic!(),
            Token::FType(_) => panic!(),
            Token::PtrType => panic!(),
        })
    }
}
