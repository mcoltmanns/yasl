use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fmt::Display;

use crate::datastructures::TypeStackEntry;
use crate::datastructures::basicblock::BasicBlock;
use crate::datastructures::statement::DType;
use crate::datastructures::statement::StatementPayload;
use crate::datastructures::statement::VirtualStatement;
use crate::datastructures::statement::VRegInstruction;
use crate::datastructures::TypeStack;
use crate::logger::Logger;
use crate::regmachine::VReg;
use crate::regmachine::VRegAllocator;
use crate::util::FilePos;
use crate::util::Positionable;

#[derive(Debug)]
pub struct VirtualProcedure {
    name: String,
    types_in: Vec<DType>,
    types_out: Vec<DType>,

    pos: FilePos,

    jump_table: HashMap<String, usize>,
    blocks: Vec<BasicBlock>,
    // (from, to)
    block_links: Vec<Vec<bool>>,
    statements: Vec<VirtualStatement>,
    block_entry_stacks: HashMap<usize, Vec<TypeStackEntry>>,
}
impl VirtualProcedure {
    pub fn empty(name: String, types_in: Vec<DType>, types_out: Vec<DType>, pos: FilePos) -> Self {
        VirtualProcedure { name, types_in, types_out, pos, jump_table: HashMap::new(), blocks: vec![], block_links: vec![], statements: vec![], block_entry_stacks: HashMap::new() }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn blocks(&self) -> &[BasicBlock] {
        &self.blocks
    }

    pub fn statements(&self) -> &[VirtualStatement] {
        &self.statements
    }

    pub fn set_statements(&mut self, statements: Vec<VirtualStatement>) {
        self.statements = statements
    }

    pub fn types_in(&self) -> &[DType] {
        &self.types_in
    }

    pub fn types_out(&self) -> &[DType] {
        &self.types_out
    }

    pub fn view_block(&self, block_i: usize) -> Option<&[VirtualStatement]> {
        let block = self.blocks().get(block_i)?;
        Some(&self.statements()[block.start()..block.start() + block.length()])
    }

    pub fn succ_ids(&self, block_i: usize) -> Option<Vec<usize>> {
        Some(self.block_links.get(block_i)?.iter().enumerate().filter(|(_succ_id, linked)| { **linked }).map(|(succ_id, _linked)| { succ_id }).collect())
    }

    pub fn build_blocks_and_jumps(&mut self, logger: &mut dyn Logger) {
        if self.statements.is_empty() {
            logger.error("undefined procedure", self.pos.clone());
        }

        let mut current_block: Option<BasicBlock> = None;

        for (i, s) in self.statements.iter().enumerate() {
            match s.payload() {
                // labels start a new block and end the last one
                // labels are included in the new block
                StatementPayload::Label { name } => {
                    if let Some(mut last_block) = current_block.take() {
                        last_block.set_length(i - last_block.start());
                        self.blocks.push(last_block);
}
                    current_block = Some(BasicBlock::new(i, 0, s.pos().clone()));
                    // also add to the jump table
                    if self.jump_table.insert(name.clone(), self.blocks().len()).is_some() {
                        logger.error("duplicate label", s.pos().clone());
                    }
                }
                // control flow changes start a new block and end the last one
                // control flow changes are included in the old block
                StatementPayload::Jump { .. }
                | StatementPayload::Jumpif { .. }
                | StatementPayload::Ret => {
                    if let Some(mut last_block) = current_block.take() {
                        last_block.set_length(i + 1 - last_block.start());
                        self.blocks.push(last_block);
                    }
                    else {
self.blocks.push(BasicBlock::new(i, 1, s.pos().clone()));
    }
                }
                // anything else starts a new block if we aren't already on one
                _ => {
                    if current_block.is_none() {
                        current_block = Some(BasicBlock::new(i, 0, s.pos().clone()))
                    }
                }
            }
        }
        // close last block
        if let Some(mut last_block) = current_block.take() {
            last_block.set_length(self.statements.len() - last_block.start());
            self.blocks.push(last_block);
        }

        self.block_links = vec![vec![false; self.blocks.len()]; self.blocks.len()];
    }

    pub fn link_blocks(&mut self, logger: &mut dyn Logger) {
        for b_i in 0..self.blocks.len() {
            // last statement in a block determines successors
            let s = self.view_block(b_i).expect("block index out of range").last().expect("no statements in block").clone(); // this clone is ok, it's only one statement
            match s.payload() {
                // jumps are succeeded by block they call
                StatementPayload::Jump { dest } => {
                    if let Some(successor_id) = self.jump_table.get(dest) {
                        self.block_links[b_i][*successor_id] = true;
                    }
                    else {
                        logger.error("invalid jump destination", s.pos().clone());
                    }
                }
                // jumpifs are succeeded by block they call, and block after them
                StatementPayload::Jumpif { dest } => {
                    if let Some(successor_id) = self.jump_table.get(dest) {
                        self.block_links[b_i][*successor_id] = true;
                    }
                    else {
logger.error("invalid jump destination", s.pos().clone());
                    }
                    let next_id = b_i + 1;
                    if next_id < self.blocks.len() {
                        self.block_links[b_i][next_id] = true;
                    }
                    else {
                        logger.error("no fallthrough block after conditional jump", s.pos().clone());
                    }
                }
                // rets have no successors
                StatementPayload::Ret => {}
                // anything else is succeeded by the block after if
                _ => {
                    let next_id = b_i + 1;
                    if next_id < self.blocks.len() {
                        self.block_links[b_i][next_id] = true;
                    }
                }
            }
        }
    }

    pub fn check_block_reachability(&self, logger: &mut dyn Logger) {
        // first block is always reachable, so we skip it
        for b_i in 1..self.blocks.len() {
            let mut reachable = false;
            // you are reachable if there is a link from any other to you
            for other_i in 0..self.blocks.len() {
                // skip self links
                if b_i == other_i { continue; }
                reachable |= self.block_links[other_i][b_i];
            }
            if !reachable {
                logger.warning("unreachable code", self.blocks[b_i].pos().clone());
            }
        }
    }

    pub fn compute_block_stack_effets(&mut self, sig_table: &HashMap<String, (Vec<DType>, Vec<DType>)>, logger: &mut dyn Logger) {
        for block in self.blocks.iter_mut() {
            let mut tracking: TypeStack = vec![];

            // pop from the tracking stack
            // if there is a value available, return that
            // if there isn't, increment the pops for the current block and return an entry that
            // depends on that pop
            fn pop(tracking: &mut TypeStack, block: &mut BasicBlock) -> TypeStackEntry {
                tracking.pop().unwrap_or_else(|| -> TypeStackEntry {
                    block.inc_pops();
                    TypeStackEntry::Depends(block.pops() - 1)
                })
            }

            // simulate the tracking stack through the statements
            // allocated more space on the tracking stack if necessary
            for s in self.statements[block.start()..block.start() + block.length()].iter() {
                match s.payload() {
                    // pushes only add to type stack
                    StatementPayload::Push { value } => { tracking.push(TypeStackEntry::Known(value.into())); }
                    // pops only discard
                    StatementPayload::Pop => { pop(&mut tracking, block); }
                    // dup duplicates type at top
                    StatementPayload::Dup => {
                        let top = pop(&mut tracking, block);
                        tracking.push(top.clone());
                        tracking.push(top);
                    }
                    StatementPayload::Swap => {
                        let a = pop(&mut tracking, block);
                        let b = pop(&mut tracking, block);
                        tracking.push(a);
                        tracking.push(b);
                    }
                    // two-input ops
                    StatementPayload::Add
                    | StatementPayload::Sub
                    | StatementPayload::Mult
                    | StatementPayload::Div
                    | StatementPayload::Mod
                    | StatementPayload::And
                    | StatementPayload::Or
                    | StatementPayload::Xor
                    | StatementPayload::Eq
                    | StatementPayload::Neq
                    | StatementPayload::Lt
                    | StatementPayload::Leq
                    | StatementPayload::Gt
                    | StatementPayload::Geq => {
                        let a = pop(&mut tracking, block);
                        let b = pop(&mut tracking, block);
                        // try to resolve depending on a or b, else depend on a
                        let result = match (&a, &b) {
                            (TypeStackEntry::Known(_), _) => a.clone(),
                            (_, TypeStackEntry::Known(_)) => b.clone(),
                            _ => a.clone()
                        };
                        tracking.push(result);
                    }
                    // one-input ops
                    StatementPayload::Inc
                    | StatementPayload::Dec
                    | StatementPayload::Bsl
                    | StatementPayload::Bsr
                    | StatementPayload::Rol
                    | StatementPayload::Ror
                    | StatementPayload::Not => {
                        let a = pop(&mut tracking, block);
                        tracking.push(a);
                },
                    // load requires a pointer and leaves its kind
                    StatementPayload::Load { kind } => {
                        pop(&mut tracking, block);
                        tracking.push(TypeStackEntry::Known(*kind));
                    }
                    // store requires a pointer and its kind
                    StatementPayload::Store { .. } => {
                        pop(&mut tracking, block);
                        pop(&mut tracking, block);
                    }
                    // cast and conv require one thing and leave the thing they convert to
                    StatementPayload::Cast { to }
                    | StatementPayload::Conv { to } => {
                        pop(&mut tracking, block);
                        tracking.push(TypeStackEntry::Known(*to));
                    }
                    // call info is in the proc table
                    StatementPayload::Call { dest } => {
                        if let Some((inputs, outputs)) = sig_table.get(dest) {
                            // iteration order doesn't matter here, because we're only counting
                            for _ in inputs.iter() {
                                pop(&mut tracking, block);
                            }
                            for output in outputs.iter() {
                                tracking.push(TypeStackEntry::Known(*output));
                            }
                        }
                        else {
                            logger.error("call to unknown procedure", s.pos().clone());
                            // no point continuing if we can't figure out the stack
                            return;
                        }
                    }
                    // jumpif requires one int
                    StatementPayload::Jumpif { .. } => {
                        pop(&mut tracking, block);
                    }
                    // ret requires as many things as the procedure declares
                    StatementPayload::Ret => {
                        for _ in self.types_out.iter() {
                            pop(&mut tracking, block);
                        }
                    }
                    // nothing else has an effect on the stack
                    StatementPayload::Label { .. }
                    | StatementPayload::Jump { .. }
                    | StatementPayload::Proc { .. } => {}
                    _ => { unimplemented!() }
                };
            }
            block.set_pushes(tracking);
        }
    }

    pub fn resolve_types(&mut self, sig_table: &HashMap<String, (Vec<DType>, Vec<DType>)>, logger: &mut dyn Logger) {
        let mut todo_ids: VecDeque<usize> = [0].into();
        let mut visited: HashSet<usize> = HashSet::new();
        // entry stack map, preload 0th block with procedure inputs
        self.block_entry_stacks = HashMap::new();
        self.block_entry_stacks.insert(0, self.types_in.iter().map(|t| TypeStackEntry::Known(*t)).collect());

        while !todo_ids.is_empty() {
            let current_id = todo_ids.pop_back().unwrap();
            // skip this block if we've already visited it or the index information makes no sense
            // if the index is wrong this block is unreachable, but we already warn about that
            // during block linking
            if visited.contains(&current_id) || current_id >= self.blocks.len() {
                continue;
            }
            let current = &mut self.blocks[current_id];
            visited.insert(current_id);

            // if this block has no successors, it must end in a return statement
            if self.block_links[current_id].iter().all(|v| !*v) {
                let last = &self.statements[current.start() + current.length() - 1];
                if !matches!(last.payload(), StatementPayload::Ret) {
                    logger.error("procedure path does not end in return statement", current.pos().clone());
                }
            }

            // get your entry stack
            let entry_stack = self.block_entry_stacks.get(&current_id).unwrap();
            //println!("{}{} stack is {:?}", self.name, current_id, entry_stack);
            // check for stack underflow
            if entry_stack.len() < current.pops() {
                logger.error("stack underflow", current.pos().clone());
                continue;
            }
            // compute exit stack
            // remember young is right
            // so exit stack before pushes is entry stack without the last <pops> elements
            let mut exit_stack: Vec<TypeStackEntry> = entry_stack[..entry_stack.len() - current.pops()].to_vec();
            for push in current.pushes().iter() {
                match push {
                    TypeStackEntry::Known(_) => exit_stack.push(push.clone()),
                    TypeStackEntry::Unknown => logger.error("type resolution failed", current.pos().clone()),
                    TypeStackEntry::Depends(i) => {
                        // in this case we depend on a value in the entry stack
                        // the popped region of the entry is entry[entry.len() - pops..]
                        // and i indexes into the popped region
                        if let Some(t) = entry_stack.get(entry_stack.len() - current.pops() + i) {
                            exit_stack.push(t.clone());
                        }
                        else {
                            panic!("invalid type dependency index")
                        }
                    }
                }
            }
            
            // check types
            // simulate this block on the entry stack
            // also set types for math ops
            let mut sim_stack = entry_stack.to_vec();
            for s in self.statements[current.start()..current.start() + current.length()].iter_mut() {
                match s.payload() {
                    // pushes only add to type stack
                    StatementPayload::Push { value } => { sim_stack.push(TypeStackEntry::Known(value.into())); }
                    // pops only discard
                    StatementPayload::Pop => { sim_stack.pop().unwrap(); }
                    // dup duplicates type at top
                    StatementPayload::Dup => {
                        let top = sim_stack.pop().unwrap();
                        sim_stack.push(top.clone());
                        sim_stack.push(top);
                    }
                    StatementPayload::Swap => {
                        let a = sim_stack.pop().unwrap();
                        let b = sim_stack.pop().unwrap();
                        sim_stack.push(a);
                        sim_stack.push(b);
                    }
                    // two-input ops
                    StatementPayload::Add
                    | StatementPayload::Sub
                    | StatementPayload::Mult
                    | StatementPayload::Div
                    | StatementPayload::Mod
                    | StatementPayload::And
                    | StatementPayload::Or
                    | StatementPayload::Xor
                    | StatementPayload::Eq
                    | StatementPayload::Neq
                    | StatementPayload::Lt
                    | StatementPayload::Leq
                    | StatementPayload::Gt
                    | StatementPayload::Geq => {
                        let a = sim_stack.pop().unwrap();
                        let b = sim_stack.pop().unwrap();
                        // try to resolve depending on a or b, else depend on a
                        match (&a, &b) {
                            (TypeStackEntry::Known(a_conc), TypeStackEntry::Known(b_conc)) => {
                                if a_conc != b_conc {
                                    logger.error("two-input operators require equal type arguments", s.pos().clone());
                                    sim_stack.push(TypeStackEntry::Unknown);
                                }
                                else {
                                    s.set_type(*a_conc);
                                    sim_stack.push(a);
                                }
                            }
                            _ => {
                                logger.error("type resolution failed", s.pos().clone());
                                sim_stack.push(TypeStackEntry::Unknown);
                            }
                        };
                    }
                    // one-input ops
                    StatementPayload::Inc
                    | StatementPayload::Dec
                    | StatementPayload::Bsl
                    | StatementPayload::Bsr
                    | StatementPayload::Rol
                    | StatementPayload::Ror
                    | StatementPayload::Not => {
                        let a = sim_stack.pop().unwrap();
                        match &a {
                            TypeStackEntry::Known(a_conc) => {
                                s.set_type(*a_conc);
                                sim_stack.push(a);
                            }
                            _ => logger.error("type resolution failed", s.pos().clone()),
                        }
                    },
                    // load requires a pointer and leaves its kind
                    StatementPayload::Load { kind } => {
                        if let TypeStackEntry::Known(dtype) = sim_stack.pop().unwrap() {
                            if !matches!(dtype, DType::Pointer) {
                                logger.error("load requires a pointer argument to load from", s.pos().clone());
                            }
                        }
                        else {
                            logger.error("type resolution failed", s.pos().clone());
                        }
                        sim_stack.push(TypeStackEntry::Known(*kind));
                    }
                    // store requires a pointer and its kind
                    // top is value, next is pointer
                    StatementPayload::Store { kind } => {
                        let value = sim_stack.pop().unwrap();
                        let pointer = sim_stack.pop().unwrap();
                        match (value, pointer) {
                            (TypeStackEntry::Known(val_t), TypeStackEntry::Known(p_t)) => {
                                if val_t != *kind || !matches!(p_t, DType::Pointer) {
                                    logger.error("store requires, in order from top of stack, a value to store which must match the type declared and a pointer to store the value at", s.pos().clone());
                    }
                            }
                            _ => {
                                logger.error("type resolution failed", s.pos().clone());
                            }
                        }
                    }
                    // cast and conv require one thing and leave the thing they convert to
                    StatementPayload::Cast { to }
                    | StatementPayload::Conv { to } => {
                        sim_stack.pop().unwrap();
                        sim_stack.push(TypeStackEntry::Known(*to));
                        s.set_type(*to);
                    }
                    // call info is in the proc table
                    StatementPayload::Call { dest } => {
                        if let Some((inputs, outputs)) = sig_table.get(dest) {
                            // require the things in the input
                            for expect in inputs.iter().rev() {
                                let actual = sim_stack.pop().unwrap();
                                match actual {
                                    TypeStackEntry::Known(actual_type) => {
                                        if actual_type != *expect {
                                            logger.error("incorrect arguments to procedure", s.pos().clone());
                                        }
                                    }
                                    _ => {
                                        logger.error("type resolution failed", s.pos().clone());
                                    }
                                }
                            }
                            // just push the things in the output
                            for output in outputs.iter() {
                                sim_stack.push(TypeStackEntry::Known(*output));
                            }
                        }
                        else {
                            logger.error("call to unknown procedure", s.pos().clone());
                            // no point continuing if we can't figure out the stack
                            return;
                        }
                    }
                    // jumpif requires one int
                    StatementPayload::Jumpif { .. } => {
                        let condition = sim_stack.pop().unwrap();
                        match condition {
                            TypeStackEntry::Known(cond_type) => {
                                if !cond_type.is_integer() {
                                    logger.error("conditional jump argument must be integer type", s.pos().clone());
                                }
                            }
                            _ => {
                                logger.error("type resolution failed", s.pos().clone());
                            }
                        }
                    }
                    // ret requires as many things as the procedure declares
                    StatementPayload::Ret => {
                        for expect in self.types_out.iter().rev() {
                            let actual = sim_stack.pop().unwrap();
                            match actual {
                                TypeStackEntry::Known(actual_type) => {
                                    if *expect != actual_type {
                                        logger.error(&format!("returned type ({:?}) does not match procedure declaration ({:?})", actual_type, expect), s.pos().clone());
                                    }
                                }
                                _ => {
                                    logger.error("type resolution failed", s.pos().clone());
                                }
                            }
                        }
                    }
                    // nothing else has an effect on the stack
                    StatementPayload::Label { .. }
                    | StatementPayload::Jump { .. }
                    | StatementPayload::Proc { .. } => {}
                    _ => { unimplemented!() }
                };
            }

            //println!("get {:?}", entry_stack);
            //println!("leave {:?}", exit_stack);
            // then propagate the output stack to the successors
            for (succ_id, _) in self.block_links[current_id].iter().enumerate().filter(|(_, linked)| { **linked }) {
                if let Some(existing_entry_stack) = self.block_entry_stacks.get(&succ_id) {
                    if existing_entry_stack.len() != exit_stack.len() {
                        logger.error("inconsistent stack depth at block merge point", self.blocks[succ_id].pos().clone());
                    }
                    else if *existing_entry_stack != exit_stack {
                        logger.error("inconsistent stack contents at block merge point", self.blocks[succ_id].pos().clone());
                    }
                }
                else {
                    self.block_entry_stacks.insert(succ_id, exit_stack.clone());
                    todo_ids.push_back(succ_id);
                }
            }
        }
    }
}
impl Display for VirtualProcedure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = format!("Procedure {}\nIn: {:?}\nOut: {:?}", self.name, self.types_in, self.types_out);
        for (b_i, block) in self.blocks.iter().enumerate() {
            s.push_str(&format!("\n  Block {}", b_i));
            s.push_str("\n  Entry stack is ");
            if let Some(es) = self.block_entry_stacks.get(&b_i) {
                s.push_str(&format!("resolved to {:?}", es));
            } 
            else {
                s.push_str(&format!("not resolved (but will be {} long)", block.pops()));
            }
            s.push_str(&format!("\n  Exit stack is {:?}", block.pushes()));
            for statement in self.statements[block.start()..block.start() + block.length()].iter() {
                s.push_str(&format!("\n    {}", statement));
            }
        }
        write!(f, "{}", s)
    }
}

pub struct VRegProcedure {
    name: String,
    inputs: Vec<VReg>,
    outputs: Vec<VReg>,
    instructions: Vec<VRegInstruction>
}
impl VRegProcedure {
    pub fn lower(ir_proc: &VirtualProcedure, sig_table: &HashMap<String, (Vec<DType>, Vec<DType>)>) -> Self {
        // procedures are lowered block by block
        // block input and output registers are propagated in call order
        // because the language uses implicit fallthroughs and we don't insert jump instructions
        // for the implicit fallthroughs, we have to emit blocks in source order
        let mut allocator = VRegAllocator::new();

        // allocate registers for outputs and inputs
        let proc_inputs: Vec<VReg> = ir_proc.types_in().iter().map(
            |slot| allocator.fresh(*slot)
        ).collect();
        let proc_outputs: Vec<VReg> = ir_proc.types_out().iter().map(
            |slot| allocator.fresh(*slot)
        ).collect();

        // register entry stack map, preload 0th block with procedure inputs
        let mut reg_stacks: HashMap<usize, Vec<VReg>> = HashMap::new();
        reg_stacks.insert(0, proc_inputs.to_vec());
        let mut todo_ids: VecDeque<usize> = [0].into();
        let mut visited: HashSet<usize> = HashSet::new();
        let mut block_instrs: HashMap<usize, Vec<VRegInstruction>> = HashMap::new();

        while !todo_ids.is_empty() {
            let current_id = todo_ids.pop_back().unwrap();
            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id);
            
            let mut instructions = vec![];

            // get your entry stack
            // we clone here because we're going to simulate on it, we don't want to change how it
            // looks in the table
            let mut reg_stack = reg_stacks.get(&current_id).unwrap().clone();
            // simulate on the entry stack
            // allocate new registers if you need to
            for s in ir_proc.view_block(current_id).unwrap() {
                match s.payload() {
                    StatementPayload::Push { value } => {
                        let dest = allocator.fresh(value.into());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::LoadImm { dest, val: value.clone() });
                    }
                    StatementPayload::Pop => {
                        reg_stack.pop();
                    }
                    StatementPayload::Dup => {
                        let src = *reg_stack.last().unwrap();
                        reg_stack.push(src);
                    }
                    StatementPayload::Swap => {
                        let a = reg_stack.pop().unwrap();
                        let b = reg_stack.pop().unwrap();
                        reg_stack.push(a);
                        reg_stack.push(b);
                    }
                    // with two-input ops we need to be careful about arg order
                    // remember we want push 1 push 2 sub to be 1 - 2, not 2 - 1
                    // the first thing we pop must be the second argument
                    StatementPayload::Add => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Add { dest, a, b });
                    }
                    StatementPayload::Sub => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Sub { dest, a, b });
                    }
                    StatementPayload::Mult => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Mul { dest, a, b });
                    }
                    StatementPayload::Div => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Div { dest, a, b });
                    }
                    StatementPayload::Mod => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Mod { dest, a, b });
                    }
                    StatementPayload::And => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::And { dest, a, b });
                    }
                    StatementPayload::Or => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Or { dest, a, b });
                    }
                    StatementPayload::Xor => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Xor { dest, a, b });
                    }
                    StatementPayload::Eq => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Eq { dest, a, b });
                    }
                    StatementPayload::Neq => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Neq { dest, a, b });
                    }
                    StatementPayload::Lt => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Lt { dest, a, b });
                    }
                    StatementPayload::Leq => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Leq { dest, a, b });
                    }
                    StatementPayload::Gt => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Gt { dest, a, b });
                    }
                    StatementPayload::Geq => {
                        let b = reg_stack.pop().unwrap();
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Geq { dest, a, b });
                    }
                    StatementPayload::Inc => {
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Inc { dest, a });
                    }
                    StatementPayload::Dec => {
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Dec { dest, a });
                    }
                    StatementPayload::Bsl => {
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Bsl { dest, a });
                    }
                    StatementPayload::Bsr => {
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Bsr { dest, a });
                    }
                    StatementPayload::Rol => {
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Rol { dest, a });
                    }
                    StatementPayload::Ror => {
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Ror { dest, a });
                    }
                    StatementPayload::Not => {
                        let a = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*a.holds());
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Not { dest, a });
                    }
                    StatementPayload::Load { kind } => {
                        let addr = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*kind);
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::LoadMem { dest, addr });
                    }
                    StatementPayload::Store { kind: _ } => {
                        let src = reg_stack.pop().unwrap();
                        let addr = reg_stack.pop().unwrap();
                        instructions.push(VRegInstruction::Store { addr, src });
                    }
                    StatementPayload::Cast { to } => {
                        let src = reg_stack.pop().unwrap();
                        let dest = allocator.fresh(*to);
                        reg_stack.push(dest);
                        instructions.push(VRegInstruction::Cast { dest, src, to: *to });
                    }
                    StatementPayload::Conv { to } => {
                        reg_stack.last_mut().unwrap().change_type(*to);
                    }
                    StatementPayload::Call { dest } => {
                        // get call info from the table
                        let (inputs, outputs) = sig_table.get(dest).unwrap();
                        // pop the input registers from the stack
                        let mut input_regs: Vec<VReg> = vec![];
                        for _ in inputs.iter() {
                            input_regs.push(reg_stack.pop().unwrap());
                        }
                        input_regs.reverse();
                        // allocate output registers to the stack
                        let mut output_regs: Vec<VReg> = vec![];
                        for output in outputs.iter() {
                            let or = allocator.fresh(*output);
                            reg_stack.push(or);
                            output_regs.push(or);
                        }
                        // emit the call
                        instructions.push(VRegInstruction::Call { dest: dest.clone(), inputs: input_regs, outputs: output_regs });
                    }
                    StatementPayload::Jumpif { dest } => {
                        let cmp = reg_stack.pop().unwrap();
                        instructions.push(VRegInstruction::Jumpif { dest: dest.clone(), cmp });
                    }
                    StatementPayload::Jump { dest } => {
                        instructions.push(VRegInstruction::Jump { dest: dest.clone() });
                    }
                    StatementPayload::Label { name } => {
                        instructions.push(VRegInstruction::Label { name: name.clone() });
                    }
                    StatementPayload::Ret => {
                        // move things into the return registers
                        for (slot, from) in proc_outputs.iter().rev().zip(reg_stack.iter().rev()) {
                            instructions.push(VRegInstruction::Move { dest: *slot, src: *from });
                        }
                        // then return from the stack
                        instructions.push(VRegInstruction::Ret);
                    }
                    _ => unimplemented!()
                };
            }

            // reconcile with successive blocks
            for succ_id in ir_proc.succ_ids(current_id).unwrap().iter() {
                // if the next block already has an entry stack allocated, move the values on your
                // stack into their stack
                // order doesn't matter because stacks are the same (guaranteed by typechecking
                // pass)
                if let Some(succ_regs) = reg_stacks.get(succ_id) {
                    assert_eq!(reg_stack.len(), succ_regs.len());
                    for (mine, theirs) in reg_stack.iter().zip(succ_regs.iter()) {
                        assert_eq!(mine.holds(), theirs.holds());
                        instructions.push(VRegInstruction::Move { dest: *theirs, src: *mine });
                    }
                }
                // otherwise just set their entry stack to your exit stack (no need to move or allocate new
                // registers or anything)
                else {
                    reg_stacks.insert(*succ_id, reg_stack.clone());
                }
                todo_ids.push_back(*succ_id);
            }
            
            // save your instructions
            block_instrs.insert(current_id, instructions);
        }

        // now we have to emit instructions in source order
        let mut instructions = vec![];
        // block ids in ascending order is source order
        for block_id in 0..ir_proc.blocks().len() {
            if let Some(instrs) = block_instrs.remove(&block_id) {
                instructions.extend(instrs);
            }
            else {
                panic!()
            }
        }

        VRegProcedure { name: ir_proc.name().clone(), inputs: proc_inputs, outputs: proc_outputs, instructions }
    }
}
impl Display for VRegProcedure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        write!(f, "\n  input registers:")?;
        for ir in self.inputs.iter() {
            write!(f, " {}", ir)?;
        }
        write!(f, "\n  output registers:")?;
        for ir in self.outputs.iter() {
            write!(f, " {}", ir)?;
        }
        for i in self.instructions.iter() {
            write!(f, "\n  {:?}", i)?;
        }
        Ok(())
    }
}
