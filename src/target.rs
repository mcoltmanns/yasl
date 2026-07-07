pub mod mos6502;

use crate::datastructures::procedure::LiveRange;
use crate::datastructures::procedure::VRegProcedure;
use crate::datastructures::program::VRegProgram;
use crate::regmachine::VReg;
use std::collections::HashMap;
use crate::datastructures::statement::DType;
use std::fmt::Debug;

// a target represents a baremetal cpu
pub trait Target {
    // a location represents a place that you can store data
    // a register, a memory address, a stack offset, etc
    type Location: Clone + PartialEq + Debug;

    fn init() -> Self;

    // allocate n contiguous locations according to whatever this target's location strategy is
    // should use preferred locations first, then resort to spillage locations
    fn alloc(&mut self, needed: usize) -> Result<Vec<Self::Location>, String>;
    // free locations back to their regions
    fn free(&mut self, locs: Vec<Self::Location>);

    // how many bits is a pointer?
    fn pointer_width(&self) -> u8;
    // how many locations do we need to store a value of a type?
    fn locs_needed(dtype: DType) -> usize;

    // given a program, emit for the target
    // returns none if ok, some(error) if something went wrong
    fn emit(program: &VRegProgram) -> Result<Vec<u8>, String>;
}

// run linear allocation on a procedure
// this uses the target's allocation strategy
// returns a map of virtual registers to locations on the target
// these locations are local to the procedure and assume clean registers at entry and a clean data
// frame
pub fn lin_alloc<T: Target>(vproc: &VRegProcedure) -> Result<HashMap<VReg, Vec<T::Location>>, String> {
    let mut alloc_map: HashMap<VReg, Vec<T::Location>> = HashMap::new();
    let mut target = T::init();

    let mut active: Vec<&LiveRange> = vec![];
    let mut ranges_increasing = vproc.live_ranges().clone();
    ranges_increasing.sort_by_key(|&r| r.start());
    for i in ranges_increasing {
        // expire old intervals
        active.sort_by_key(|&r| r.start() + r.length());
        active.retain(
            |&j| {
                // if overlap, keep active
                if j.start() + j.length() >= i.start() {
                    return true;
                }
                // otherwise free the register allocated to this range and drop
                target.free(alloc_map.get(j.register()).unwrap().to_vec());
                false
            }
        );
        // in the normal algorithm, we spill if the number of active ranges is >= the number of
        // registers available
        // because the target decides where to spill based on what is free, all we have to do
        // is allocate for this range, add that to the map, and continue
        // find out how many locations we need for this virtual register
        let needed = T::locs_needed(*i.register().holds());
        alloc_map.insert(*i.register(), target.alloc(needed)?);
        active.push(i);
    }

    Ok(alloc_map)
}
