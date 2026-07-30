use std::collections::{HashMap, HashSet, hash_map::ValuesMut};
use crate::{datastructures::{procedure::VirtualProcedure, statement::{DType, StatementPayload, VirtualStatement}}, logger::Logger, regmachine::VReg, util::{FilePos}};
use crate::datastructures::procedure::VRegProcedure;
use std::fmt::Display;
use crate::util::{Named};

/// A program is a generic collection of procedures, with a couple other details
///
/// Programs are generic because they can target different conceptual machines.
/// Consider Program<StackProcedure> vs. Program<VRegProcedure>.
type StackProgram = Program<StackProcedure>;
type RegisterProgram = Program<VRegProcedure>;
struct Program<Proc> {
    /// This program's entry procedure
    entry: Option<Proc>,
    /// This program's non-maskable interrupt handler (optional)
    nmi: Option<Proc>,
    /// This program's maskable hardware interrupt handler (optional)
    hwi: Option<Proc>,
    /// This program's maskable software interrupt handler table
    swi_table: [Option<Proc>; 256],
    /// Procedure table mapping names to non-special procedures
    proc_table: HashMap<String, Proc>,
}
impl<Proc: Named> Program<Proc> {
    /// Construct an empty program.
    pub fn empty() -> Program<Proc> {
        Program { entry: None, nmi: None, hwi: None, swi_table: [None; 256], proc_table: HashMap::new() }
    }

    // getters/setters
    pub fn set_entry(&mut self, entry: Proc) {
        self.entry = Some(entry);
    }
    pub fn get_entry(&self) -> Option<&Proc> {
        self.entry.as_ref()
    }
    pub fn get_entry_mut(&mut self) -> Option<&mut Proc> {
        self.entry.as_mut()
    }
    pub fn has_entry(&self) -> bool {
        self.entry.is_some()
    }

    pub fn set_nmi(&mut self, nmi: Proc) {
        self.nmi = Some(nmi)
    }
    pub fn get_nmi(&self) -> Option<&Proc> {
        self.nmi.as_ref()
    }
    pub fn get_nmi_mut(&mut self) -> Option<&mut Proc> {
        self.nmi.as_mut()
    }
    pub fn has_nmi(&self) -> bool {
        self.nmi.is_some()
    }

    pub fn set_hwi(&mut self, hwi: Proc) {
        self.hwi = Some(hwi)
    }
    pub fn get_hwi(&self) -> Option<&Proc> {
        self.hwi.as_ref()
    }
    pub fn get_hwi_mut(&mut self) -> Option<&mut Proc> {
        self.hwi.as_mut()
    }
    pub fn has_hwi(&self) -> bool {
        self.hwi.is_some()
    }

    pub fn set_swi(&mut self, swi: Proc, swi_id: u8) {
        self.swi_table[swi_id as usize] = Some(swi);
    }
    pub fn get_swi(&self, swi_id: u8) -> Option<&Proc> {
        self.swi_table[swi_id as usize].as_ref()
    }
    pub fn get_swi_mut(&mut self, swi_id: u8) -> Option<&mut Proc> {
        self.swi_table[swi_id as usize].as_mut()
    }
    pub fn has_swi(&self, swi_id: u8) -> bool {
        self.swi_table[swi_id as usize].is_some()
    }

    /// Insert a procedure into the procedure table.
    /// If the procedure table did not include this procedure, None is returned.
    /// Otherwise, the procedure is updated and the old procedure is returned.
    pub fn set_proc(&mut self, proc: Proc) -> Option<Proc> {
        self.proc_table.insert(*proc.name(), proc)
    }
    /// Get a non-entry or -interrupt handler procedure from this program's procedure table.
    pub fn get_proc(&mut self, proc_name: &str) -> Option<&Proc> {
        self.proc_table.get(proc_name)
    }
    /// Mutable access to non-entry or -interrupt handler procedures in this program.
    pub fn get_proc_mut(&mut self, proc_name: &str) -> Option<&mut Proc> {
        self.proc_table.get_mut(proc_name)
    }
    /// Check existence of non-entry or -interrupt handler procedure in this program.
    pub fn has_proc(&self, proc_name: &str) -> bool {
        self.proc_table.contains_key(proc_name)
    }
}

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
