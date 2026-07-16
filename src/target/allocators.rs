use std::collections::HashMap;
use crate::datastructures::procedure::{LiveRange, VRegProcedure};
use crate::regmachine::VReg;
use crate::target::Target;

// run linear allocation on a procedure
// this uses the target's allocation strategy
// returns a map of virtual registers to locations on the target
// these locations are local to the procedure and assume clean registers at entry and a clean data
// frame
pub fn lin_alloc<T: Target>(vproc: &VRegProcedure) -> Result<HashMap<VReg, Vec<T::Location>>, String> {
    let mut alloc_map: HashMap<VReg, Vec<T::Location>> = HashMap::new();
    let mut target = T::init_for_alloc();

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
                target.free_locs(alloc_map.get(j.register()).unwrap().to_vec());
                false
            }
        );
        // in the normal algorithm, we spill if the number of active ranges is >= the number of
        // registers available
        // because the target decides where to spill based on what is free, all we have to do
        // is allocate for this range, add that to the map, and continue
        // find out how many locations we need for this virtual register
        let needed = T::locs_needed(*i.register().holds());
        // then allocate those locations, if possible, and insert them to the map
        alloc_map.insert(*i.register(), target.alloc_n_locs(needed)?);
        // refresh the live range
        active.push(i);
    }

    Ok(alloc_map)
}
