use crate::datastructures::TypeStack;
use crate::datastructures::TypeStackEntry;

#[derive(Debug)]
pub struct BasicBlock {
    start: usize,
    length: usize,

    pops: usize,
    pushes: TypeStack,
}

// Basic blocks don't need a position field, because their position depends on the position of their first statement
impl BasicBlock {
    pub fn new(start: usize, length: usize) -> Self {
        BasicBlock { start, length, pops: 0, pushes: TypeStack::new() }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn set_length(&mut self, length: usize) {
        self.length = length
    }

    pub fn pops(&self) -> usize {
        self.pops
    }

    pub fn inc_pops(&mut self) {
        self.pops += 1
    }

    pub fn pushes(&self) -> &TypeStack {
        &self.pushes
    }

    pub fn set_pushes(&mut self, entry: Vec<TypeStackEntry>) {
        self.pushes = entry;
    }
}
