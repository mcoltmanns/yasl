mod codegen;

use std::iter::zip;
use std::slice::SliceIndex;
use crate::datastructures::statement::VRegInstruction;
use crate::target::{lin_alloc, DType};
use crate::target::Target;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MOS6502Location {
    RegisterPool(u8),
    FrameSpill(u8),
}

/// first three letters are opcode
///
/// stuff after indicates addressing mode
///
/// none is immediate or implied
///
/// A is absolute (argument is address, effect is address)
///
/// AX or AY is absolute, x or y indexed (arg is address, effect is arg + x/y with carry)
///
/// ZX or ZY is zero page, x or y indexed (arg is zp address, effect is arg + x/y WITHOUT carry)
///
/// X is x-indexed indirect (arg is a zero-page address, to which x is added (no carry). from this pointer, read a 16 bit address. apply the operation to the value at that address)
///
/// apply index, then indirection (x selects which pointer to use)
///
/// Y is indirect y-indexed (arg is a pointer to a base address, to which y is added (with carry). the op is applied to what is at this pointer)
///
/// apply indirection, then index (y modifies the pointer)
pub enum MOS6502Instruction {
    BRK,
    BPL(u8),
    JSR(u16),
    BMI(u8),
    RTI,
    BVC(u8),
    RTS,
    BVS(u8),
    BCC(u8),
    LDY(u8),
    BCS(u8),
    CPY(u8),
    BNE(u8),
    CPX(u8),
    BEQ(u8),
    ORAX(u8),
    ORAY(u8),
    ANDX(u8),
    ANDY(u8),
    EORX(u8),
    EORY(u8),
    ADCX(u8),
    ADCY(u8),
    STAX(u8),
    STAY(u8),
    LDAX(u8),
    LDAY(u8),
    CMPX(u8),
    CMPY(u8),
    SBCX(u8),
    SBCY(u8),
    LDX(u8),
    BITZ(u8),
    STYZ(u8),
    STYZX(u8),
    LDYZ(u8),
    LDYZX(u8),
    CPYZ(u8),
    CPXZ(u8),
    ORAZ(u8),
    ORAZX(u8),
    ANDZ(u8),
    ANDZX(u8),
    EORZ(u8),
    EORZX(u8),
    ADCZ(u8),
    ADCZX(u8),
    STAZ(u8),
    STAZX(u8),
    LDAZ(u8),
    LDAZX(u8),
    CMPZ(u8),
    CMPZX(u8),
    SBCZ(u8),
    SBCZX(u8),
    ASLZ(u8),
    ASLZX(u8),
    ROLZ(u8),
    ROLZX(u8),
    LSRZ(u8),
    LSRZX(u8),
    RORZ(u8),
    RORZX(u8),
    STXZ(u8),
    STXZY(u8),
    LDXZ(u8),
    LDXZY(u8),
    DECZ(u8),
    DECZX(u8),
    INCZ(u8),
    INCZX(u8),
    PHP,
    CLC,
    PLP,
    SEC,
    PHA,
    CLI,
    PLA,
    SEI,
    DEY,
    TYA,
    TAY,
    CLV,
    INY,
    CLD,
    INX,
    SED,
    ORA(u8),
    ORAAY(u16),
    AND(u8),
    ANDAY(u16),
    EOR(u8),
    EORAY(u16),
    ADC(u8),
    ADCAY(u16),
    STAAY(u16),
    LDA(u8),
    LDAAY(u16),
    CMP(u8),
    CMPAY(u16),
    SBC(u8),
    SBCAY(u16),
    ASL,
    ROL,
    LSR,
    ROR,
    TXA,
    TXS,
    TAX,
    TSX,
    DEX,
    NOP,
    BIT(u16),
    JMP(u16),
    JMPI(u16),
    STY(u16),
    LDYA(u16),
    LDYAX(u16),
    CPYA(u16),
    CPXA(u16),
    ORAA(u16),
    ORAAX(u16),
    ANDA(u16),
    ANDAX(u16),
    EORA(u16),
    EORAX(u16),
    ADCA(u16),
    ADCAX(u16),
    STAA(u16),
    STAAX(u16),
    LDAA(u16),
    LDAAX(u16),
    CMPA(u16),
    CMPAX(u16),
    SBCA(u16),
    SBCAX(u16),
    ASLA(u16),
    ASLAX(u16),
    ROLA(u16),
    ROLAX(u16),
    LSRA(u16),
    LSRAX(u16),
    RORA(u16),
    RORAX(u16),
    STXA(u16),
    LDXA(u16),
    LDXAY(u16),
    DECA(u16),
    DECAX(u16),
    INCA(u16),
    INCAX(u16),
}
impl MOS6502Instruction {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            MOS6502Instruction::BRK => vec![0x00],
            MOS6502Instruction::BPL(arg) => vec![0x10, *arg],
            MOS6502Instruction::JSR(arg) => vec![0x20, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::BMI(arg) => vec![0x30, *arg],
            MOS6502Instruction::RTI => vec![0x40],
            MOS6502Instruction::BVC(arg) => vec![0x50, *arg],
            MOS6502Instruction::RTS => vec![0x60],
            MOS6502Instruction::BVS(arg) => vec![0x70, *arg],
            MOS6502Instruction::BCC(arg) => vec![0x90, *arg],
            MOS6502Instruction::LDY(arg) => vec![0xa0, *arg],
            MOS6502Instruction::BCS(arg) => vec![0xb0, *arg],
            MOS6502Instruction::CPY(arg) => vec![0xc0, *arg],
            MOS6502Instruction::BNE(arg) => vec![0xd0, *arg],
            MOS6502Instruction::CPX(arg) => vec![0xe0, *arg],
            MOS6502Instruction::BEQ(arg) => vec![0xf0, *arg],
            MOS6502Instruction::ORAX(arg) => vec![0x01, *arg],
            MOS6502Instruction::ORAY(arg) => vec![0x11, *arg],
            MOS6502Instruction::ANDX(arg) => vec![0x21, *arg],
            MOS6502Instruction::ANDY(arg) => vec![0x31, *arg],
            MOS6502Instruction::EORX(arg) => vec![0x41, *arg],
            MOS6502Instruction::EORY(arg) => vec![0x51, *arg],
            MOS6502Instruction::ADCX(arg) => vec![0x61, *arg],
            MOS6502Instruction::ADCY(arg) => vec![0x71, *arg],
            MOS6502Instruction::STAX(arg) => vec![0x81, *arg],
            MOS6502Instruction::STAY(arg) => vec![0x91, *arg],
            MOS6502Instruction::LDAX(arg) => vec![0xa1, *arg],
            MOS6502Instruction::LDAY(arg) => vec![0xb1, *arg],
            MOS6502Instruction::CMPX(arg) => vec![0xc1, *arg],
            MOS6502Instruction::CMPY(arg) => vec![0xd1, *arg],
            MOS6502Instruction::SBCX(arg) => vec![0xe1, *arg],
            MOS6502Instruction::SBCY(arg) => vec![0xf1, *arg],
            MOS6502Instruction::LDX(arg) => vec![0xa2, *arg],
            MOS6502Instruction::BITZ(arg) => vec![0x24, *arg],
            MOS6502Instruction::STYZ(arg) => vec![0x84, *arg],
            MOS6502Instruction::STYZX(arg) => vec![0x94, *arg],
            MOS6502Instruction::LDYZ(arg) => vec![0xa4, *arg],
            MOS6502Instruction::LDYZX(arg) => vec![0xb4, *arg],
            MOS6502Instruction::CPYZ(arg) => vec![0xc4, *arg],
            MOS6502Instruction::CPXZ(arg) => vec![0xe4, *arg],
            MOS6502Instruction::ORAZ(arg) => vec![0x05, *arg],
            MOS6502Instruction::ORAZX(arg) => vec![0x15, *arg],
            MOS6502Instruction::ANDZ(arg) => vec![0x25, *arg],
            MOS6502Instruction::ANDZX(arg) => vec![0x35, *arg],
            MOS6502Instruction::EORZ(arg) => vec![0x45, *arg],
            MOS6502Instruction::EORZX(arg) => vec![0x55, *arg],
            MOS6502Instruction::ADCZ(arg) => vec![0x65, *arg],
            MOS6502Instruction::ADCZX(arg) => vec![0x75, *arg],
            MOS6502Instruction::STAZ(arg) => vec![0x85, *arg],
            MOS6502Instruction::STAZX(arg) => vec![0x95, *arg],
            MOS6502Instruction::LDAZ(arg) => vec![0xa5, *arg],
            MOS6502Instruction::LDAZX(arg) => vec![0xb5, *arg],
            MOS6502Instruction::CMPZ(arg) => vec![0xc5, *arg],
            MOS6502Instruction::CMPZX(arg) => vec![0xd5, *arg],
            MOS6502Instruction::SBCZ(arg) => vec![0xe5, *arg],
            MOS6502Instruction::SBCZX(arg) => vec![0xf5, *arg],
            MOS6502Instruction::ASLZ(arg) => vec![0x06, *arg],
            MOS6502Instruction::ASLZX(arg) => vec![0x16, *arg],
            MOS6502Instruction::ROLZ(arg) => vec![0x26, *arg],
            MOS6502Instruction::ROLZX(arg) => vec![0x36, *arg],
            MOS6502Instruction::LSRZ(arg) => vec![0x46, *arg],
            MOS6502Instruction::LSRZX(arg) => vec![0x56, *arg],
            MOS6502Instruction::RORZ(arg) => vec![0x66, *arg],
            MOS6502Instruction::RORZX(arg) => vec![0x76, *arg],
            MOS6502Instruction::STXZ(arg) => vec![0x86, *arg],
            MOS6502Instruction::STXZY(arg) => vec![0x96, *arg],
            MOS6502Instruction::LDXZ(arg) => vec![0xa6, *arg],
            MOS6502Instruction::LDXZY(arg) => vec![0xb6, *arg],
            MOS6502Instruction::DECZ(arg) => vec![0xc6, *arg],
            MOS6502Instruction::DECZX(arg) => vec![0xd6, *arg],
            MOS6502Instruction::INCZ(arg) => vec![0xe6, *arg],
            MOS6502Instruction::INCZX(arg) => vec![0xf6, *arg],
            MOS6502Instruction::PHP => vec![0x08],
            MOS6502Instruction::CLC => vec![0x18],
            MOS6502Instruction::PLP => vec![0x28],
            MOS6502Instruction::SEC => vec![0x38],
            MOS6502Instruction::PHA => vec![0x48],
            MOS6502Instruction::CLI => vec![0x58],
            MOS6502Instruction::PLA => vec![0x68],
            MOS6502Instruction::SEI => vec![0x78],
            MOS6502Instruction::DEY => vec![0x88],
            MOS6502Instruction::TYA => vec![0x98],
            MOS6502Instruction::TAY => vec![0xa8],
            MOS6502Instruction::CLV => vec![0xb8],
            MOS6502Instruction::INY => vec![0xc8],
            MOS6502Instruction::CLD => vec![0xd8],
            MOS6502Instruction::INX => vec![0xe8],
            MOS6502Instruction::SED => vec![0xf8],
            MOS6502Instruction::ORA(arg) => vec![0x09, *arg],
            MOS6502Instruction::ORAAY(arg) => vec![0x19, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::AND(arg) => vec![0x29, arg.to_le_bytes()[0]],
            MOS6502Instruction::ANDAY(arg) => vec![0x39, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::EOR(arg) => vec![0x49, arg.to_le_bytes()[0]],
            MOS6502Instruction::EORAY(arg) => vec![0x59, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ADC(arg) => vec![0x69, arg.to_le_bytes()[0]],
            MOS6502Instruction::ADCAY(arg) => vec![0x79, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::STAAY(arg) => vec![0x99, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::LDA(arg) => vec![0xa9, *arg],
            MOS6502Instruction::LDAAY(arg) => vec![0xb9, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::CMP(arg) => vec![0xc9, *arg],
            MOS6502Instruction::CMPAY(arg) => vec![0xd9, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::SBC(arg) => vec![0xe9, *arg],
            MOS6502Instruction::SBCAY(arg) => vec![0xf9, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ASL => vec![0x0a],
            MOS6502Instruction::ROL => vec![0x2a],
            MOS6502Instruction::LSR => vec![0x4a],
            MOS6502Instruction::ROR => vec![0x6a],
            MOS6502Instruction::TXA => vec![0x8a],
            MOS6502Instruction::TXS => vec![0x9a],
            MOS6502Instruction::TAX => vec![0xaa],
            MOS6502Instruction::TSX => vec![0xba],
            MOS6502Instruction::DEX => vec![0xca],
            MOS6502Instruction::NOP => vec![0xea],
            MOS6502Instruction::BIT(arg) => vec![0x2c, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::JMP(arg) => vec![0x4c, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::JMPI(arg) => vec![0x6c, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::STY(arg) => vec![0x8c, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::LDYA(arg) => vec![0xac, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::LDYAX(arg) => vec![0xbc, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::CPYA(arg) => vec![0xcc, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::CPXA(arg) => vec![0xec, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ORAA(arg) => vec![0x0d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ORAAX(arg) => vec![0x1d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ANDA(arg) => vec![0x2d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ANDAX(arg) => vec![0x3d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::EORA(arg) => vec![0x4d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::EORAX(arg) => vec![0x5d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ADCA(arg) => vec![0x6d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ADCAX(arg) => vec![0x7d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::STAA(arg) => vec![0x8d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::STAAX(arg) => vec![0x9d, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::LDAA(arg) => vec![0xad, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::LDAAX(arg) => vec![0xbd, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::CMPA(arg) => vec![0xcd, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::CMPAX(arg) => vec![0xdd, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::SBCA(arg) => vec![0xed, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::SBCAX(arg) => vec![0xfd, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ASLA(arg) => vec![0x0e, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ASLAX(arg) => vec![0x1e, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ROLA(arg) => vec![0x2e, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::ROLAX(arg) => vec![0x3e, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::LSRA(arg) => vec![0x4e, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::LSRAX(arg) => vec![0x5e, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::RORA(arg) => vec![0x6e, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::RORAX(arg) => vec![0x7e, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::STXA(arg) => vec![0x8e, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::LDXA(arg) => vec![0xae, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::LDXAY(arg) => vec![0xbe, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::DECA(arg) => vec![0xce, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::DECAX(arg) => vec![0xde, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::INCA(arg) => vec![0xee, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
            MOS6502Instruction::INCAX(arg) => vec![0xfe, arg.to_le_bytes()[0], arg.to_le_bytes()[1]],
        }
    }
}

pub struct MOS6502Target {
    // everything is little-endian
    // $0000-$00FF is zero page, fast access
    // $0100-$01FF is hardware stack, reserved for the processor's call/return stack (strictly
    // addresses)
    // $FFFA-$FFFF are interrupt and reset vectors, also reserved
    //
    // the first 64 bytes ($0000-$0039) are the zero page register pool (used by the compiler during register allocation)
    // $0040-$0041 is DFP (data frame pointer)
    // $0042-$0052 are floating point scratch
    // it's much easier (at little spatial cost) to work on floats when their components are byte-aligned
    // all of these float sections have enough space for the components of an up to 64 bit float
    // also all of the mantissa registers have enough room to be used as 64-bit math accumulators if need be
    // $0042 is SIG0 (sign 0)
    // $0043-$0044 is EXP0 (exponent 0)
    // $0045-$004c is MAN0 (mantissa 0)
    // $004d is SIG1 (sign 1)
    // $004e-$004f is EXP1 (exponent 1)
    // $0050-$0057 is MAN1 (mantissa 1)
    // $0058 is SIG2
    // $0059-$0060 is EXP2
    // $0061-$0068 is MAN2
    // $0000-$0068 are all compiler reserved
    // $0069-$00ff is free zero page (for the user)
    // $0100-$01ff is the hardware stack
    // $0200-$05ff is the data frame stack (grows down, holds arguments and spills)
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
    // in memory. if the user does not specify, assume $8000 (this is the start of rom in the
    // architecture we're assuming)
    // zero page is the working registers

    // free registers are a u64 bitmap where 1 is free, 0 is used
    free_registers: u64,
    // vector of spill blocks which have been used and freed
    // args are start, length (max length is 255 because of limitations in the 6502 indirect addressing modes)
    // the starts of spills are byte offsets from the stack data frame (since we spill to stack)
    free_spill: Vec<(u8, u8)>,
    // where the next spill will take place
    next_spill: u8,
}
impl MOS6502Target {
    // low byte of the DFP
    // this tracks the current base of the current stack data frame
    const DFP_LO: u8 = 0x0040;
    // addresses of all the float registers
    // signs are 1 byte (always 1 bit), exponents are 2 bytes (11 bits at most), mantissas are 7 bytes (52 bits at most)
    const SIG0: u8 = 0x42;
    const EXP0_LO: u8 = 0x43;
    const MAN0_LO: u8 = 0x45;
    const SIG1: u8 = 0x4c;
    const EXP1_LO: u8 = 0x4d;
    const MAN1_LO: u8 = 0x4f;
    const SIG2: u8 = 0x56;
    const EXP2_LO: u8 = 0x57;
    const MAN2_LO: u8 = 0x59;
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

    fn alloc(&mut self, needed: usize) -> Result<Vec<Self::Location>, String> {
        // if we don't need any space, return an empty location vector
        if needed == 0 {
            return Ok(vec![]);
        }
        else if needed > u8::MAX as usize {
            return Err("Cannot allocate more than 255 memory".to_string());
        }

        let needed = needed as u8;

        // try to find space in the registers
        for start in 0..=(64 - needed) {
            let mask = ((1u64 << needed) - 1) << start;

            if self.free_registers & mask == mask {
                self.free_registers &= !mask;

                return Ok((start..start + needed).map(|i| MOS6502Location::RegisterPool(i as u8)).collect());
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
                return Ok((start..start + needed).map(MOS6502Location::FrameSpill).collect());
            }
            // fits too large?
            if len > needed {
                // push the block's start up by the amount of space we need
                self.free_spill[i].0 += needed;
                // and shorten the block by that amount too
                self.free_spill[i].1 -= needed;
                // and return all the locations in the block before modification
                return Ok((start..start + needed).map(MOS6502Location::FrameSpill).collect());
            }
        }
        // if we couldn't find any blocks to reuse, allocate more spill space
        let start = self.next_spill;
        self.next_spill += needed;
        if self.next_spill > u8::MAX {
            return Err("Cannot allocate more than 255 spill".to_string());
        }
        Ok((start..start + needed).map(MOS6502Location::FrameSpill).collect())
    }

    fn emit(program: &crate::datastructures::program::VRegProgram) -> Result<Vec<u8>, String> {
        // emit a program
        // emit more or less in source order
        // we want to emit binary, not assembly

        // how do we emit a program?
        // we need a datastructure to hold the program, aka a model of the target's memory
        // this can just be an array of bytes
        // it's wasteful to model the entire memory since only a part of it is rom
        let mut system_rom = vec![0; 0xffff + 1];
        // initialize the data frame pointer
        system_rom[Self::DFP_LO as usize] = 0x00;
        system_rom[Self::DFP_LO as usize + 1] = 0x02;

        // where we're emitting to right now
        let mut curr_emit_addr = MOS6502Target::PROG_START_DEFAULT;

        // now the target is pretty much ready to go
        // we go procedure by procedure and write instructions
        for (proc_name, vreg_proc) in program.proc_table() {
            // first we allocate for the procedure
            let allocation = lin_alloc::<MOS6502Target>(vreg_proc)?;

            println!("{}", proc_name);

            // if the procedure is the entry point, write the reset vector
            if proc_name == "main" {
                system_rom[0xfffc] = curr_emit_addr.to_le_bytes()[0];
                system_rom[0xfffd] = curr_emit_addr.to_le_bytes()[1];
            }
            
            let mut write_bytes = | bytes: &[u8] | {
                system_rom[curr_emit_addr as usize..curr_emit_addr as usize + bytes.len()].copy_from_slice(bytes);
                curr_emit_addr += bytes.len() as u16;
            };

            for instruction in vreg_proc.instructions() {
                println!("\t{:?}", instruction);
                match instruction {
                    // load a literal into memory
                    VRegInstruction::LoadImm { dest, val } => {
                        // pointers are target-dependent, so we have to do pointer bounds checking here
                        // it's guaranteed to be non-negative since it's a u64, so we just have to check the upper bounds
                        if *dest.holds() == DType::Pointer {
                            // if a pointer fits in 2 bytes, only the two lsb will be nonzero
                            // so if the length is > 2 and all bytes 2.. are 0, the pointer will fit
                            // the length check shouldn't be necessary? but it's safer
                            let ptr_bytes = val.as_bytes();
                            let fits = ptr_bytes.len() > 2 && ptr_bytes[2..].iter().all(|&x| x == 0);
                            if !fits {
                                // TODO instead of panicking here, you need to make VRegInstruction a positionable struct and integrate the target into the usual error-reporting framework
                                panic!("pointer too large for target address space");
                            }
                        }
                        // look up the destination location
                        let dloc = allocation.get(dest).unwrap();
                        // loading immediate values happens as a byte loop
                        for (location, byte) in zip(dloc, val.as_bytes()) {
                            write_bytes(&codegen::store_byte(location, byte));
                        }
                    }
                    VRegInstruction::Move { dest, src } => {
                        // move is pretty easy, just load to accumulator/store to memory
                        let dlocs = allocation.get(dest).unwrap();
                        let slocs = allocation.get(src).unwrap();
                        for (dloc, sloc) in zip(dlocs, slocs) {
                            // load a with whatever was at the start register
                            write_bytes(&codegen::load_acc(sloc));
                            // write it to the destination
                            write_bytes(&codegen::store_acc(dloc));
                        }
                    }

                    // math stuff
                    VRegInstruction::Add { dest, a, b } => {
                        let dlocs = allocation.get(dest).unwrap();
                        let alocs = allocation.get(a).unwrap();
                        let blocs = allocation.get(b).unwrap();
                        if dest.holds().is_integer() {
                            // first clear the carry flag
                            write_bytes(&MOS6502Instruction::CLC.to_bytes());
                            // then for each a-b-d pair, perform the addition
                            // carry flag is considered
                            for (dloc, (aloc, bloc)) in zip(dlocs, zip(alocs, blocs)) {
                                write_bytes(&codegen::add_whole(dloc, aloc, bloc))
                            }
                        }
                        else {
                            //todo!("floats are not implemented yet")
                            write_bytes(&codegen::unpack_float(alocs, 0));
                            write_bytes(&codegen::unpack_float(blocs, 1));
                        }
                    }
                    VRegInstruction::Sub { dest, a, b } => {
                        // subtraction is the same idea as addition, but we use SBC
                        let dlocs = allocation.get(dest).unwrap();
                        let alocs = allocation.get(a).unwrap();
                        let blocs = allocation.get(b).unwrap();
                        // for subtraction you have to set carry
                        write_bytes(&MOS6502Instruction::SEC.to_bytes());
                        // then for each a-b-d pair, subtract (considering carry)
                        for (dloc, (aloc, bloc)) in zip(dlocs, zip(alocs, blocs)) {
                            write_bytes(&codegen::sub_whole(dloc, aloc, bloc))
                        }
                    }
                    VRegInstruction::Mul { dest, a, b } => {
                        // multiplication is a little harder
                        // we don't know what our registers contain, so we can't really make use of any fancy multiplication tricks (all of the stuff we know at compile time was handled already anyway)
                        // so we just do booth's
                    }

                    _ => println!("\tunimplemented instruction: {:?}", instruction)
                }
            }
        }
        Ok(system_rom)
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
        let mut merged: Vec<(u8, u8)> = Vec::new();
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

