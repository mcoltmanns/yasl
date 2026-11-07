use crate::logger::Logger;
use crate::parser::datastructures::{StackProcedure, StackProgram, StackStatement};
use crate::util::Positioned;

/// The flowchecker performs control flow analysis.
/// This can occur at the procedure level (flow between basicblocks) or at the program level (flow between procedures)
/// In both cases, the flowchecker builds a linkage graph between units (blocks or procedures) and then uses this graph to search for unreachable units.
/// Unreachable units emit a warning and are dropped from the program. (TODO implement dropping them)
///
/// The flowchecker should be run after the blocker! (It needs the jump table to be built)
pub fn run(program: &mut StackProgram, logger: &mut dyn Logger) -> Result<(), ()> {
    let mut checker = FlowChecker { logger };
    // first check procedures
    checker.check_proc(&mut program.entry)?;
    if let Some(hwi) = program.hwi.as_mut() {
        checker.check_proc(hwi)?;
    }
    if let Some(nmi) = program.nmi.as_mut() {
        checker.check_proc(nmi)?;
    }
    for swi_proc in program.swi_table.iter_mut() {
        if let Some(swi) = swi_proc {
            checker.check_proc(swi)?;
        }
    }
    for proc in program.proc_table.values_mut() {
        checker.check_proc(proc)?;
    }
    // then check the whole program
    checker.check_program(program)?;
    Ok(())
}

struct FlowChecker<'l> {
    logger: &'l mut dyn Logger,
}
impl<'l> FlowChecker<'l> {
    pub fn check_proc(&mut self, proc: &mut Positioned<StackProcedure>) -> Result<(), ()> {
        let mut block_links = vec![vec![false; proc.blocks.len()]; proc.blocks.len()];
        for b_i in 0..proc.blocks.len() {
            let block = &proc.blocks[b_i];
            // last statement in a block determines successors
            // get the slice of statements that is this block
            let block_statements = &proc.statements[block.start()..block.start() + block.length()];
            let last_statement = block_statements.last().expect("empty block should not be possible");
            match &**last_statement {
                // jumps are succeeded by the block they call
                StackStatement::Jump { dest } => {
                    if let Some(succ_id) = proc.jump_table.get(dest) {
                        block_links[b_i][*succ_id] = true;
                    }
                    else {
                        self.logger.error("invalid jump destination", last_statement.pos().clone());
                        return Err(()); // again TODO should this be fatal?
                    }
                }
                // jumpifs are succeeded by the block they call, and the block after them
                StackStatement::Jumpif { dest } => {
                    if let Some(succ_id) = proc.jump_table.get(dest) {
                        block_links[b_i][*succ_id] = true;
                    }
                    else {
                        self.logger.error("invalid jump destination", last_statement.pos().clone());
                        return Err(());
                    }
                    let next_id = b_i + 1;
                    if next_id < proc.blocks.len() {
                        block_links[b_i][next_id] = true;
                    }
                    else {
                        self.logger.error("no fallthrough after conditional jump", last_statement.pos().clone());
                        return Err(());
                    }
                }
                // rets have no successors
                StackStatement::Ret { .. } => {}
                // anything else is succeeded by the block after it, if there is one
                _ => {
                    let next_id = b_i + 1;
                    if next_id < proc.blocks.len() {
                        block_links[b_i][next_id] = true;
                    }
                }
            }
        }
        // save block links
        proc.block_links = block_links;
        // check block reachability
        // emit warnings for dead blocks
        // first block is always reachable
        for b_i in 1..proc.blocks.len() {
            let mut reachable = false;
            // you are reachable if there is a link from any other block to you
            for other_i in 0..proc.blocks.len() {
                if b_i == other_i { continue; } // don't count self-links
                reachable |= proc.block_links[other_i][b_i];
            }
            if !reachable {
                self.logger.warning(&format!("unreachable code ({} statements)", proc.blocks[b_i].length()), proc.statements[proc.blocks[b_i].start()].pos().clone());
                // TODO implement dropping dead blocks - this will involve index recalculation for all blocks after the dropped one in source
                //  and reassignment of proc.block_links
            }
        }
        Ok(())
    }

    pub fn check_program(&mut self, proc: &mut StackProgram) -> Result<(), ()> {
        // similar concept to block reachability check, but we do procedures
        // for any procedure, if any other procedure calls it, it is reachable
        // the main procedures and interrupt handlers are always reachable
        // so the only ones that need to be checked are procedures in proc_table
        // not entirely true - the user may write procedures that are only ever called in the context of handling interrupts

        // best way to do this?
        // keep a stack of reachable procedures - init with main and all interrupts
        // keep a mask of procedures which have been visited
        // pop a procedure, mark it visited
        // for every call site in the procedure, update proc_links

        // TODO implement this after you do the proc id refactor
        unimplemented!()
    }
}
