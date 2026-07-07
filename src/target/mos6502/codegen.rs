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
            println!("\t\twrite {} to {:02X} plus DFP", byte, *i);
        }
        // registerpool is a direct location in zero page
        MOS6502Location::RegisterPool(i) => {
            // store a zero page
            out.append(&mut MOS6502Instruction::STAZ(*i).to_bytes());
            println!("\t\twrite {} to {:02X}", byte, *i);
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
            // when reconstructing remember you have to ASL this 8x for it to make any sense
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
            // second contains 7 bits of the exponent and one of the mantissa
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
            // mask out the last bit of the exponent
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
