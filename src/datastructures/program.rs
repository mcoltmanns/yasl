use std::collections::{HashMap, HashSet, hash_map::ValuesMut};
use crate::{datastructures::{procedure::VirtualProcedure, statement::{DType, StatementPayload, VirtualStatement}}, logger::Logger, regmachine::VReg, util::{FilePos, Positionable}};
use crate::datastructures::procedure::VRegProcedure;
use std::fmt::Display;

pub struct VirtualProgram {
    proc_table: HashMap<String, VirtualProcedure>,
    // map procedure names to the list of procedure names they call (call graph) (basically an adjacency list)
    x_calls_y: HashMap<String, Vec<String>>,
    // name of the program
    name: String,
}
impl VirtualProgram {
    /// Constructs an IR program from a series of statements.
    /// This builds the procedure table and fills it with procedures.
    pub fn new(name: &str, statements: &[VirtualStatement], logger: &mut dyn Logger) -> Self {
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
                    // insert it into the call graph (with an empty call list)
                    x_calls_y.insert(name.clone(), vec![]);
                }
                _ => {
                    match &current_proc {
                        // inside a procedure
                        Some(proc) => {
                            // push back the current statement
                            current_statements.push(s.clone());
                            // if it's a call, update the call graph
                            if let StatementPayload::Call { dest } = s.payload() {
                                if x_calls_y.get(proc.name()).is_some_and(|callees_list: &Vec<String> | { !callees_list.contains(&dest) }) {
                                    // if the current procedure has an entry in the call graph and the list of things it calls does not contain the name of this call statement
                                    // add the name this call references to the list of things this procedure calls
                                    x_calls_y.get_mut(proc.name()).map(|vec| vec.push(dest.clone()));
                                }
                                else if x_calls_y.get(proc.name()).is_none() {
                                    // if the current procedure doesn't have an entry in the call graph initiate a new list containing only the destination name
                                    x_calls_y.insert(proc.name().clone(), vec![dest.clone()]);
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

        // check that the entry point is defined
        if !proc_table.contains_key("main") {
            logger.error("no main procedure defined", FilePos::new("", 0, 0));
        }
        // check that the interrupt handler is defined
        if !proc_table.contains_key("trapper") {
            logger.warning("no interrupt handler defined", FilePos { name: name.to_string(), line: 0, col: 0 })
        }
        // if it is, make sure it has no arguments or outputs
        else if proc_table["trapper"].types_in().len() > 0 || proc_table["trapper"].types_out().len() > 0 {
            logger.error("interrupt handler must be zero-effect (cannot have arguments or return values)", proc_table["trapper"].statements()[0].pos().clone())
        }

        VirtualProgram { name: name.parse().unwrap(), proc_table, x_calls_y }
    }

    pub fn sig_table(&self) -> HashMap<String, (Vec<DType>, Vec<DType>)> {
        self.proc_table.iter().map(|(name, proc)| {
            (name.clone(), (proc.types_in().to_vec(), proc.types_out().to_vec()))
        }).collect()
    }

    pub fn proc_table(&self) -> &HashMap<String, VirtualProcedure> {
        &self.proc_table
    }

    pub fn call_graph(&self) -> &HashMap<String, Vec<String>> { &self.x_calls_y }

    pub fn procedures_mut(&mut self) -> ValuesMut<'_, String, VirtualProcedure> {
        self.proc_table.values_mut()
    }

    pub fn get_proc(&self, name: &str) -> Option<&VirtualProcedure> {
        self.proc_table.get(name)
    }

    pub fn get_mut_proc(&mut self, name: &str) -> Option<&mut VirtualProcedure> {
        self.proc_table.get_mut(name)
    }

    pub fn name(&self) -> &String { &self.name }
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
    call_graph: HashMap<String, Vec<String>>,
    name: String,
}
impl VRegProgram {
    // lower an ir program to an infinite register format program
    // register ids are local to program procedures
    // also determines live ranges of registers within procedures
    pub fn lower(name: &str, ir_program: &VirtualProgram, logger: &dyn Logger) -> Self {
        let mut reg_proc_table: HashMap<String, VRegProcedure> = HashMap::new();
        let sig_table = &ir_program.sig_table();
        for (name, proc) in ir_program.proc_table() {
            let reg_proc = VRegProcedure::lower(proc, sig_table, logger);
            reg_proc_table.insert(name.clone(), reg_proc);
        }
        VRegProgram { name: name.to_string(), proc_table: reg_proc_table, call_graph: ir_program.call_graph().clone() }
    }

    pub fn proc_table(&self) -> &HashMap<String, VRegProcedure> {
        &self.proc_table
    }

    pub fn name(&self) -> &String { &self.name }
    
    pub fn call_graph(&self) -> &HashMap<String, Vec<String>> { &self.call_graph }
}
impl Display for VRegProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for proc in self.proc_table.values() {
            writeln!(f, "{}", proc)?;
        }
        Ok(())
    }
}
