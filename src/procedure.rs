use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::thread::current;
use crate::basicblock::TypeStackEntry;
use crate::statement::DType;
use crate::statement::LiteralValue;
use crate::statement::Statement;
use crate::statement::StatementKind;
use crate::logger::Logger;
use crate::basicblock::BasicBlock;
use crate::util::FilePos;

pub type ProcedureTable = HashMap<String, Procedure>;
pub type SignatureTable = HashMap<String, (Vec<DType>, Vec<DType>)>;

pub struct Procedure {
    name: String,
    pos: FilePos,
    inputs: Vec<DType>,
    outputs: Vec<DType>,
    blocks: Vec<BasicBlock>,
    statements: Vec<Statement>,
    // local table for jump labels
    jump_table: HashMap<String, usize>,
    calls: HashSet<String>,
    called_by: HashSet<String>,
}

impl Procedure {
    pub fn new(name: String, pos: FilePos, inputs: Vec<DType>, outputs: Vec<DType>, statements: Vec<Statement>) -> Procedure {
        Procedure { name, pos, inputs, outputs, blocks: vec![], statements, jump_table: HashMap::new(), calls: HashSet::new(), called_by: HashSet::new() }
    }

    pub fn reachable(&self) -> bool {
        // you are reachable if you are not unreachable
        // and you are unreachable if your name is not main and your predecessor set contains only
        // your name
        let unreachable = self.name != "main" && self.called_by.get(&self.name).is_some_and(|_| self.called_by.len() <= 1);
        !unreachable
    }

    pub fn pos(&self) -> &FilePos {
        &self.pos
    }

    pub fn get_blocks(&self) -> &Vec<BasicBlock> {
        &self.blocks
    }

    pub fn get_jump(&self, label: &String) -> Option<&usize> {
        self.jump_table.get(label)
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn get_intypes(&self) -> &Vec<DType> {
        &self.inputs
    }

    pub fn get_outtypes(&self) -> &Vec<DType> {
        &self.outputs
    }

    pub fn set_statements(&mut self, sts: Vec<Statement>) {
        self.statements = sts
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn compute_block_pushes_and_pops(&mut self, b_i: usize, sig_table: &SignatureTable, logger: &mut dyn Logger) {
        // this simulates types on the procedure stack for a given block (by index)
        // figure out what we need at least on the stack when we arrive at this block, and what we
        // will leave, and how type relations flow through the block (how outs depend on ins)
        let b = &mut self.blocks[b_i];
        let statements = &self.statements[b.start..b.start+b.length];
        // tracking the stack within this block
        let mut tracking: Vec<TypeStackEntry> = vec![];

        // pops from the tracking stack
        // if there was something there, returns that
        // if there wasn't, increment our required pops and return an entry that depends on that pop
        // position
        fn pop(tracking: &mut Vec<TypeStackEntry>, pops: &mut usize) -> TypeStackEntry {
            tracking.pop().unwrap_or_else(|| -> TypeStackEntry {
                *pops += 1;
                TypeStackEntry::Depends(*pops - 1)
            })
        }

        // simulate the tracking stack through the statements, allocate more space on the entry
        // stack if necessary
        for s in statements {
            println!("{:?}", tracking);
            match s.kind() {
                // pushes only add to the type stack
                StatementKind::Push { value } => {
                    match value.value() {
                        LiteralValue::Pointer(..) => tracking.push(TypeStackEntry::Known(DType::Pointer)),
                        LiteralValue::I8(..) => tracking.push(TypeStackEntry::Known(DType::I8)),
                        LiteralValue::I16(..) => tracking.push(TypeStackEntry::Known(DType::I16)),
                        LiteralValue::I32(..) => tracking.push(TypeStackEntry::Known(DType::I32)),
                        LiteralValue::I64(..) => tracking.push(TypeStackEntry::Known(DType::I64)),
                        LiteralValue::U8(..) => tracking.push(TypeStackEntry::Known(DType::U8)),
                        LiteralValue::U16(..) => tracking.push(TypeStackEntry::Known(DType::U16)),
                        LiteralValue::U32(..) => tracking.push(TypeStackEntry::Known(DType::U32)),
                        LiteralValue::U64(..) => tracking.push(TypeStackEntry::Known(DType::U64)),
                        LiteralValue::F16(..) => tracking.push(TypeStackEntry::Known(DType::F16)),
                        LiteralValue::F32(..) => tracking.push(TypeStackEntry::Known(DType::F32)),
                        LiteralValue::F64(..) => tracking.push(TypeStackEntry::Known(DType::F64)),
                    };
                }
                // pops only discard, so no need to push the result of pop from tracking here
                StatementKind::Pop => {
                    pop(&mut tracking, &mut b.pops);
                }
                // dup just duplicates the type at the top of the stack
                StatementKind::Dup => {
                    let top = pop(&mut tracking, &mut b.pops);
                    tracking.push(top.clone());
                    tracking.push(top);
                }
                StatementKind::Swap => {
                    let a = pop(&mut tracking, &mut b.pops);
                    let b = pop(&mut tracking, &mut b.pops);
                    tracking.push(a.clone());
                    tracking.push(b.clone());
                }
                // two-arg operators
                // they all require both inputs be the same, and return their input type
                // for logic operators this means you might need to cast if you want your output to
                // be useful for a jump
                StatementKind::Add | StatementKind::Sub | StatementKind::Mult | StatementKind::Div | StatementKind::Mod | StatementKind::And | StatementKind::Or | StatementKind::Xor | StatementKind::Eq | StatementKind::Neq | StatementKind::Lt | StatementKind::Leq | StatementKind::Gt | StatementKind::Geq => {
                    let a = pop(&mut tracking, &mut b.pops);
                    let b = pop(&mut tracking, &mut b.pops);
                    // try to resolve the output type
                    // if a or b are known, use that
                    // otherwise depend on whatever the first arg depended on
                    let result = match (&a, &b) {
                        (TypeStackEntry::Known(t), _) => TypeStackEntry::Known(t.clone()),
                        (_, TypeStackEntry::Known(t)) => TypeStackEntry::Known(t.clone()),
                        _ => a.clone(),
                    };
                    tracking.push(result);
                }
                // increment and decrement just require anything and return that thing
                StatementKind::Inc | StatementKind::Dec | StatementKind::Bsl | StatementKind::Bsr | StatementKind::Rol | StatementKind::Ror | StatementKind::Not => {
                    let a = pop(&mut tracking, &mut b.pops);
                    tracking.push(a.clone());
                }
                // load requires a pointer and leaves its kind
                StatementKind::Load { kind } => {
                    // the pointer must be a pointer
                    pop(&mut tracking, &mut b.pops);
                    tracking.push(TypeStackEntry::Known(kind.clone()));
                }
                // store has 2 args: thing to store, and pointer to store it at (stack: dest val
                // (top))
                StatementKind::Store { .. } => {
                    pop(&mut tracking, &mut b.pops);
                    pop(&mut tracking, &mut b.pops);
                }
                // cast and conv require one thing and leave the thing they cast or convert to
                StatementKind::Cast { to } | StatementKind::Conv { to } => {
                    pop(&mut tracking, &mut b.pops);
                    tracking.push(TypeStackEntry::Known(to.clone()));
                }
                // call info can be looked up in procedure table
                StatementKind::Call { dest } => {
                    if let Some((inputs, outputs)) = sig_table.get(dest) {
                        // remember stack tops are the ends of vectors
                        // but here it fully does not matter what direction you iterate
                        for _ in inputs.iter() {
                            pop(&mut tracking, &mut b.pops);
                        }
                        // again stack tops are the ends of vectors
                        for output in outputs.iter() {
                            tracking.push(TypeStackEntry::Known(output.clone()))
                        }
                    }
                    else {
                        logger.error(format!("cannot find signature for unknown procedure \"{}\"", dest), s.pos().line, s.pos().col);
                        return;
                    }
                }
                // jumpif requires an integer
                // we check jump destinations later (during block linkage)
                StatementKind::Jumpif { .. } => {
                    pop(&mut tracking, &mut b.pops);
                }
                // if we arrive at a return statement, we expect the output types of this procedure
                // to be on the stack
                // doing it this way has the consequence that blocks which end in rec always have
                // no output (but that's ok because nothing comes after them)
                StatementKind::Ret => {
                    // check that all of the types we want to return are available on the
                    // procedure stack
                    // remember top of stack is at back (but again no matter)
                    for _output in self.outputs.iter() {
                        pop(&mut tracking, &mut b.pops);
                    }
                }
                // none of the other things have an effect on the local data stack
                StatementKind::Label { .. } | StatementKind::Jump { .. } | StatementKind::Proc { .. } => {}
            };
        }

        b.pushes = tracking;
    }

    pub fn resolve_types(&mut self, logger: &mut dyn Logger) {
        // resolve types throughout the procedure
        // applies blocks as transformations
        // right now we know how many things the blocks take from the global stack, and what they
        // push to the global stack (sometime what they push depends on what they see on the input)
        // here you need to simulate the global stack and go through, propagate
        // also here you need to check types within the blocks at each statement (or do that in a
        // later procedure, once you know all the types)
        // you will need to track full entry stacks, there's no way around it
        if self.blocks.is_empty() {
            logger.error("undefined procedure".to_string(), self.pos.line, self.pos.col);
            return;
        }
        let mut todo_ids: VecDeque<usize> = vec![0].into();
        let mut visited: HashSet<usize> = HashSet::new();

        // entry stack map, preload 0th block with procedure inputs
        let mut entry_stacks: HashMap<usize, Vec<TypeStackEntry>> = HashMap::new();
        entry_stacks.insert(0, self.inputs.iter().map(|t| TypeStackEntry::Known(t.clone())).collect());

        while !todo_ids.is_empty() {
            let current_id = todo_ids.pop_back().unwrap();
            let current = &mut self.blocks[current_id];
            visited.insert(current_id);

            // if this block has no successors, it has to end in a return statement
            // return statements are always at the ends of blocks by definition
            if current.successors.is_empty() {
                let last = &self.statements[current.start + current.length - 1];
                if !matches!(last.kind(), StatementKind::Ret) {
                    logger.error("procedure has no valid path to return".to_string(), current.pos.line, current.pos.col);
                }
            }

            // get your entry stack
            // unwrap is safe because we can only visit a block if its entry stack has been
            // populated
            let entry_stack = entry_stacks.get(&current_id).unwrap();

            println!("{}{} stack is {:?}", self.name, current_id, entry_stack);

            // check the head of the stack against the expected stack size
            if entry_stack.len() < current.pops {
                logger.error("stack underflow in procedure".to_string(), current.pos.line, current.pos.col);
                continue;
            }
            // compute the exit stack
            // remember young is right
            // so the exit stack before pushes is the entry stack without the last <pops> elements
            let mut exit_stack: Vec<TypeStackEntry> = entry_stack[..entry_stack.len() - current.pops].to_vec();
            for push in current.pushes.iter() {
                match push {
                    TypeStackEntry::Known(_) => exit_stack.push(push.clone()),
                    TypeStackEntry::Unknown => {
                        logger.error("unable to resolve type".to_string(), current.pos.line, current.pos.col);
                    }
                    TypeStackEntry::Depends(i) => {
                        // in this case we depend on a value in the entry
                        // the popped region(region we use) of the entry is entry[entry.len() - pops..]
                        // i indexes into the popped region
                        if let Some(t) = entry_stack.get(entry_stack.len() - current.pops + i) {
                            exit_stack.push(t.clone());
                        }
                        else {
                            panic!("invalid type dependency index");
                        }
                    }
                }
            }
            println!("get {:?}", entry_stack);
            println!("leave {:?}", exit_stack);
            
            // then we can propagate the outputs to the successors
            for succ_id in current.successors.iter() {
                // if you've already seen this successor (they already have an entry stack
                // propagated from somewhere else)
                // check consistency with what you got this time around
                if let Some(existing_entry_stack) = entry_stacks.get(succ_id) {
                    if *existing_entry_stack != exit_stack {
                        logger.error("unable to reconcile types at block merge point".to_string(), current.pos.line, current.pos.col);
                    }
                }
                else {
                    entry_stacks.insert(*succ_id, exit_stack.clone());
                    if !todo_ids.contains(succ_id) {
                        todo_ids.push_back(*succ_id);
                    }
                }
            }
        }

        // this bit works, now entry and exit stacks are fully resolved
        // probably now the thing would be to simulate block walkthroughs one last time and check
        // types as you go

        // now all the entry stacks are fully resolved
        // we can check constraints and resolve exit stacks
        /*for current in &mut self.blocks {
            // first step is to check block constraints
            // first do equality constraints
            for (a, b, pos) in current.const_equal.iter() {
                // first resolve the constraint if it is unresolved and its target is known
                let resolved_a = match a {
                    TypeStackEntry::Depends(i) => current.entry_stack[*i].clone(),
                    other => other.clone()
                };
                let resolved_b = match b {
                    TypeStackEntry::Depends(i) => current.entry_stack[*i].clone(),
                    other => other.clone()
                };
                // now if the constraint is fully known, we can validate
                if let TypeStackEntry::Known(conc_a) = resolved_a && let TypeStackEntry::Known(conc_b) = resolved_b {
                    if conc_a != conc_b {
                        logger.error(format!("type mismatch ({:?} != {:?})", conc_a, conc_b), pos.line, pos.col);
                    }
                }
                // if the constraint isn't fully known at this point it's an error
                else {
                    logger.error("type resolution failed".to_string(), pos.line, pos.col);
                }
                // we don't really need to store resolved constraints
            }
            // then do integer constraints
            for (a, pos) in current.const_int.iter() {
                let resolved_a = match a {
                    TypeStackEntry::Depends(i) => current.entry_stack[*i].clone(),
                    other => other.clone()
                };
                match resolved_a {
                    TypeStackEntry::Known(t) if t.is_integer() => {}
                    TypeStackEntry::Known(t) => {
                        logger.error(format!("conditional jumps can only operate on integer types, got {:?}", t), pos.line, pos.col);
                    }
                    _ => {
                        logger.error("type resolution failed_".to_string(), pos.line, pos.col);
                    }
                }
            }
        }*/
    }

    pub fn build_jumps_and_blocks(&mut self, logger: &mut dyn Logger) {
        // this is the first procedural definition pass
        // it builds the jump table and blocks out the statements into basic blocks
        // it also sets the predecessor and successor lists for each block
        // first we have to build the basic blocks, since the jump table maps label names to
        // basic block vector indices
        let mut current_block: Option<BasicBlock> = None;

        for (i, s) in self.statements.iter().enumerate() {
            match s.kind() {
                StatementKind::Label { name } => {
                    // labels terminate the last block and start a new one
                    // labels are always at the start of a block
                    // if we were already working on a block, terminate it
                    if let Some(mut last_block) = current_block.take() {
                        last_block.length = i - last_block.start;
                        self.blocks.push(last_block);
                    }
                    // and start a new one
                    current_block = Some(BasicBlock::new(i, 0, s.pos().clone()));
                    // also because this is a label, create an entry in the jump table
                    // if the entry existed, log an error
                    if self.jump_table.insert(name.clone(), self.blocks.len()).is_some() {
                        logger.error(format!("duplicate label \"{}\" in procedure \"{}\"", name, self.name), s.pos().line, s.pos().col);
                    }
                }
                StatementKind::Jump { dest: _ }
                | StatementKind::Jumpif { dest: _ }
                | StatementKind::Ret => {
                    // jumps and rets terminate the current block
                    // jumps and rets are always at the end of the block they terminate, so the
                    // indexing is a little different
                    // if we were already working on a block, terminate it and include this
                    // statement
                    // leave starting the next block open
                    if let Some(mut last_block) = current_block.take() {
                        last_block.length = (i + 1) - last_block.start;
                        self.blocks.push(last_block);
                    }
                    // if there was no previous block, create a block containing only this
                    // statement
                    else {
                        self.blocks.push(BasicBlock::new(i, 1, s.pos().clone()));
                    }
                }
                _ => {
                    // anything else starts a new block if we aren't already working on one
                    if current_block.is_none() {
                        current_block = Some(BasicBlock::new(i, 0, s.pos().clone()));
                    }
                }
            };
        }

        // when we're done, if we're still working on a block, close it
        // a procedure ending in an open block is an error, but we will catch this later
        if let Some(mut last_block) = current_block.take() {
            last_block.length = self.statements.len() - last_block.start;
            self.blocks.push(last_block);
        }
    }

    pub fn link_blocks(&mut self, logger: &mut dyn Logger) {
        // once we know the jump table we can link the blocks together by setting their
        // predecessors/successors
        // just go block by block and see where the labels go
        // we have do this after jump table definition pass because labels are use before define
        // to keep the borrow checker from complaining, first collect edges then apply them to the
        // lists
        let mut edges: Vec<(usize, usize)> = vec![];
        for (b_id, b) in self.blocks.iter().enumerate() {
            // only the last statement in a block determines successors
            let s = &self.statements[b.start + b.length - 1];
            match s.kind() {
                // jumps are succeeded by the block they call
                StatementKind::Jump { dest } => {
                    if let Some(successor_id) = self.get_jump(dest) {
                        edges.push((b_id, *successor_id));
                    }
                    else {
                        logger.error(format!("label \"{}\" undefined in procedure \"{}\"", dest, self.name), s.pos().line, s.pos().col);
                    }
                }
                // jumpifs are succeeded by the block they call, or the block after them
                // (fallthrough)
                StatementKind::Jumpif { dest } => {
                    if let Some(successor_id) = self.get_jump(dest) {
                        edges.push((b_id, *successor_id));
                    }
                    else {
                        logger.error(format!("label \"{}\" undefined in procedure \"{}\"", dest, self.name), s.pos().line, s.pos().col);
                    }
                    // if there are blocks left, everything ok. otherwise throw err
                    let next_id = b_id + 1;
                    if next_id < self.blocks.len() {
                        edges.push((b_id, next_id));
                    }
                    else {
                        logger.error("no fallthrough block after conditional jump".to_string(), s.pos().line, s.pos().col);
                    }
                }
                // rets have no successors
                StatementKind::Ret => {}
                // anything else is succeeded by the block after it
                _ => {
                    // if there are blocks left, everything ok
                    // otherwise you might be missing a return instruction, but this can only be
                    // checked conclusively during propagation
                    // so we don't throw an error yet
                    let next_id = b_id + 1;
                    if next_id < self.blocks.len() {
                        edges.push((b_id, next_id));
                    }
                }
            }
        }

        for (from, to) in edges {
            self.blocks[from].successors.insert(to);
            self.blocks[to].predecessors.insert(from);
        }

        // check for unreachable blocks
        // the first block is always reachable, so skip it
        for (b_id, b) in self.blocks.iter().enumerate().skip(1) {
            // you are unreachable if you have no predecessors or if you are your only predecessor
            if b.predecessors.is_empty() || (b.predecessors.len() == 1 && b.predecessors.contains(&b_id)){
                let pos = self.statements[b.start].pos();
                logger.warning("unreachable code".to_string(), pos.line, pos.col);
            }
        } 
    }
}

