use crate::datastructures::TypeStack;
use crate::parser::datastructures::StackStatement;

/// A BasicBlock is a sequence of statements during which control flow is not interrupted.
#[derive(Debug, Default)]
pub struct BasicBlock {
    /// The index of the first statement belonging to this block
    start: usize,
    /// The number of statements belonging to this block
    length: usize,
    /// The number of types this block expects on the stack at entry
    pops: usize,
    /// The types this block leaves on the stack at exit
    pushes: TypeStack,
}
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
        self.length = length;
    }
    
    pub fn pops(&self) -> usize {
        self.pops
    }
    
    pub fn inc_pops(&mut self) {
        self.pops += 1;
    }
    
    pub fn set_pops(&mut self, pops: usize) {
        self.pops = pops;
    }
    
    pub fn pushes(&self) -> &TypeStack {
        &self.pushes
    }
    
    pub fn set_pushes(&mut self, pushes: TypeStack) {
        self.pushes = pushes;
    }
}