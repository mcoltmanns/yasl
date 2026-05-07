use yasl::datastructures::program::VRegProgram;
use yasl::datastructures::program::VirtualProgram;
use yasl::logger;
use yasl::logger::Logger;
use yasl::tokenizer;
use yasl::parser;
use std::env;
use std::fs;

fn main() {
    println!("This is yasl {}", env!("CARGO_PKG_VERSION"));

    let mut logger = logger::StdoutLogger::new();

    let args: Vec<String> = env::args().collect();

    let src_path = std::path::Path::new(&args[1]);
    let src_string = match fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(err) => {
            println!("unable to open file \"{}\": {}\ncompilation failed", src_path.to_str().unwrap(), err);
            return;
        }
    };

    // tokenize the input
    let mut tokenizer = tokenizer::Tokenizer::new(src_path.to_str().unwrap().to_string(), src_string);
    let tokens = tokenizer.run();

    // parse the input into statements
    let mut parser = parser::Parser::new(tokens);
    parser.parse_tokens(&mut logger);

    // not worth continuing if the syntax is wrong
    if logger.has_error() {
        println!("compilation failed");
        return;
    }

    // build the procedure table and derive a signature table from it
    let mut ir_program = VirtualProgram::new(parser.statements(), &mut logger);
    let sig_table = ir_program.sig_table();

    // for every procedure, link the blocks and build the jump table
    // make sure all blocks are reachable
    // and then do type resolution and checking
    for p in ir_program.procedures_mut() {
        p.build_blocks_and_jumps(&mut logger);
        p.link_blocks(&mut logger);
        p.check_block_reachability(&mut logger);
        p.compute_block_stack_effets(&sig_table, &mut logger);
        p.resolve_types(&sig_table, &mut logger);
    }
    // not worth continuing if the types are wrong
    if logger.has_error() {
        println!("compilation failed");
        return;
    }

    // now the program is correct
    // the compiler can throw no more errors, the only things that can go wrong from here on out
    // are internal and cause crashes
    // first we lower the program to infinite virtual registers
    let vreg_program = VRegProgram::lower(&ir_program);
    println!("{}", vreg_program);

    // then we select our target and do register allocation and instruction selection based on that
    // get linear scan working first, then maybe worry about graph coloring
    // we want to target: x86_64 (64 bit) x86 (32 bit) 6502 (8 bit)
    // probably do the 6502 first, because it is the simplest
    // targets own their allocators and their emitters

    // linear scan allocation
    // within a procedure, determine live ranges of your registers
    // a register goes live when it is first used and goes dead when it is last used
    // registers which have overlapping live ranges will need to be spilled
    // careful - registers used in loops are live for the whole loop (from the label to the jump)
    // plus any use points after the jump
    
    // 6502
    // we are targeting bare metal 6502
    // accumulator (A)
    // in many cases operations can be performed directly on the accumulator
    // pretty much everything runs through A
    // X and Y are really only useful for indexing
    // the stack is tiny. only use it for jsr/rts and pha/pla
    // your accumulator is only 8 bits. everything will need to be spilled.
    // the hardware stack is at $0100 to $01ff, zero page is $0000-$00ff and is very fast. but
    // limited.
    // if things are in zero page they can also be used for indirect addressing
    // prioritize allocating zero page to pointers
    // our magic infinite registers are almost all wider than 8 bits.
    // map virtual registers to real locations, which have an address (u16) and a width (u8), both
    // in bytes
    //
    // first step: map virtual registers to real locations
}
