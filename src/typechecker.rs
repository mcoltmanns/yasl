use crate::primitives::DType;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum TypeStackEntry {
    Known(DType),
    Depends(usize),
    Unknown,
}

#[derive(Debug, Default)]
pub struct TypeStack {
    stack: Vec<TypeStackEntry>,
    draws: usize, // how many unknown values does this stack draw on?
}
impl TypeStack {
    /// Initialize an empty stack
    pub fn new() -> TypeStack {
        TypeStack { stack: Vec::new(), draws: 0 }
    }

    /// Initialize a stack from a vec of dtypes
    pub fn from_vec(from: Vec<DType>) -> TypeStack {
        TypeStack { stack: from.iter().map(
            |dt| {
                TypeStackEntry::Known((*dt).into())
            }
        ).collect() , draws: 0 }
    }

    /// Push a real type to the stack
    pub fn push(&mut self, thing: DType) {
        self.stack.push(TypeStackEntry::Known(thing.into()))
    }

    /// Simulates popping from this stack.
    /// If a value is available, return that.
    /// If there isn't, increment the draw count for this stack and return an entry that depends on that draw
    pub fn pop(&mut self) -> TypeStackEntry {
        self.stack.pop().unwrap_or_else(
            || -> TypeStackEntry {
                self.draws += 1;
                TypeStackEntry::Depends(self.draws - 1)
            }
        )
    }

    /// Get the number of types this stack draws on
    pub fn get_draws(&self) -> usize {
        self.draws
    }
}

struct Typechecker<'t> {
    stack_program: &'t mut StackProgram,
    logger: &'t mut dyn Logger,
}
impl <'t> Typechecker<'t> {
    /// Compute the
    fn block_effects(&mut self, block: &BasicBlock) -> Result<(), ()> {
        block.

        Ok(())
    }
}
