use yasl::datastructures::program::Program;
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
            println!("unable to open file \"{}\": {}", src_path.to_str().unwrap(), err);
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
    let mut ir_program = Program::new(parser.statements(), &mut logger);
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
    println!("{}", ir_program);

    //// so now we know the program is correct (at least as correct as we can know it to be)
    //// it's time to think about code generation
    //// the general pipeline is:
    //// source -> typed ir -> virtual register ir --(register allocation)-> real register ir
    //// --(instruction selection)-> assembly
    //
    //// lowering to virtual register IR
    //// each place on the stack becomes a virtual register name
    //// i mean each!
    //// you have infinite virtual registers
    /*let reg_proc_table = convert_proc_table(&procedure_table, &mut logger);
    for rp in reg_proc_table.values() {
        println!("{}", rp.name);
        println!("input registers: {:?}", rp.inputs);
        println!("output registers: {:?}", rp.outputs);
        for i in &rp.instructions {
            println!("  {:?}", i);
        }
    }*/
}
