use crate::{basicblock::TypeStackEntry, logger::Logger, procedure::{Procedure, ProcedureTable}, statement::{DType, Literal, StatementKind}};
use std::{collections::{HashMap, HashSet, VecDeque}, iter};

// virtual registers have an id and hold a value of a given type
#[derive(Debug, Clone, PartialEq)]
pub struct VReg {
    id: usize,
    dtype: DType,
}
impl VReg {
    pub fn new(id: usize, dtype: DType) -> VReg {
        VReg { id, dtype }
    }
    pub fn id(&self) -> &usize {
        &self.id
    }
    pub fn dtype(&self) -> &DType {
        &self.dtype
    }
}

// register instructions are a bit different from the stack statements
#[derive(Debug)]
pub enum RegInstruction {
    // load a register with a literal value
    LoadImm(VReg, Literal),
    // don't need pop or dup
    // move dest source
    Mov(VReg, VReg),
    // memory control
    // load the first register with the value pointed at by the second register as a given type
    // load dest ptr type
    Load(VReg, VReg, DType),
    // store the first register at the location pointed at by the second register as a given type
    // store from ptr type
    Store(VReg, VReg, DType),

    // operators
    // args are always dest, source1, source2... in left-right order
    Add(VReg, VReg, VReg),
    Sub(VReg, VReg, VReg),
    Mul(VReg, VReg, VReg),
    Div(VReg, VReg, VReg),
    Mod(VReg, VReg, VReg),
    Inc(VReg, VReg),
    Dec(VReg, VReg),
    And(VReg, VReg, VReg),
    Or(VReg, VReg, VReg),
    Not(VReg, VReg),
    Xor(VReg, VReg, VReg),
    Bsl(VReg, VReg),
    Bsr(VReg, VReg),
    Rol(VReg, VReg),
    Ror(VReg, VReg),
    Eq(VReg, VReg, VReg),
    Neq(VReg, VReg, VReg),
    Lt(VReg, VReg, VReg),
    Leq(VReg, VReg, VReg),
    Gt(VReg, VReg, VReg),
    Geq(VReg, VReg, VReg),

    // type conversion
    Cast(VReg, VReg, DType),
    Conv(VReg, VReg, DType),

    // control flow
    Label(String),
    // no instruction for procedure declaration because we eliminated that at the IR level
    Jump(String),
    JumpIf(VReg, String),
    // call name inputs outputs
    Call(String, Vec<VReg>, Vec<VReg>),
    // return to last thing on the call stack
    // return values are handled with register allocations and moves
    Ret
}

pub type RegProcedureTable = HashMap<String, RegProcedure>;
pub fn convert_proc_table(ir_table: &ProcedureTable, logger: &mut dyn Logger) -> RegProcedureTable {
    let mut table = RegProcedureTable::new();
    for ir_proc in ir_table.values() {
        if ir_proc.reachable() {
            let converted = RegProcedure::lower_ir_procedure(ir_proc, ir_table); 
            table.insert(converted.name.clone(), converted);
        }
        else {
            logger.warning("unreachable code".to_string(), ir_proc.pos().line, ir_proc.pos().col);
        }
    }
    table
}

// instead of procedure declarations we have these things now
pub struct RegProcedure {
    pub name: String,
    pub inputs: Vec<VReg>,
    pub outputs: Vec<VReg>,
    pub instructions: Vec<RegInstruction>
}
impl RegProcedure {
    pub fn lower_ir_procedure(ir_proc: &Procedure, ir_table: &ProcedureTable) -> RegProcedure {
        // this converts ir procedures to register procedures
        let mut next_id: usize = 0;
        let mut new_reg = |dtype: DType| -> VReg {
            let reg = VReg{ id: next_id, dtype };
            next_id += 1;
            reg
        };
        // the great thing about stack languages is we don't need to worry about ssa or phi nodes
        // or any of that. in order to take advantage of that though we have to allocate registers
        // block by block. this is probably not as optimization friendly as straight up ssa but
        // we're not after speed here.
        // array of instructions to emit
        // TODO have to emit instructions by block!
        let mut block_instructions: HashMap<usize, Vec<RegInstruction>> = HashMap::new();
        // allocating the registers is a little complicated
        // we can't just go through in source order and allocate, because there might be back edges
        // we also can't keep a procedure-level stack and allocate per block
        // what we do is keep a map of entry registers to block ids
        let mut block_entry_regs: HashMap<usize, Vec<VReg>> = HashMap::new();
        // as we process blocks in topological order (by following the successor lists), we
        // propagate the register stack through to each block's entry
        // in blocks that can be arrived at from multiple places (where the entry register stack is
        // already populated), we emit move instructions to reconcile the stacks
        // first we seed the register block map with the entry block's entry stack
        let mut proc_entry_regs: Vec<VReg> = vec![];
        for arg in ir_proc.get_intypes().iter() {
            proc_entry_regs.push(new_reg(arg.clone()));
        }
        block_entry_regs.insert(0, proc_entry_regs);
        // we also set up the output registers
        // we need as many of these as we have procedure outputs in the signature
        // whenever a ret happens, we move outputs into these
        let mut proc_exit_regs: Vec<VReg> = vec![];
        for arg in ir_proc.get_outtypes().iter() {
            proc_exit_regs.push(new_reg(arg.clone()));
        }
        // then we just do a worklist
        let mut todo_ids: VecDeque<usize> = vec![0].into();
        let mut visited: HashSet<usize> = HashSet::new();

        while !todo_ids.is_empty() {
            let block_id = todo_ids.pop_back().unwrap();
            // skip if we've already done this block
            if visited.contains(&block_id) {
                continue;
            }
            let block = &ir_proc.get_blocks()[block_id];
            // instructions vector for this block
            let mut instructions = vec![];
            // seed our register stack from the block entry map
            let mut stack_regs = block_entry_regs.get(&block_id).unwrap().clone();
            // sanity check, make sure the register stack on entry to this block looks like the
            // type stack we validated earlier
            println!("{} {:?} {:?}", block_id, block.entry_stack, stack_regs);
            for (reg, tse) in stack_regs.iter().rev().zip(block.entry_stack.iter()) {
                match (tse, reg) {
                    (TypeStackEntry::Known(tse_type), VReg { id: _, dtype } ) => {
                        if *tse_type != *dtype {
                            panic!("type and register stack incompatibility after type check")
                        }
                    }
                    _ => {
                        panic!("unknown type stack entry after type resolution")
                    }
                }
            }
            // allocate and drop registers as needed
            for s in &ir_proc.get_statements()[block.start..block.start + block.length] {
                println!("{}", s);
                println!("{:?}", stack_regs);
                match s.kind() {
                    StatementKind::Push { value } => {
                        // allocate a new register, load it with the value, push it to the stack
                        let r = new_reg(value.clone().into());
                        instructions.push(RegInstruction::LoadImm(r.clone(), value.clone()));
                        stack_regs.push(r);
                    }
                    StatementKind::Pop => {
                        // just drop the last register
                        // good to panic here if something breaks
                        stack_regs.pop().unwrap();
                    }
                    StatementKind::Dup => {
                        // duplicate the top register and push it
                        let top = stack_regs.last().unwrap();
                        stack_regs.push(top.clone());
                    }
                    StatementKind::Swap => {
                        // swap the top two registers
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        stack_regs.push(a);
                        stack_regs.push(b);
                    }
                    StatementKind::Load { kind } => {
                        // allocate a new register, and load the top of the stack into it
                        let ptr = stack_regs.pop().unwrap();
                        // sanity check
                        if ptr.dtype != DType::Pointer { panic!() }
                        let r = new_reg(kind.clone());
                        instructions.push(RegInstruction::Load(r.clone(), ptr, kind.clone()));
                        stack_regs.push(r);
                    }
                    StatementKind::Store { kind } => {
                        // store the top of the stack to the location at the second position in the
                        // stack
                        let val = stack_regs.pop().unwrap();
                        let ptr = stack_regs.pop().unwrap();
                        if ptr.dtype != DType::Pointer { panic!() }
                        instructions.push(RegInstruction::Store(ptr, val, kind.clone()));
                    }
                    StatementKind::Label { name } => {
                        instructions.push(RegInstruction::Label(name.clone()));
                    }
                    StatementKind::Jump { dest } => {
                        instructions.push(RegInstruction::Jump(dest.clone()));
                    }
                    StatementKind::Jumpif { dest } => {
                        let cond = stack_regs.pop().unwrap();
                        instructions.push(RegInstruction::JumpIf(cond, dest.clone()));
                    }
                    StatementKind::Call { dest } => {
                        // look up destination signature to know how many ins/outs
                        let dest_proc = ir_table.get(dest).unwrap();
                        // pop as many inputs as we need from the reg stack
                        let mut call_in_regs: Vec<VReg> = dest_proc.get_intypes().iter().map(|_| stack_regs.pop().unwrap()).collect();
                        call_in_regs.reverse();
                        // allocate as many outputs as we need
                        let call_out_regs: Vec<VReg> = dest_proc.get_outtypes().iter().map(|dt| new_reg(dt.clone())).collect();
                        // emit that instruction
                        instructions.push(RegInstruction::Call(dest.clone(), call_in_regs, call_out_regs.clone()));
                        // push the outputs to the stack
                        for or in call_out_regs {
                            stack_regs.push(or);
                        }
                    }
                    StatementKind::Ret => {
                        // pop as many registers off the stack as we need
                        let outs: Vec<VReg> = ir_proc.get_outtypes().iter().map(|_| stack_regs.pop().unwrap()).collect();
                        // move them into the return registers we allocated
                        for (exit_reg, from) in proc_exit_regs.iter().rev().zip(outs.iter()) {
                            if exit_reg.dtype != from.dtype {
                                panic!("type mismatch between allocated return registers and working registers at proc return")
                            }
                            instructions.push(RegInstruction::Mov(exit_reg.clone(), from.clone()));
                        }
                        instructions.push(RegInstruction::Ret);
                        // just sanity check, the stack should be empty now
                        if !stack_regs.is_empty() { panic!("nonempty stack at ret call during ir lower") }
                    }
                    StatementKind::Cast { to } => {
                        let src = stack_regs.pop().unwrap();
                        let r = new_reg(to.clone());
                        instructions.push(RegInstruction::Cast(r.clone(), src, to.clone()));
                        stack_regs.push(r);
                    }
                    StatementKind::Conv { to } => {
                        let src = stack_regs.pop().unwrap();
                        let r = new_reg(to.clone());
                        instructions.push(RegInstruction::Conv(r.clone(), src, to.clone()));
                        stack_regs.push(r);
                    }
                    StatementKind::Proc { .. } => {
                        // if a procedure makes it in here something's really wrong
                        panic!("nested proc during ir lowering")
                    }
                    // now all the operators
                    // watch the order on two-arg ops!
                    StatementKind::Add => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Add(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Sub => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Sub(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Div => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Div(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Mult => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Mul(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Mod => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Mod(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Inc => {
                        let a = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Inc(dest.clone(), a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Dec => {
                        let a = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Dec(dest.clone(), a));
                        stack_regs.push(dest);
                    }
                    StatementKind::And => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::And(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Or => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Or(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Not => {
                        let a = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Not(dest.clone(), a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Xor => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Xor(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Bsl => {
                        let a = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Bsl(dest.clone(), a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Bsr => {
                        let a = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Bsr(dest.clone(), a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Rol => {
                        let a = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Rol(dest.clone(), a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Ror => {
                        let a = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Ror(dest.clone(), a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Eq => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Eq(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Neq => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Neq(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Lt => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Lt(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Leq => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Leq(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Gt => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Gt(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                    StatementKind::Geq => {
                        let a = stack_regs.pop().unwrap();
                        let b = stack_regs.pop().unwrap();
                        let dest = new_reg(a.dtype.clone());
                        instructions.push(RegInstruction::Geq(dest.clone(), b, a));
                        stack_regs.push(dest);
                    }
                }
            }

            // now stack_regs contains this block's exit stack
            // do another sanity check, make sure it aligns with what we discovered in type
            // analysis
            for (tse, reg) in block.exit_stack.iter().zip(stack_regs.iter().rev()) {
                match (tse, reg) {
                    (TypeStackEntry::Known(tse_type), VReg { id: _, dtype } ) => {
                        if *tse_type != *dtype {
                            panic!("type and register stack incompatibility after type check")
                        }
                    }
                    _ => {
                        panic!("unknown type stack entry after type resolution")
                    }
                }
            }
            // then propagate this exit stack to the successors and queue them for visitation
            for succ_id in &block.successors {
                // if there's no entry registers for that block, insert directly
                block_entry_regs.entry(*succ_id).or_insert_with(|| stack_regs.clone());

                // if there are already registers there, reconcile with moves
                if let Some(existing) = block_entry_regs.get(succ_id) && *existing != *stack_regs {
                    for (existing_reg, current_reg) in existing.iter().zip(stack_regs.iter()) {
                        if existing_reg != current_reg {
                            instructions.push(RegInstruction::Mov(existing_reg.clone(), current_reg.clone()));
                        }
                    }
                }
            }

            // mark self visited
            visited.insert(block_id);
            // queue all successors for visitation
            for succ_id in &block.successors {
                todo_ids.push_front(*succ_id);
            }

            block_instructions.insert(block_id, instructions);
        }
        
        // collect all the block instructions in source order
        let mut collected_instructions: Vec<RegInstruction> = vec![];
        for (b_i, _) in ir_proc.get_blocks().iter().enumerate() {
            collected_instructions.append(block_instructions.get_mut(&b_i).unwrap());
        }

        RegProcedure { name: ir_proc.name().clone(), inputs: block_entry_regs.get(&0).unwrap().clone(), outputs: proc_exit_regs, instructions: collected_instructions }
    }
}
