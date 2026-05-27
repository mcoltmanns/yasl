// the register machine is used to lower the program from its typechecked ir format to a register
// machine form
// it emits VirtualInstructions

use std::fmt::Display;
use std::hash::Hash;

use crate::datastructures::statement::DType;

#[derive(Clone, Copy, Debug, Eq)]
pub struct VReg {
    id: usize,
    holds: DType,
}
impl VReg {
    pub fn new(id: usize, holds: DType) -> Self {
        VReg { id, holds }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn holds(&self) -> &DType {
        &self.holds
    }

    pub fn change_type(&mut self, nt: DType) {
        self.holds = nt
    }
}
impl Display for VReg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r{}({:?})", self.id, self.holds)
    }
}
impl PartialEq for VReg {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Hash for VReg {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Default)]
pub struct VRegAllocator {
    next: usize,
}
impl VRegAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fresh(&mut self, kind: DType) -> VReg {
        let out = VReg::new(self.next, kind);
        self.next += 1;
        out
    }
}
