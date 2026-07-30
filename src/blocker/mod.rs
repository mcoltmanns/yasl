use std::collections::HashMap;
use crate::blocker::datastructures::BasicBlock;
use crate::logger::Logger;
use crate::parser::datastructures::{StackProcedure, StackProgram, StackStatement};
use crate::util::Positioned;

/// The blocker operates on StackPrograms.
/// For each procedure, it generates the BasicBlocks (statement sequences during which control flow is not tranferred).
/// It also builds the jump table within procedures.

mod datastructures;

pub fn run(program: &mut StackProgram, logger: &mut dyn Logger) -> Result<(), ()> {
    let mut blocker = Blocker { logger };
    // block entry
    blocker.block_procedure(&mut program.entry)?;
    // block optional interrupt handlers
    if let Some(hwi) = program.hwi.as_mut() {
        blocker.block_procedure(hwi)?;
    }
    if let Some(nmi) = program.nmi.as_mut() {
        blocker.block_procedure(nmi)?;
    }
    for swi_proc in program.swi_table.iter_mut() {
        if let Some(swi) = swi_proc {
            blocker.block_procedure(swi)?;
        }
    }
    for proc in program.proc_table.values_mut() {
        blocker.block_procedure(proc)?;
    }
    Ok(())
}

struct Blocker<'s> {
    //program: StackProgram,
    logger: &'s mut dyn Logger
}
impl<'s> Blocker<'s> {
    /// Build the basic blocks and jump table for a procedure
    fn block_procedure(&mut self, proc: &mut Positioned<StackProcedure>) -> Result<(), ()> {
        let mut blocks = vec![];
        let mut jump_table: HashMap<String, usize> = HashMap::new();
        let mut current_block: Option<BasicBlock> = None;
        for (si, s) in proc.statements.iter().enumerate() {
            match &**s {
                // labels start a new block and end the last one, and are included in the block they start
                StackStatement::Label { name } => {
                    if let Some(mut last_block) = current_block.take() {
                        last_block.set_length(si - last_block.start());
                        blocks.push(last_block);
                    }
                    current_block = Some(BasicBlock::new(si, 0));
                    // labels are added to jump table
                    if jump_table.insert(name.clone(), blocks.len()).is_some() {
                        self.logger.error("duplicate label", s.pos().clone());
                        // TODO should a duplicate label be grounds for a fatal failure?
                        //.TODO it definitely needs to be logged, but do we need to return err here?
                        return Err(());
                    }
                }
                // control flow changes start a new block and end the last one, and are included in the block they end
                StackStatement::Jump { .. }
                | StackStatement::Jumpif { .. }
                | StackStatement::Ret { .. } => {
                    if let Some(mut last_block) = current_block.take() {
                        last_block.set_length(si + 1 - last_block.start());
                        blocks.push(last_block);
                    }
                    else {
                        // case where there's a lone control flow change
                        blocks.push(BasicBlock::new(si, 1));
                    }
                }
                // anything else starts a new block if we aren't already on one
                _ => {
                    if current_block.is_none() {
                        current_block = Some(BasicBlock::new(si, 0));
                    }
                }
            }
            // close the final block
            if let Some(mut last_block) = current_block.take() {
                last_block.set_length(proc.statements.len() - last_block.start());
                blocks.push(last_block);
            }
        }
        Ok(())
    }
}