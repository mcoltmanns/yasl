/**
* macros for codegen for the 6502
*/

use crate::target::mos6502::{MOS6502Instruction, MOS6502Location, MOS6502Target};

/// store a literal byte at the given location
pub fn store_byte(location: &MOS6502Location, byte: u8) -> Vec<u8> {
    let mut out = vec![];
    // load the byte
    out.append(&mut MOS6502Instruction::LDA(byte).to_bytes());
    // now look at where we want to write
    match location {
        // framespill is an index off of the dfp
        MOS6502Location::FrameSpill(i) => {
            // we can index directly through the y register (indirect indexed/indirect y)
            // a zero page address (the dfp) points to a base address, to which the y register is added
            // load the offset into the y register
            out.append(&mut MOS6502Instruction::LDY(*i).to_bytes());
            // then store a at the dfp plus the y register
            out.append(&mut MOS6502Instruction::STAY(MOS6502Target::DFP_LO).to_bytes());
            println!("\t\twrite {:02X} to {:02X} plus DFP", byte, *i);
        }
        // registerpool is a direct location in zero page
        MOS6502Location::RegisterPool(i) => {
            // store a zero page
            out.append(&mut MOS6502Instruction::STAZ(*i).to_bytes());
            println!("\t\twrite {:02X} to {:02X}", byte, *i);
        }
    }
    out
}

/// load the accumulator with a byte from a given location
pub fn load_acc(location: &MOS6502Location) -> Vec<u8> {
    let mut out = vec![];
    match location {
        MOS6502Location::FrameSpill(i) => {
            // if we're loading from frame spill, it's an indirect y read
            out.append(&mut MOS6502Instruction::LDY(*i).to_bytes());
            out.append(&mut MOS6502Instruction::LDAY(MOS6502Target::DFP_LO).to_bytes());
        }
        MOS6502Location::RegisterPool(i) => {
            // if we're loading from the register pool, it's a zero page read
            out.append(&mut MOS6502Instruction::LDAZ(*i).to_bytes());
        }
    }
    out
}

/// store the byte in the accumulator to a given location
pub fn store_acc(location: &MOS6502Location) -> Vec<u8> {
    let mut out = vec![];
    match location {
        MOS6502Location::FrameSpill(i) => {
            // if we're storing to frame spill, it's an indirect y write
            out.append(&mut MOS6502Instruction::LDY(*i).to_bytes());
            out.append(&mut MOS6502Instruction::STAY(MOS6502Target::DFP_LO).to_bytes());
        }
        MOS6502Location::RegisterPool(i) => {
            // if we're writing to the register pool, it's a zero page write
            out.append(&mut MOS6502Instruction::STAZ(*i).to_bytes());
        }
    }
    out
}

/// dest = a + b, for whole types
/// considers and sets carry bit
/// you should probably always clear the carry bit before you use this (unless you're doing multi-byte math
pub fn add_whole(dest: &MOS6502Location, a: &MOS6502Location, b: &MOS6502Location) -> Vec<u8> {
    let mut out = vec![];
    // the first thing to do is load the accumulator with the first operand
    out.append(&mut load_acc(a));
    // then depending on where b is, select the right load+add combo
    match b {
        MOS6502Location::FrameSpill(i) => {
            // if it's a frame spill, index through y
            out.append(&mut MOS6502Instruction::LDY(*i).to_bytes());
            out.append(&mut MOS6502Instruction::ADCY(MOS6502Target::DFP_LO).to_bytes());
        }
        MOS6502Location::RegisterPool(i) => {
            // if it's not, we can work right in zp
            out.append(&mut MOS6502Instruction::ADCZ(*i).to_bytes());
        }
    }
    // now the result is in the accumulator, so write it
    out.append(&mut store_acc(dest));
    out
}

/// dest = a - b, for whole types
/// considers and sets carry bit
/// you should probably always SET the carry bit before you use this (unless you're doing multi-byte math)
pub fn sub_whole(dest: &MOS6502Location, a: &MOS6502Location, b: &MOS6502Location) -> Vec<u8> {
    let mut out = vec![];
    // the first thing to do is load the accumulator with the first operand
    out.append(&mut load_acc(a));
    // then depending on where b is, select the right load+sub combo
    match b {
        MOS6502Location::FrameSpill(i) => {
            // if it's a frame spill, index through y
            out.append(&mut MOS6502Instruction::LDY(*i).to_bytes());
            out.append(&mut MOS6502Instruction::SBCY(MOS6502Target::DFP_LO).to_bytes());
        }
        MOS6502Location::RegisterPool(i) => {
            // if it's not, we can work right in zp
            out.append(&mut MOS6502Instruction::SBCZ(*i).to_bytes());
        }
    }
    // now the result is in the accumulator, so write it
    out.append(&mut store_acc(dest));
    out
}

/// unpack a float of arbitrary width into one of the float scratch registers
pub fn unpack_float(from: &[MOS6502Location], reg: usize) -> Vec<u8> {
    let mut out = vec![];
    // decide where we're gonna unpack
    let sign_dest;
    let exp_dest;
    let man_dest;
    match reg {
        0 => {
            sign_dest = MOS6502Target::SIG0;
            exp_dest = MOS6502Target::EXP0_LO;
            man_dest = MOS6502Target::MAN0_LO;
        }
        1 => {
            sign_dest = MOS6502Target::SIG1;
            exp_dest = MOS6502Target::EXP1_LO;
            man_dest = MOS6502Target::MAN1_LO;
        }
        2 => {
            sign_dest = MOS6502Target::SIG2;
            exp_dest = MOS6502Target::EXP2_LO;
            man_dest = MOS6502Target::MAN2_LO;
        }
        _ => {
            panic!("unsupported reg {}", reg);
        }
    }
    // then unpack
    // we can figure the width of the float from the number of locations passed
    match from.len() {
        2 => {
            // float16
            // remember byte order is LE
            let bottom = &from[0];
            let top = &from[1];
            // top byte holds: sign / 5x exponent / 2x fraction
            // bottom byte holds 8x fraction (big endian)
            // in order to take apart the top byte, we use a series of masks
            // first load the top byte into the x register, this might save us a few cycles if it's in spill memory
            out.append(&mut load_acc(top));
            out.append(&mut MOS6502Instruction::TAX.to_bytes());
            // mask out the sign bit into the accumulator
            let sign_mask: u8 = 0b1000_0000;
            out.append(&mut MOS6502Instruction::AND(sign_mask).to_bytes());
            // don't need to align because we can just do a zero or bit test
            out.append(&mut MOS6502Instruction::STAZ(sign_dest).to_bytes());
            // mask out the exponent into the accumulator
            let exp_mask: u8 = 0b0111_1100;
            out.append(&mut MOS6502Instruction::TXA.to_bytes());
            out.append(&mut MOS6502Instruction::AND(exp_mask).to_bytes());
            // align to bottom of byte with two right shifts
            out.append(&mut MOS6502Instruction::LSR.to_bytes());
            out.append(&mut MOS6502Instruction::LSR.to_bytes());
            // save
            out.append(&mut MOS6502Instruction::STAZ(exp_dest).to_bytes());
            // same idea for the bit of fraction
            let frac_mask: u8 = 0b0000_0011;
            out.append(&mut MOS6502Instruction::TXA.to_bytes());
            out.append(&mut MOS6502Instruction::AND(frac_mask).to_bytes());
            // don't need to align because the bits are already at the bottom of the byte
            // but we place it at frac_dest + 1 because this is the high part of the fraction
            out.append(&mut MOS6502Instruction::STAZ(man_dest + 1).to_bytes());
            // and then the low part of the fraction
            out.append(&mut load_acc(bottom));
            out.append(&mut MOS6502Instruction::STAZ(man_dest).to_bytes());
        }
        4 => {
            // float32
            // remember le, sig bytes are at back
            let top_byte = &from[3];
            let second_byte = &from[2];
            // top contains sign and most of the exponent
            // second contains 1 bit of the exponent and 7 of the mantissa
            let sign_mask: u8 = 0b1000_0000;
            let top_exp_mask: u8 = !sign_mask;
            let bottom_exp_mask: u8 = 0b1000_0000;
            let bottom_man_mask: u8 = !bottom_exp_mask;
            // same idea as with f16, first load top byte into x
            out.append(&mut load_acc(top_byte));
            out.append(&mut MOS6502Instruction::TAX.to_bytes());
            // mask out the sign bit
            out.append(&mut MOS6502Instruction::AND(sign_mask).to_bytes());
            out.append(&mut MOS6502Instruction::STAZ(sign_dest).to_bytes());
            // mask out the top part of the exponent (7 bits)
            out.append(&mut MOS6502Instruction::TXA.to_bytes());
            out.append(&mut MOS6502Instruction::AND(top_exp_mask).to_bytes());
            // align to top of byte with one left shift
            out.append(&mut MOS6502Instruction::ASL.to_bytes());
            // and save
            out.append(&mut MOS6502Instruction::STAZ(exp_dest).to_bytes());
            // then load the second byte
            out.append(&mut load_acc(second_byte));
            out.append(&mut MOS6502Instruction::TAX.to_bytes());
            // mask out the bottom bit of the exponent (it's at the top of the byte though)
            out.append(&mut MOS6502Instruction::AND(bottom_exp_mask).to_bytes());
            // rotate left twice, that puts it in the right place
            // have to clear carry so we don't contaminate the masked result
            out.append(&mut MOS6502Instruction::CLC.to_bytes());
            out.append(&mut MOS6502Instruction::ROL.to_bytes());
            out.append(&mut MOS6502Instruction::ROL.to_bytes());
            // now the bit is where it's supposed to be
            // OR it with the rest of the exponent
            out.append(&mut MOS6502Instruction::ORAZ(exp_dest).to_bytes());
            // now the complete exponent is in the accumulator - write it
            out.append(&mut MOS6502Instruction::STAZ(exp_dest).to_bytes());
            // still have to grab the highest bit of the mantissa - first reload the byte
            out.append(&mut MOS6502Instruction::TXA.to_bytes());
            out.append(&mut MOS6502Instruction::AND(bottom_man_mask).to_bytes());
            // now the bottom 7 bits of the accumulator contain the top 7 bits of the mantissa
            // lucky for us, this is also byte-aligned, so we can directly write the accumulator to man_dest + 2
            out.append(&mut MOS6502Instruction::STAZ(man_dest + 2).to_bytes());
            // and now we just load/store the other two bytes of the mantissa
            out.append(&mut load_acc(&from[1]));
            out.append(&mut MOS6502Instruction::STAZ(man_dest + 1).to_bytes());
            out.append(&mut load_acc(&from[0]));
            out.append(&mut MOS6502Instruction::STAZ(man_dest).to_bytes());
        }
        8 => {
            // float 64
            // remember le, sig bytes are at back
            // 8th byte is (msb) sign, 7 msb of exponent (lsb)
            // 7th byte is (msb) 4 lsb of exponent, 4 msb of mantissa (lsb)
            let top_byte = &from[7];
            let second_byte = &from[6];
            let sign_mask: u8 = 0b1000_0000;
            let top_exp_mask: u8 = !sign_mask;
            let bottom_exp_mask: u8 = 0b1111_0000;
            let bottom_man_mask: u8 = !bottom_exp_mask;
            // like before, first load top byte into x
            out.append(&mut load_acc(top_byte));
            out.append(&mut MOS6502Instruction::TAX.to_bytes());
            // mask out sign bit
            out.append(&mut MOS6502Instruction::AND(sign_mask).to_bytes());
            out.append(&mut MOS6502Instruction::STAZ(sign_dest).to_bytes());
            // mask out top part of exponent (7 bits)
            out.append(&mut MOS6502Instruction::TXA.to_bytes());
            out.append(&mut MOS6502Instruction::AND(top_exp_mask).to_bytes());
            // this one is annoying - the byte alignment on the exponent is poor
            // the exponent is 11 bits wide total, so the top 3 bits have to go to their own byte (low 3 bits of exp_dest + 1)
            // we're already using x, luckily we still have y and won't need to load anything through there for a while (until we grab the next byte from memory)
            // so save the whole top part of the exponent to y
            out.append(&mut MOS6502Instruction::TAY.to_bytes());
            // now we can extract the top 3 bits (of the top part), shift them to the bottom of the byte, write
            out.append(&mut MOS6502Instruction::AND(top_exp_mask & 0b0111_0000).to_bytes());
            // shift to bottom (4 shifts is not nice but it's quicker than rotating because that has to go through the carry bit)
            for _ in 0..4 {
                out.append(&mut MOS6502Instruction::LSR.to_bytes());
            }
            // write
            out.append(&mut MOS6502Instruction::STAZ(exp_dest + 1).to_bytes());
            // then the next 4 bits - reload from y first
            out.append(&mut MOS6502Instruction::TYA.to_bytes());
            // extract bottom 4
            out.append(&mut MOS6502Instruction::AND(top_exp_mask & 0b0000_1111).to_bytes());
            // shift them left 4x (into the top of the byte)
            for _ in 0..4 {
                out.append(&mut MOS6502Instruction::ASL.to_bytes());
            }
            // write them to memory
            out.append(&mut MOS6502Instruction::STAZ(exp_dest).to_bytes());
            // now we just have to grab the last 4 bytes of the exponent
            // we don't need the first byte anymore, so we can load the second into x
            out.append(&mut load_acc(second_byte));
            out.append(&mut MOS6502Instruction::TAX.to_bytes());
            // mask out the bottom nibble of the exponent
            out.append(&mut MOS6502Instruction::AND(bottom_exp_mask).to_bytes());
            // the bottom nibble of the exponent is still in the top nibble of the accumulator, so shift right 4x
            for _ in 0..4 {
                out.append(&mut MOS6502Instruction::LSR.to_bytes());
            }
            // the other option would be to write the byte 'backwards' first and then rotate, but that would need 9 ROL as well as an extra load/store (i think)
            // so this is still quicker, despite the extra ROM it takes
            // or our the top and bottom nibbles together
            out.append(&mut MOS6502Instruction::ORAZ(exp_dest).to_bytes());
            // and save the exponent's bottom byte
            out.append(&mut MOS6502Instruction::STAZ(exp_dest).to_bytes());
            // reload the second byte for the top nibble of the mantissa
            out.append(&mut MOS6502Instruction::TXA.to_bytes());
            out.append(&mut MOS6502Instruction::AND(bottom_man_mask).to_bytes());
            // accumulator holds 0000_(very top nibble of mantissa)
            // where does this get written? there are 52 bits of mantissa total
            // there are 48 remaining after this nibble, these are the bottom 6 bytes (man_dest to man_dest + 5)
            // so we write to the bottom nibble of man_dest + 6
            // at least we don't need to do any shifting!
            out.append(&mut MOS6502Instruction::STAZ(man_dest + 6).to_bytes());
            // then for the other bytes it's pretty simple
            for i in 0..6 {
                out.append(&mut load_acc(&from[i]));
                out.append(&mut MOS6502Instruction::STAZ(man_dest + i as u8).to_bytes());
            }
        }
        _ => panic!("unknown float size")
    }
    out
}

/// pack a float of arbitrary width from one of the float scratch registers back into memory (memory means either a zp virtual register or an actual RAM spill location)
pub fn pack_float(to: &[MOS6502Location], reg: usize) -> Vec<u8> {
    // TODO think this is working but it's hard to tell. how to test??
    let mut out = vec![];
    // decide where we're gonna pack from
    let sign_src;
    let exp_src;
    let man_src;
    match reg {
        0 => {
            sign_src = MOS6502Target::SIG0;
            exp_src = MOS6502Target::EXP0_LO;
            man_src = MOS6502Target::MAN0_LO;
        }
        1 => {
            sign_src = MOS6502Target::SIG1;
            exp_src = MOS6502Target::EXP1_LO;
            man_src = MOS6502Target::MAN1_LO;
        }
        2 => {
            sign_src = MOS6502Target::SIG2;
            exp_src = MOS6502Target::EXP2_LO;
            man_src = MOS6502Target::MAN2_LO;
        }
        _ => {
            panic!("unsupported reg {}", reg);
        }
    }
    // then pack
    // again, figure the width of the float from the number of locations passed
    // packing is simpler
    match to.len() {
        2 => {
            let bottom = &to[0];
            let top = &to[1];
            // the sign bit comes aligned to the top of the byte, where we want it anyway, so let's not worry about that yet
            // the top two bits of the mantissa are also already aligned properly
            // so first load the exponent bits and shift them to the middle of the byte
            // it's faster to load then shift because we need the information in accumulator anyway
            out.append(&mut MOS6502Instruction::LDAZ(exp_src).to_bytes());
            for _ in 0..2 {
                out.append(&mut MOS6502Instruction::ASL.to_bytes());
            }
            // now it's as easy as doing ORA with the sign and the top part of the mantissa
            out.append(&mut MOS6502Instruction::ORAZ(sign_src).to_bytes());
            out.append(&mut MOS6502Instruction::ORAZ(man_src + 1).to_bytes());
            // the top byte is done, write it
            out.append(&mut store_acc(top));
            // then the bottom byte is one load and one store
            out.append(&mut MOS6502Instruction::LDAZ(man_src).to_bytes());
            out.append(&mut store_acc(bottom));
        }
        4 => {
            let top = &to[3];
            let second = &to[2];
            // top gets sign and 7 bits of exponent
            // second gets 1 bit of exponent and 7 of mantissa
            // lda/tax is just as expensive as doing lda 2x
            // use the carry flag to hold the bottom bit of the exponent - clear it first
            out.append(&mut MOS6502Instruction::CLC.to_bytes());
            // load the exponent byte (doesn't touch carry)
            out.append(&mut MOS6502Instruction::LDAZ(exp_src).to_bytes());
            // ROR 1x, puts the bottom bit in the carry flag
            out.append(&mut MOS6502Instruction::ROR.to_bytes());
            // OR in the sign bit (doesn't touch carry)
            out.append(&mut MOS6502Instruction::ORAZ(sign_src).to_bytes());
            // store top byte (doesn't touch carry)
            out.append(&mut store_acc(top));
            // ROR again, puts the bottom bit of the exponent (top bit of second byte) at the top of acc
            out.append(&mut MOS6502Instruction::ROR.to_bytes());
            // the bottom of the second byte is the top 7 bits of the mantissa
            // luckily these are already byte-aligned and stored at man_src + 2, so we just need to ORA and write
            out.append(&mut MOS6502Instruction::ORAZ(man_src + 2).to_bytes());
            out.append(&mut store_acc(second));
            // and then just load/store the other two bytes of mantissa
            for i in 0..2 {
                out.append(&mut MOS6502Instruction::LDAZ(man_src + i as u8).to_bytes());
                out.append(&mut store_acc(&to[i]));
            }
        }
        8 => {
            let top = &to[7];
            let second = &to[6];
            // top gets sign and 7 msbits of exponent
            // second gets 4 lsbits of exponent, 4 bits of mantissa
            // like with unpacking, the alignment is really not that great on the exponent
            // exp_src holds the 8 lsb of the exponent, exp_src + 1 holds the top 3, and nothing is really aligned at all
            // top byte: sign 7msb exponent
            // load the top bits of the exponent
            out.append(&mut MOS6502Instruction::LDAZ(exp_src + 1).to_bytes());
            // shift them left 4x
            for _ in 0..4 {
                out.append(&mut MOS6502Instruction::ASL.to_bytes());
            }
            // write them back - load-shift-write is 3+2*4+3 = 18 cycles, as opposed to shifting in place which is 4*5=20 cycles
            out.append(&mut MOS6502Instruction::STAZ(exp_src + 1).to_bytes());
            // load 8 lsb of exponent
            out.append(&mut MOS6502Instruction::LDAZ(exp_src).to_bytes());
            // we care about the top nibble of these, shift right 4x to isolate
            for _ in 0..4 {
                out.append(&mut MOS6502Instruction::LSR.to_bytes());
            }
            // OR in the top 3 bits of the exponent we saved earlier
            out.append(&mut MOS6502Instruction::ORAZ(exp_src + 1).to_bytes());
            // OR in the sign
            out.append(&mut MOS6502Instruction::ORAZ(sign_src).to_bytes());
            // top byte done
            out.append(&mut store_acc(top));
            // second byte: 4lsb exponent 4msb mantissa
            // load the exponent
            out.append(&mut MOS6502Instruction::LDAZ(exp_src).to_bytes());
            // shift it left so the 4 lsb are in top nibble
            for _ in 0..4 {
                out.append(&mut MOS6502Instruction::ASL.to_bytes());
            }
            // the top nibble of the mantissa is already byte-aligned, so just or it with acc
            out.append(&mut MOS6502Instruction::ORAZ(man_src + 6).to_bytes());
            // done second byte
            out.append(&mut store_acc(second));
            // top two bytes are done, now the bottom 6 are just copying
            for i in 0..6 {
                out.append(&mut MOS6502Instruction::LDAZ(man_src + i as u8).to_bytes());
                out.append(&mut store_acc(&to[i]));
            }
        }
        _ => panic!("unknown float size")
    }
    out
}

/// given an array of moves which logically happen in parallel, generate an equivalent sequential ordering that's conflict-free
///
/// where a cycle exists, use one temp location
///
/// this is a standard algorithm (llvm/cranelift)
pub fn sequentialize_moves<'a>(mut pending: Vec<(&'a MOS6502Location, &'a MOS6502Location)>, scratch: &'a MOS6502Location) -> Vec<(&'a MOS6502Location, &'a MOS6502Location)> {
    let mut result = Vec::with_capacity(pending.len()); // we must have at least as many moves as the input
    while !pending.is_empty() {
        // a move is safe if its destination isn't read from by anything else
        // find the index of the next safe move
        let safe = pending.iter().position(|(_, dest)| !pending.iter().any(|(other_src, _)| other_src == dest));
        // if we found a safe move, add it to the list of moves to make
        if let Some(safe_idx) = safe {
            result.push(pending.remove(safe_idx));
        }
        else {
            // if there were no safe moves, break the next cycle
            // add a move from the next destination into the scratch register (save whatever this move would've overwritten)
            let (_, break_dest) = pending[0].clone();
            result.push((break_dest, scratch)); // it doesn't actually matter where we add this 'save' move because the first part of the loop figures out the correct ordering
            for (src, _) in pending.iter_mut() {
                if *src == break_dest {
                    *src = scratch;
                }
            }
        }
    }
    result
}
