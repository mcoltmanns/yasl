use crate::target::VReg;
use std::collections::HashMap;
use crate::datastructures::statement::VRegInstruction;
use crate::target::DType;
use crate::target::Target;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MOS6502Location {
    RegisterPool(u8),
    FrameSpill(u16),
}
pub struct MOS6502Target {
    // $0000-$00FF is zero page, fast access
    // $0100-$01FF is hardware stack, reserved for the processor's call/return stack
    // $FFFA-$FFFF are interrupt and reset vectors, also reserved
    //
    // the first 64 bytes ($0000-$0039) are the zero page register pool
    // $0040-$0041 is DFP (data frame pointer)
    // $0042-$0043 is IND (indirect load pointer)
    // $0044-$004A is SCR0 (64 bit math scratch)
    // $004B-$0052 is SCR1 (64 bit math scratch)
    // $0000-$0052 are all compiler reserved
    // $0053-$00ff is free zero page
    // $0100-$01ff is the hardware stack
    // $0200-$05ff is the data frame stack (grows down)
    //
    // tying in with hardware specs:
    // $0000-$5fff is RAM
    // $6000-$7fff is devices
    // $8000-$FFFF is ROM
    //
    // in order to keep a stack overflow from overwriting data in memory, the data frame stack
    // should grow down towards the hardware stack. that way if something does happen, at least
    // you're not guaranteed to mess up other parts of the system
    // also there are things to consider with the data stack shape. what if it crosses page
    // boundaries? how much do you want to allocate? data stack frames hold parameters and
    // returns, and also spills. but with 64B of virtual registers we are unlikely to need many
    // spills (for reasonably sized values at least)
    // a 64 bit value takes 8 bytes. one page is 256 bytes. 4 pages? 1MB, plenty of room for
    // expansion
    // so that puts data frame stack in $0200-$05ff
    //
    // for instruction emission we need to know one more thing: where the program should be placed
    // in memory. if the user does not specify, assume $8000.

    // free registers are a u64 bitmap where 1 is free, 0 is used
    free_registers: u64,
    // vector of spill blocks which have been used and freed
    free_spill: Vec<(u16, u16)>,
    // where the next spill will take place
    next_spill: u16,
}
impl MOS6502Target {
    // low byte of the DFP
    const DFP_LO: u16 = 0x0040;
    // low byte of the indirection pointer
    const IND_LO: u16 = 0x0042;
    // low bytes of the two scratch registers
    const SCR0_LO: u16 = 0x0044;
    const SCR1_LO: u16 = 0x004B;

    const PROG_START_DEFAULT: u16 = 0x8000;
}
impl Target for MOS6502Target {
    type Location = MOS6502Location;

    fn init() -> Self {
        MOS6502Target { 
            free_registers: 0xffffffffffffffff,
            free_spill: vec![],
            next_spill: 0
        }
    }

    fn pointer_width(&self) -> u8 {
        16
    }

    fn locs_needed(dtype: DType) -> usize {
        match dtype {
            DType::I8 | DType::U8 => 1,
            DType::I16 | DType::U16 | DType::F16 | DType::Pointer => 2,
            DType::I32 | DType::U32 | DType::F32 => 4,
            DType::I64 | DType::U64 | DType::F64 => 8,
        }
    }

    fn alloc(&mut self, needed: usize) -> Vec<Self::Location> {
        if needed == 0 {
            return vec![];
        }

        let needed = needed as u16;

        // try to find space in the registers
        for start in 0..=(64 - needed) {
            let mask = ((1u64 << needed) - 1) << start;

            if self.free_registers & mask == mask {
                self.free_registers &= !mask;

                return (start..start + needed).map(|i| MOS6502Location::RegisterPool(i as u8)).collect();
            }
        }

        // if that didn't work, allocate a spill
        // first try to reuse spill blocks that have been freed
        for i in 0..self.free_spill.len() {
            let (start, len) = self.free_spill[i];
            // fits perfectly?
            if len == needed {
                // unfree
                self.free_spill.remove(i);
                // and return all the locations in that block
                return (start..start + needed).map(MOS6502Location::FrameSpill).collect();
            }
            // fits too large?
            if len > needed {
                // push the block's start up by the amount of space we need
                self.free_spill[i].0 += needed;
                // and shorten the block by that amount too
                self.free_spill[i].1 -= needed;
                // and return all the locations in the block before modification
                return (start..start + needed).map(MOS6502Location::FrameSpill).collect();
            }
        }
        // if we couldn't find any blocks to reuse, allocate more spill space
        let start = self.next_spill;
        self.next_spill += needed;
        (start..start + needed).map(MOS6502Location::FrameSpill).collect()
    }

    fn emit(program: &crate::datastructures::program::VRegProgram) {
        // emit a program
        // emit more or less in source order
        // we want to emit binary, not assembly

        // first we set the 
    }

    fn free(&mut self, locs: Vec<Self::Location>) {
        // free all the locations depending on what they are
        for loc in locs.iter() {
            match loc {
                MOS6502Location::RegisterPool(i) => {
                    let mask = ((1u64 << 1) - 1) << i;
                    self.free_registers |= mask;
                }
                MOS6502Location::FrameSpill(i) => {
                    self.free_spill.push((*i, 1))
                }
            }
        }
        // because this method of freeing results in a bunch of tiny free blocks, we have to
        // coalesce free blocks afterwards
        self.free_spill.sort_by_key(|block| block.0);
        let mut merged: Vec<(u16, u16)> = Vec::new();
        for block in &self.free_spill {
            // look at the last block we merged. if it ends where we start, extend it by our length
            if let Some(last) = merged.last_mut() && last.0 + last.1 == block.0 {
                last.1 += block.1;
                continue;
            }
            // otherwise add to the merged list
            merged.push(*block);
        }
        self.free_spill = merged;
    }
}

