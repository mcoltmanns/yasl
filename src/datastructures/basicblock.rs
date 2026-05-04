use crate::datastructures::TypeStack;
use crate::datastructures::TypeStackEntry;
use crate::util::Positionable;
use crate::util::FilePos;

#[derive(Debug)]
pub struct BasicBlock {
    start: usize,
    length: usize,
    pos: FilePos,

    pops: usize,
    pushes: TypeStack,
}
impl Positionable for BasicBlock {
    fn pos(&self) -> &FilePos {
        &self.pos
    }
    fn line(&self) -> usize {
        self.pos.line
    }
    fn col(&self) -> usize {
        self.pos.col
    }
}
impl BasicBlock {
    pub fn new(start: usize, length: usize, pos: FilePos) -> Self {
        BasicBlock { start, length, pos, pops: 0, pushes: vec![] }
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
