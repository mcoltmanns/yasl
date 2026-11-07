use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use yasl::logger::{Logger, StderrLogger};
use yasl::{parser, tokenizer};
use yasl::util::source::SourceMap;

fn main() {
    println!("This is yasl {}", env!("CARGO_PKG_VERSION"));

    let mut logger = StderrLogger::default();

    let args: Vec<String> = env::args().collect();
    let src_name = &args[1];

    let mut src_map = SourceMap::default();
    let src_id = src_map.add(PathBuf::from(src_name));
    let src_file = src_map.get_file(src_id);
    let src_string = src_file.get_contents().unwrap();

    // tokenize the input
    let tokens = tokenizer::tokenize(src_string, src_id);

    let bail = |logger: &mut dyn Logger| -> ! {
        logger.flush(&src_map);
        exit(1)
    };

    // parse the input
    let mut stackprogram = parser::parse(tokens, &mut logger).unwrap_or_else(|_| bail(&mut logger));

    //// complete the stack program with basic block info
    //blocker::run(&mut stackprogram, &mut logger).unwrap_or_else(|_| bail(&mut logger));

    //// do control flow checks
    //flowchecker::run(&mut stackprogram, &mut logger).unwrap_or_else(|_| bail(&mut logger));

    // build a stack program from the tokenized input
    //let stack_program = StackProgram::from_tokens(tokens, &mut logger)?;

    // build the procedure table and derive a signature table from it
    /*let mut ir_program = VirtualProgram::new(src_path_str, parser.statements(), &mut logger);
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
    }*/
    println!("done");
}
