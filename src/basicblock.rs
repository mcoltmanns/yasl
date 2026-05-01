use std::collections::{HashSet, VecDeque};

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

    // typing information
    // a block needs certain types on the stack at entry in order to function
    // and will leave certain types at exit
    // these are the entry and exit type vectors
    pub entry_stack: Vec<TypeStackEntry>,
    pub exit_stack: Vec<TypeStackEntry>,
    // the constraints array is built during block-local analysis
    // each element represents a pair of types which must be equal when resolved
    // they are resolved according to the entry vector
    pub const_equal: Vec<(TypeStackEntry, TypeStackEntry, FilePos)>,
    // the integer check array is also built like the constraint array, but it is for the special
    // jumpif case
    // all the entries in here must be integer types when resolved
    pub const_int: Vec<(TypeStackEntry, FilePos)>,

    pub pos: FilePos,
}

impl BasicBlock {
    pub fn new(start: usize, length: usize, pos: FilePos) -> BasicBlock {
        BasicBlock { start, length, predecessors: HashSet::new(), successors: HashSet::new(), entry_stack: vec![], exit_stack: vec![], const_equal: vec![], const_int: vec![], pos }
    }
}
