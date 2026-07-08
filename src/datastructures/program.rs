use std::collections::{HashMap, HashSet, hash_map::ValuesMut};
use crate::{datastructures::{procedure::VirtualProcedure, statement::{DType, StatementPayload, VirtualStatement}}, logger::Logger, regmachine::VReg, util::{FilePos, Positionable}};
use crate::datastructures::procedure::VRegProcedure;
use std::fmt::Display;

pub struct VirtualProgram {
    proc_table: HashMap<String, VirtualProcedure>,
    // map procedure names to the list of procedure names they call
    x_calls_y: HashMap<String, Vec<String>>
}
impl VirtualProgram {
    /// Constructs an IR program from a series of statements.
    /// This builds the procedure table and fills it with procedures.
    /// TODO it also builds the procedure link table/call graph
    pub fn new(statements: &[VirtualStatement], logger: &mut dyn Logger) -> Self {
        let mut proc_table = HashMap::new();
        let mut x_calls_y = HashMap::new();

        let mut current_proc: Option<VirtualProcedure> = None;
        let mut current_statements: Vec<VirtualStatement> = vec![];

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
                    current_proc = Some(VirtualProcedure::empty(name.clone(), t_in.clone(), t_out.clone(), s.pos().clone()));
                }
                _ => {
                    match &current_proc {
                        Some(proc) => {
                            current_statements.push(s.clone());
                            if let StatementPayload::Call { dest } = s.payload() {
                                if x_calls_y.get(proc.name()).is_some_and(|callees_list: &Vec<String> | { callees_list.contains(&dest) }) {
                                    // TODO build call graph??!
                                }
                            }
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

        VirtualProgram { proc_table, x_calls_y }
    }

    pub fn sig_table(&self) -> HashMap<String, (Vec<DType>, Vec<DType>)> {
        self.proc_table.iter().map(|(name, proc)| {
            (name.clone(), (proc.types_in().to_vec(), proc.types_out().to_vec()))
        }).collect()
    }

    pub fn proc_table(&self) -> &HashMap<String, VirtualProcedure> {
        &self.proc_table
    }

    pub fn procedures_mut(&mut self) -> ValuesMut<'_, String, VirtualProcedure> {
        self.proc_table.values_mut()
    }

    pub fn get_proc(&self, name: &str) -> Option<&VirtualProcedure> {
        self.proc_table.get(name)
    }

    pub fn get_mut_proc(&mut self, name: &str) -> Option<&mut VirtualProcedure> {
        self.proc_table.get_mut(name)
    }
}
impl Display for VirtualProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = "".to_string();
        for p in self.proc_table.values() {
            s.push_str(&format!("{}\n", p));
        }
        write!(f, "{}", s)
    }
}

pub struct VRegProgram {
    proc_table: HashMap<String, VRegProcedure>,
}
impl VRegProgram {
    // lower an ir program to an infinite register format program
    // register ids are local to program procedures
    // also determines live ranges of registers within procedures
    pub fn lower(ir_program: &VirtualProgram) -> Self {
        let mut reg_proc_table: HashMap<String, VRegProcedure> = HashMap::new();
        let sig_table = &ir_program.sig_table();
        for (name, proc) in ir_program.proc_table() {
            let reg_proc = VRegProcedure::lower(proc, sig_table);
            reg_proc_table.insert(name.clone(), reg_proc);
        }
        VRegProgram { proc_table: reg_proc_table }
    }

    pub fn proc_table(&self) -> &HashMap<String, VRegProcedure> {
        &self.proc_table
    }
}
impl Display for VRegProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for proc in self.proc_table.values() {
            writeln!(f, "{}", proc)?;
        }
        Ok(())
    }
}
