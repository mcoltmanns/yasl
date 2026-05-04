use std::collections::{HashMap, hash_map::{Values, ValuesMut}};
use crate::{datastructures::{procedure::Procedure, statement::{DType, Statement, StatementPayload}}, logger::Logger, util::{FilePos, Positionable}};
use std::fmt::Display;

pub struct Program {
    proc_table: HashMap<String, Procedure>,
}

impl Program {
    /// Constructs an IR program from a series of statements.
    /// This builds the procedure table and fills it with procedures.
    /// It does not build the procedure link table.
    pub fn new(statements: &[Statement], logger: &mut dyn Logger) -> Self {
        let mut proc_table = HashMap::new();

        let mut current_proc: Option<Procedure> = None;
        let mut current_statements: Vec<Statement> = vec![];

        for s in statements {
            match s.payload() {
                StatementPayload::Proc { name, t_in, t_out } => {
                    // if we were working on a procedure, finish it up
                    if let Some(mut prev_proc) = current_proc.take() {
                        // finish the last procedure
                        prev_proc.set_statements(current_statements);
                        current_statements = vec![];
                        // insert it into the procedure table
                        proc_table.insert(prev_proc.name().clone(), prev_proc);
                    }
                    // throw an error if this procedure is already defined
                    if proc_table.contains_key(name) {
                        logger.error(&format!("procedure \"{}\" defined twice", name), s.pos().clone());
                    }
                    // start a new procedure
                    current_proc = Some(Procedure::empty(name.clone(), t_in.clone(), t_out.clone()));
                }
                _ => {
                    match &current_proc {
                        Some(_) => {
                            current_statements.push(s.clone());
                        }
                        None => {
                            logger.warning("unreachable code", s.pos().clone());
                        }
                    }
                }
            }
        }
        // finish the last procedure
        if let Some(mut prev_proc) = current_proc.take() {
            // finish the last procedure
            // turn its statement vector into blocks
            prev_proc.set_statements(current_statements);
            // insert it into the procedure table
            proc_table.insert(prev_proc.name().clone(), prev_proc);
        }

        if !proc_table.contains_key("main") {
            logger.error("no main procedure defined", FilePos::new("", 0, 0));
        }

        Program { proc_table }
    }

    pub fn sig_table(&self) -> HashMap<String, (Vec<DType>, Vec<DType>)> {
        self.proc_table.iter().map(|(name, proc)| {
            (name.clone(), (proc.types_in().to_vec(), proc.types_out().to_vec()))
        }).collect()
    }

    pub fn proc_table(&self) -> &HashMap<String, Procedure> {
        &self.proc_table
    }

    pub fn procedures(&self) -> Values<'_, String, Procedure> {
        self.proc_table.values()
    }

    pub fn procedures_mut(&mut self) -> ValuesMut<'_, String, Procedure> {
        self.proc_table.values_mut()
    }

    pub fn get_proc(&self, name: &str) -> Option<&Procedure> {
        self.proc_table.get(name)
    }

    pub fn get_mut_proc(&mut self, name: &str) -> Option<&mut Procedure> {
        self.proc_table.get_mut(name)
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = "".to_string();
        for p in self.proc_table.values() {
            s.push_str(&format!("{}\n", p));
        }
        write!(f, "{}", s)
    }
}
