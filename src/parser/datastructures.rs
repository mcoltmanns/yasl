use std::cmp::PartialEq;
use std::fmt::{Display, Formatter};
use crate::primitives::{BasicBlock, DType, Literal};
use crate::util::Positioned;

#[derive(Debug)]
pub enum StackStatement {
    Push { value: Literal },
    Pop,
    Dup,
    Swap,
    Load { kind: DType },
    Store { kind: DType },
    Label { name: String },
    Jump { dest: String },
    Jumpif { dest: String },
    Call { dest: String, t_in: Vec<DType>, t_out: Vec<DType> },
    Ret { t_out: Vec<DType>},
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
    Not,
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

#[derive(Debug)]
pub enum ProcedureKind { Main, Default, Swi(u8), Hwi, Nmi }
impl PartialEq for ProcedureKind {
    fn eq(&self, other: &Self) -> bool {
        match (self,other) {
            (&ProcedureKind::Main, &ProcedureKind::Main) => true,
            (&ProcedureKind::Default, &ProcedureKind::Default) => true,
            (&ProcedureKind::Hwi, &ProcedureKind::Hwi) => true,
            (&ProcedureKind::Nmi, &ProcedureKind::Nmi) => true,
            (&ProcedureKind::Swi(a), &ProcedureKind::Swi(b)) => a == b,
            _ => false,
        }
    }
}

pub struct StackProcedure {
    name: String,
    kind: ProcedureKind,
    id: usize,
    external: bool,
    /// The types this procedure expects
    types_in: Vec<DType>,
    /// The types this procedure leaves
    types_out: Vec<DType>,
    ///// Map labels in this procedure to indices into the basic block array
    //jump_table: HashMap<String, usize>,
    ///// Basic blocks in this procedure
    //blocks: Vec<BasicBlock>,
    ///// Which blocks follow which?
    ///// block_links[a, b] == true -> b follows a
    //block_links: Vec<Vec<bool>>,
    /// Statements in this procedure
    statements: Vec<Positioned<StackStatement>>,
    /// Basic blocks in this procedure
    blocks: Vec<BasicBlock>,
}
impl StackProcedure {
    pub fn new(name: String, external: bool, types_in: Vec<DType>, types_out: Vec<DType>) -> Result<Self, String> {
        let mut kind: ProcedureKind = ProcedureKind::Default;
        match name.as_str() {
            "main" => {
                kind = ProcedureKind::Main;
                if external {
                    return Err("main procedure may not be marked external".to_string())
                }
            }
            "hwi" => {
                kind = ProcedureKind::Hwi;
                if external {
                    return Err("hwi procedure may not be marked external".to_string())
                }
            }
            "nmi" => {
                kind = ProcedureKind::Nmi;
                if external {
                    return Err("nmi procedure may not be marked external".to_string())
                }
            }
            _ => {
                if name.starts_with("swi") {
                    let id = name.trim_start_matches("swi").parse::<u8>().map_err(
                        |_| format!("invalid swi handler id: {}", name.trim_start_matches("swi"))
                    )?;
                    kind = ProcedureKind::Swi(id);
                }
            }
        }
        Ok(StackProcedure {
            name,
            kind,
            external,
            id: usize::MAX,
            types_in,
            types_out,
            statements: Vec::new(),
            blocks: Vec::new(),
        })
    }

    pub fn set_statements(&mut self, statements: Vec<Positioned<StackStatement>>) {
        self.statements = statements;
    }

    pub fn is_extern(&self) -> bool {
        self.external
    }
}

pub struct StackProgram {
    procedures: Vec<Positioned<StackProcedure>>,
    entry_id: Option<usize>,
    nmi_id: Option<usize>,
    hwi_id: Option<usize>,
    swi_ids: [Option<usize>; 256],
    // constants are replaced during parsing and are declare-before use, so no need to track them further
}

impl Display for StackProgram {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl StackProgram {
    pub fn empty() -> StackProgram {
        StackProgram {
            procedures: Vec::new(),
            entry_id: None,
            nmi_id: None,
            hwi_id: None,
            swi_ids: [None; 256],
        }
    }

    pub fn add_procedure(&mut self, mut procedure: Positioned<StackProcedure>) -> Result<usize, String> {
        procedure.id = self.procedures.len();

        match procedure.kind {
            ProcedureKind::Main => {
                if self.entry_id.is_some() {
                    return Err("illegal redefinition of main procedure".to_string());
                }
                self.entry_id = Some(procedure.id);
            }
            ProcedureKind::Default => {}
            ProcedureKind::Swi(id) => {
                if self.swi_ids[id as usize].is_some() {
                    return Err(format!("illegal redefinition of swi handler {}", id));
                }
                self.swi_ids[id as usize] = Some(procedure.id);
            }
            ProcedureKind::Nmi => {
                if self.nmi_id.is_some() {
                    return Err("illegal redefinition of nmi handler".to_string());
                }
                self.nmi_id = Some(procedure.id);
            }
            ProcedureKind::Hwi => {
                if self.hwi_id.is_some() {
                    return Err("illegal redefinition of hwi handler".to_string());
                }
                self.hwi_id = Some(procedure.id);
            }
        }

        self.procedures.push(procedure);

        Ok(self.procedures.len() - 1)
    }

    pub fn get_entry_id(&self) -> Option<usize> {
        self.entry_id
    }

    pub fn get_entry(&self) -> Option<&Positioned<StackProcedure>> {
        if let Some(entry_id) = self.entry_id {
            return self.procedures.get(entry_id)
        }
        None
    }
}
