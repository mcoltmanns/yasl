use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
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

    pub fn simulate_block_types(&mut self, b_i: usize, sig_table: &SignatureTable, logger: &mut dyn Logger) {
        // this simulates types on the procedure stack for this block
        // cannot fully resolve, that is done during propagation
        let b = &mut self.blocks[b_i];
        let statements = &self.statements[b.start..b.start+b.length];
        let mut tracking: Vec<TypeStackEntry> = vec![];
        let mut entry: Vec<TypeStackEntry> = vec![];
        // the constraints array
        // each element represents a pair of types which must be equal when resolution is complete
        // as resolution progresses, these things can be resolved as the entry vector fills in
        let mut constraints: Vec<(TypeStackEntry, TypeStackEntry, FilePos)> = vec![];
        let mut int_constraints: Vec<(TypeStackEntry, FilePos)> = vec![];

        // pops from the tracking stack
        // if there was something there, returns that
        // if there wasn't, allocate an empty entry slot and return a dependency entry that refers to
        // the newly allocated entry slot
        fn pop_from_tracking(tracking: &mut Vec<TypeStackEntry>, entry: &mut Vec<TypeStackEntry>) -> TypeStackEntry {
            tracking.pop().unwrap_or_else(|| -> TypeStackEntry {
                let slot = entry.len();
                // technically the order we push here is wrong
                // because the last thing to call this wants a thing from the bottom of the entry
                // stack, not the back
                // but that would mean we need to update the constraint indices each time
                // so just remember that this particular stack and the constraints are backwards
                entry.push(TypeStackEntry::Unknown);
                TypeStackEntry::Depends(slot)
            })
        }

        for s in statements {
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
                    pop_from_tracking(&mut tracking, &mut entry);
                }
                // dup just duplicates the type at the top of the stack
                StatementKind::Dup => {
                    let top = pop_from_tracking(&mut tracking, &mut entry);
                    tracking.push(top.clone());
                    tracking.push(top);
                }
                StatementKind::Swap => {
                    let a = pop_from_tracking(&mut tracking, &mut entry);
                    let b = pop_from_tracking(&mut tracking, &mut entry);
                    tracking.push(a);
                    tracking.push(b);
                }
                // two-arg operators
                // they all require both inputs be the same, and return their input type
                // for logic operators this means you might need to cast if you want your output to
                // be useful for a jump
                StatementKind::Add | StatementKind::Sub | StatementKind::Mult | StatementKind::Div | StatementKind::Mod | StatementKind::And | StatementKind::Or | StatementKind::Xor | StatementKind::Eq | StatementKind::Neq | StatementKind::Lt | StatementKind::Leq | StatementKind::Gt | StatementKind::Geq => {
                    let a = pop_from_tracking(&mut tracking, &mut entry);
                    let b = pop_from_tracking(&mut tracking, &mut entry);
                    // add to the constraint array
                    constraints.push((a.clone(), b.clone(), s.pos().clone()));
                    // try to resolve the output type
                    // if a or b are known, use that
                    // otherwise depend on whatever the first arg depended on
                    let result = match (&a, &b) {
                        (TypeStackEntry::Known(t), _) => TypeStackEntry::Known(t.clone()),
                        (_, TypeStackEntry::Known(t)) => TypeStackEntry::Known(t.clone()),
                        _ => a,
                    };
                    tracking.push(result);
                }
                // increment and decrement just require anything and return that thing
                StatementKind::Inc | StatementKind::Dec | StatementKind::Bsl | StatementKind::Bsr | StatementKind::Rol | StatementKind::Ror | StatementKind::Not => {
                    let a = pop_from_tracking(&mut tracking, &mut entry);
                    tracking.push(a.clone());
                }
                // load requires a pointer and leaves its kind
                StatementKind::Load { kind } => {
                    // the pointer must be a pointer
                    let ptr = pop_from_tracking(&mut tracking, &mut entry);
                    constraints.push((ptr, TypeStackEntry::Known(DType::Pointer), s.pos().clone()));
                    tracking.push(TypeStackEntry::Known(kind.clone()));
                }
                // store has 2 args: thing to store, and pointer to store it at (stack: dest val
                // (top))
                StatementKind::Store { kind } => {
                    let thing = pop_from_tracking(&mut tracking, &mut entry);
                    let ptr = pop_from_tracking(&mut tracking, &mut entry);
                    // thing must be kind, and ptr must be a pointer
                    constraints.push((thing, TypeStackEntry::Known(kind.clone()), s.pos().clone()));
                    constraints.push((ptr, TypeStackEntry::Known(DType::Pointer), s.pos().clone()));
                }
                // cast and conv require one thing and leave the thing they cast or convert to
                StatementKind::Cast { to } | StatementKind::Conv { to } => {
                    pop_from_tracking(&mut tracking, &mut entry);
                    tracking.push(TypeStackEntry::Known(to.clone()));
                }
                // call info can be looked up in procedure table
                StatementKind::Call { dest } => {
                    if let Some((inputs, outputs)) = sig_table.get(dest) {
                        // remember stack tops are the ends of vectors
                        // so iterate backwards
                        for input in inputs.iter().rev() {
                            let top = pop_from_tracking(&mut tracking, &mut entry);
                            constraints.push((top, TypeStackEntry::Known(input.clone()), s.pos().clone()));
                        }
                        // again stack tops are the ends of vectors
                        // but since we're pushing iterate forwards
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
                    let cond = pop_from_tracking(&mut tracking, &mut entry);
                    int_constraints.push((cond, s.pos().clone()));
                }
                // if we arrive at a return statement, we expect the output types of this procedure
                // to be on the stack
                // doing it this way has the consequence that blocks which end in rec always have
                // no output
                StatementKind::Ret => {
                    // check that all of the types we want to return are available on the
                    // procedure stack
                    // remember top of stack is at back
                    for output in self.outputs.iter().rev() {
                        let top = pop_from_tracking(&mut tracking, &mut entry);
                        constraints.push((top, TypeStackEntry::Known(output.clone()), s.pos().clone()));
                    }
                    // because this is the return, we also expect that the tracking stack is
                    // empty
                    if !tracking.is_empty() {
                        logger.error(format!("procedure returns with {} extra value{} on stack", tracking.len(), if tracking.len() > 1 { "s" } else { "" }), s.pos().line, s.pos().col);
                    }
                }
                // none of the other things have an effect on the local data stack
                StatementKind::Label { .. } | StatementKind::Jump { .. } | StatementKind::Proc { .. } => {}
            };
        }

        b.entry_stack = entry;
        b.exit_stack = tracking;
        b.const_equal = constraints;
        b.const_int = int_constraints;
    }

    pub fn resolve_types(&mut self, logger: &mut dyn Logger) {
        // the first block is always the entry block
        if self.blocks.is_empty() {
            logger.error("undefined procedure".to_string(), self.pos.line, self.pos.col);
            return;
        }
        let entry_block = &mut self.blocks[0];
        if entry_block.entry_stack.len() != self.inputs.len() {
            logger.error(format!("procedure consumes {} argument{}, but signature declares {}", entry_block.entry_stack.len(), if entry_block.entry_stack.len() == 1 { "s" } else { "" }, self.inputs.len()), self.pos.line, self.pos.col);
            return;
        }
        // we have to iterate over one of these stacks backwards because entry_stack is backwards
        // see definition of pop_from_tracking for explanation why
        for (slot, input) in entry_block.entry_stack.iter_mut().zip(self.inputs.iter().rev()) {
            *slot = TypeStackEntry::Known(input.clone());
        }

        // now that the entry block's inputs are known and at least somewhat correct, we can add it
        // to the worklist and propagate input types through the blocks in the procedure
        // the worklist is also a convenient place to make sure we have a reachable return
        // statement, since it is guaranteed to visit every block reachable from the entry
        // point
        let mut todo_ids: VecDeque<usize> = vec![0].into();
        let mut visited: HashSet<usize> = HashSet::new();

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

            // first we resolve what outputs we can
            let exit_stack: Vec<TypeStackEntry> = current.exit_stack.iter().map(
                |output| -> TypeStackEntry {
                    match output {
                        TypeStackEntry::Depends(ev_i) => current.entry_stack[*ev_i].clone(),
                        other => other.clone(),
                    }
                }
            ).collect();

            // then we can propagate the outputs to the successors
            let successors: Vec<usize> = current.successors.iter().cloned().collect();
            for succ_id in successors {
                let succ = &mut self.blocks[succ_id];
                let mut changed = false;
                // if the successor has the wrong number of outputs, throw
                // and go to the next one bc partial propagation doesn't help anyone
                if succ.entry_stack.len() != exit_stack.len() {
                    logger.error(format!("stack depth mismatch at block boundary (expected {}, got {})", succ.entry_stack.len(), exit_stack.len()), succ.pos.line, succ.pos.col);
                    continue;
                }
                // propagate the resolved exit stack (incoming) for this block into the entry stack of the
                // next block (slot)
                for (slot, incoming) in succ.entry_stack.iter_mut().zip(exit_stack.iter()) {
                    match incoming {
                        // there should be no dependencies in the temporary exit stack, since they
                        // were resolved
                        TypeStackEntry::Depends(_) => panic!("unresolved dependencies in exit stack"),
                        // unknowns should not be propagated
                        TypeStackEntry::Unknown => {}
                        // known values should be propagated
                        TypeStackEntry::Known(_) => {
                            // if the slot is unknown, then we propagate and remember there's more
                            // work to do
                            if *slot == TypeStackEntry::Unknown {
                                *slot = incoming.clone();
                                changed = true;
                            }
                            // but if the slot is known, and the types aren't right, it's an error
                            else if slot != incoming {
                                logger.error(format!("type mismatch ({:?} != {:?})", slot, incoming), succ.pos.line, succ.pos.col);
                            }
                        }
                    }
                }

                // only visit successors if you changed something or haven't seen them yet, and if
                // they're not already slated for reprocessing
                if (changed || !visited.contains(&succ_id)) && !todo_ids.contains(&succ_id) {
                    todo_ids.push_back(succ_id);
                }
            }
        }

        // now all the entry stacks are fully resolved
        // we can check constraints and resolve exit stacks
        for current in &mut self.blocks {
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
                        logger.error("type resolution failed".to_string(), pos.line, pos.col);
                    }
                }
            }

            // then we can resolve the outputs from the inputs, and write them
            current.exit_stack = current.exit_stack.iter().map(
                |output| -> TypeStackEntry {
                    match output {
                        TypeStackEntry::Depends(ev_i) => current.entry_stack[*ev_i].clone(),
                        other => other.clone(),
                    }
                }
            ).collect();
        }
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

