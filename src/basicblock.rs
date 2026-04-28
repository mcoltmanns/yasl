use crate::{procedure::Procedure, statement::Statement};

pub struct BasicBlock {
    // at which statement in its procedure does this block begin?
    pub start: usize,
    // how many statements are in this block?
    pub length: usize,
    // indices of the blocks that precede this one in its procedure
    pub predecessors: Vec<usize>,
    // ditto for successors
    pub successors: Vec<usize>,
}
