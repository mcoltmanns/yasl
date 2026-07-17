use yasl::datastructures::program::VRegProgram;
use yasl::datastructures::program::VirtualProgram;
use yasl::logger::{Logger, StdoutLogger};
use yasl::target::Target;
use yasl::target::mos6502::{MOS6502Target};
use yasl::target::x86_64::{X86_64Target};
use yasl::tokenizer;
use yasl::parser;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use yasl::util::{FilePos, Positionable};

fn main() {
    println!("This is yasl {}", env!("CARGO_PKG_VERSION"));

    let mut logger = StdoutLogger::new();

    let args: Vec<String> = env::args().collect();

    let src_path = std::path::Path::new(&args[1]);
    let src_path_str = src_path.to_str().unwrap();
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
    let mut ir_program = VirtualProgram::new(src_path_str, parser.statements(), &mut logger);
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

    // lower the program to virtual register form
    // in theory nothing can go wrong here
    let vreg_program = VRegProgram::lower(src_path_str, &ir_program, &logger);
    println!("{}", vreg_program);

    // select target
    let target_str = "6502";
    let mut bytes = vec![];
    // the pipeline for targets is basically all the same, but because target types can't be known at compile time we need this big match statement
    match target_str {
        "6502" => {
            let alloc_map = MOS6502Target::build_alloc_map(&vreg_program, &mut logger);
            let trap_locs = MOS6502Target::build_trap_set(&vreg_program, &alloc_map, 64, &mut logger);
            println!("{:#?}", trap_locs);
            bytes = MOS6502Target::emit(&vreg_program, &alloc_map, &trap_locs, &mut logger);
        }
        "x86_64" => {
            todo!()
        }
        _ => {
            panic!("specify a target string")
        }
    }
    if let Ok(mut file) = File::create("./out.bin") {
        file.write_all(&*bytes).expect("unable to write output");
    }
    else {
        panic!("unable to create output file");
    }
    println!("done");
}
