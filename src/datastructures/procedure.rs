use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fmt::Display;

use crate::datastructures::TypeStackEntry;
use crate::datastructures::basicblock::BasicBlock;
use crate::datastructures::statement::DType;
use crate::datastructures::statement::StatementPayload;
use crate::datastructures::statement::Statement;
use crate::datastructures::TypeStack;
use crate::logger::Logger;
use crate::util::FilePos;
use crate::util::Positionable;

#[derive(Debug)]
pub struct Procedure {
    name: String,
    types_in: Vec<DType>,
    types_out: Vec<DType>,

    pos: FilePos,

    jump_table: HashMap<String, usize>,
    blocks: Vec<BasicBlock>,
    // (from, to)
    block_links: Vec<Vec<bool>>,
    statements: Vec<Statement>,
    block_entry_stacks: HashMap<usize, Vec<TypeStackEntry>>,
}

impl Procedure {
    pub fn empty(name: String, types_in: Vec<DType>, types_out: Vec<DType>, pos: FilePos) -> Self {
        Procedure { name, types_in, types_out, pos, jump_table: HashMap::new(), blocks: vec![], block_links: vec![], statements: vec![], block_entry_stacks: HashMap::new() }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn blocks(&self) -> &[BasicBlock] {
        &self.blocks
    }

    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    pub fn set_statements(&mut self, statements: Vec<Statement>) {
        self.statements = statements
    }

    pub fn types_in(&self) -> &[DType] {
        &self.types_in
    }

    pub fn types_out(&self) -> &[DType] {
        &self.types_out
    }

    pub fn view_block(&self, block_i: usize) -> Option<&[Statement]> {
        let block = self.blocks().get(block_i)?;
        Some(&self.statements()[block.start()..block.start() + block.length()])
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
                        tracking.push(TypeStackEntry::Known(kind.clone()));
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
                        tracking.push(TypeStackEntry::Known(to.clone()));
                    }
                    // call info is in the proc table
                    StatementPayload::Call { dest } => {
                        if let Some((inputs, outputs)) = sig_table.get(dest) {
                            // iteration order doesn't matter here, because we're only counting
                            for _ in inputs.iter() {
                                pop(&mut tracking, block);
                            }
                            for output in outputs.iter() {
                                tracking.push(TypeStackEntry::Known(output.clone()));
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
        self.block_entry_stacks.insert(0, self.types_in.iter().map(|t| TypeStackEntry::Known(t.clone())).collect());

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
            let mut sim_stack = entry_stack.to_vec();
            for s in self.statements[current.start()..current.start() + current.length()].iter() {
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
                        if matches!(a, TypeStackEntry::Unknown) {
                            logger.error("type resolution failed", s.pos().clone());
                        }
                        sim_stack.push(a);
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
                        sim_stack.push(TypeStackEntry::Known(kind.clone()));
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
                        sim_stack.push(TypeStackEntry::Known(to.clone()));
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
                                sim_stack.push(TypeStackEntry::Known(output.clone()));
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

impl Display for Procedure {
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
