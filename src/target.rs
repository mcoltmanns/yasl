pub mod mos6502;
pub mod x86_64;
mod allocators;

use crate::datastructures::program::{VRegProgram, VirtualProgram};
use crate::regmachine::VReg;
use std::collections::{HashMap, HashSet};
use crate::datastructures::statement::DType;
use std::fmt::Debug;
use std::hash::Hash;
use crate::logger::Logger;
use crate::util::{FilePos, Positionable};

/// map of procedure names to their maps of virtual registers to target locations
type AllocMap<L> = HashMap<String, HashMap<VReg, Vec<L>>>;
/// set of locations used by all procedures involved in interrupt handling
type TrapSet<L> = HashSet<L>;
// a target represents a baremetal cpu
pub trait Target {
    // a location represents a place that you can store data
    // a register, a memory address, a stack offset, etc
    type Location: Clone + Copy + Eq + PartialEq + Debug + Hash ;

    /// set this target up for allocation
    ///
    /// mark all locations as free
    ///
    /// this is not a general initialization function! this should only be called in the context of allocation.
    ///
    /// target use pipeline is build_alloc_map -> build_trap_set -> emit
    fn init_for_alloc() -> Self;
    /// allocate n contiguous locations according to whatever this target's location strategy is
    /// should use preferred locations first, then resort to spillage locations
    /// use Target::locs_needed() to find out how many locations a given dtype needs
    fn alloc_n_locs(&mut self, needed: usize) -> Result<Vec<Self::Location>, String>;
    /// mark locations as free
    fn free_locs(&mut self, locs: Vec<Self::Location>);
    /// lookup how many locations are needed to store a value of a given type
    fn locs_needed(dtype: DType) -> usize;

    /// build the global allocation map for a program
    fn build_alloc_map(vreg_program: &VRegProgram, logger: &mut dyn Logger) -> AllocMap<Self::Location> where Self: Sized {
        let mut allocation_map = AllocMap::new();
        for (proc_name, vreg_proc) in vreg_program.proc_table() {
            let a = allocators::lin_alloc::<Self>(vreg_proc);
            if let Ok(allocation) = a {
                allocation_map.insert(proc_name.clone(), allocation);
            } else if let Err(e) = a {
                logger.error(&*e, FilePos { name: vreg_program.name().clone(), line: vreg_proc.line(), col: vreg_proc.col() })
            }
        }
        allocation_map
    }

    /// build the set of locations that the interrupt handler uses
    fn build_trap_set(virtual_program: &VRegProgram, alloc_map: &AllocMap<Self::Location>, max_locs: usize, logger: &mut dyn Logger) -> TrapSet<Self::Location> {
        // the trap set
        let mut trap_locations = TrapSet::new();
        if !alloc_map.contains_key("trapper") {
            return trap_locations;
        }
        // set of procedure names we've seen
        let mut visited: HashSet<String> = HashSet::new();
        // vector of procedures to visit next, at first just trap
        let mut next = vec!["trapper".to_string()];
        while let Some(this_name) = next.pop() {
            // seen this procedure already? (recursion) skip it
            if visited.contains(&this_name) {
                continue;
            }
            // mark this procedure as visited
            visited.insert(this_name.clone());
            // get the locations this procedure uses
            let these_locations = &alloc_map[&this_name];
            // insert leach location into the trap location set
            for (_, locations) in these_locations {
                locations.iter().for_each(|loc| { trap_locations.insert(*loc); });
            }
            // for each procedure name that comes after this one in the call graph, add to the processing list
            virtual_program.call_graph()[&this_name].iter().for_each(|next_name| { next.push(next_name.clone()) })
        }
        // if the trap set is greater than some threshold, warn
        if trap_locations.len() > max_locs {
            logger.warning(&format!("interrupt handler uses over {} real locations (context switch may be slow)", max_locs), FilePos { name: virtual_program.name().clone(), line: 0, col: 0})
        }
        trap_locations
    }

    // given a program, emit (a byte vector) for the target
    // returns none if ok, some(error) if something went wrong
    fn emit(program: &VRegProgram, global_allocs: &AllocMap<Self::Location>, trap_set: &TrapSet<Self::Location>, logger: &mut dyn Logger) -> Vec<u8>;
}
