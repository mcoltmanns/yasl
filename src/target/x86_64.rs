use crate::datastructures::program::{VRegProgram, VirtualProgram};
use crate::datastructures::statement::DType;
use crate::logger::Logger;
use crate::target::{AllocMap, Target, TrapSet};

pub struct X86_64Target {
}
impl X86_64Target {
}
impl Target for X86_64Target {
    type Location = u64; // TODO placeholder!
    fn init_for_alloc() -> Self {
        todo!()
    }
    fn alloc_n_locs(&mut self, needed: usize) -> Result<Vec<Self::Location>, String> {
        todo!()
    }
    fn free_locs(&mut self, locs: Vec<Self::Location>) {
        todo!()
    }
    fn locs_needed(dtype: DType) -> usize {
        todo!()
    }
    fn build_alloc_map(vreg_program: &VRegProgram, logger: &mut dyn Logger) -> AllocMap<Self::Location>
    where
        Self: Sized,
    {
        todo!()
    }
    fn build_trap_set(virtual_program: &VRegProgram, alloc_map: &AllocMap<Self::Location>, max_locs: usize, logger: &mut dyn Logger) -> TrapSet<Self::Location> {
        todo!()
    }
    fn emit(program: &VRegProgram, global_allocs: &AllocMap<Self::Location>, trap_set: &TrapSet<Self::Location>, logger: &mut dyn Logger) -> Vec<u8> {
        todo!()
    }
}