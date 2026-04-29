use crate::statement::DType;

#[derive(Clone, Debug)]
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
    pub predecessors: Vec<usize>,
    // ditto for successors
    pub successors: Vec<usize>,

    // typing information
    // a block needs certain types on the stack at entry in order to function
    // and will leave certain types at exit
    // these are the entry and exit type vectors
    pub entry_stack: Vec<TypeStackEntry>,
    pub exit_stack: Vec<TypeStackEntry>,
    // the constraints array is built during block-local analysis
    // each element represents a pair of types which must be equal when resolved
    // they are resolved according to the entry vector
    pub constraints: Vec<(TypeStackEntry, TypeStackEntry)>
}

impl BasicBlock {
    pub fn new(start: usize, length: usize) -> BasicBlock {
        BasicBlock { start, length, predecessors: vec![], successors: vec![], entry_stack: vec![], exit_stack: vec![], constraints: vec![] }
    }
}
