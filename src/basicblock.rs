use std::collections::{HashSet};

use crate::{statement::DType, util::FilePos};

#[derive(Clone, Debug, PartialEq)]
pub enum TypeStackEntry {
    Unknown,
    Known(DType),
    // index into the block's entry vector
    Depends(usize),
}

pub struct BasicBlock {
    // at which statement in its procedure does this block begin?
    pub start: usize,
    // how many statements are in this block?
    pub length: usize,
    // indices of the blocks that precede this one in its procedure
    pub predecessors: HashSet<usize>,
    // ditto for successors
    pub successors: HashSet<usize>,

    // at the end of the day, blocks are just transformations over a stack of types
    // blocks pop a certain number of types to the stack, and push a certain number of types to the
    // stack. what exactly is popped doesn't matter, because we can pop inside the block and check
    // types at the operation locations. what is pushed matters, because that information will be
    // propagated to subsequent blocks
    pub pops: usize,
    pub pushes: Vec<TypeStackEntry>,

    // operation constraints are resolved during a type checking pass

    pub pos: FilePos,
}

impl BasicBlock {
    pub fn new(start: usize, length: usize, pos: FilePos) -> BasicBlock {
        BasicBlock { start, length, predecessors: HashSet::new(), successors: HashSet::new(), pops: 0, pushes: vec![], pos }
    }
}
