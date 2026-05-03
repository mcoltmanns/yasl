use crate::util::Positionable;
use crate::util::FilePos;

#[derive(Debug)]
pub struct BasicBlock {
    start: usize,
    length: usize,
    pos: FilePos
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
        BasicBlock { start, length, pos }
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
}
