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
    println!("{}", ir_program);

    // now the program is correct
    // next step is to lower to a virtual register/virtual instruction ir
    // here we can already emit typed math instructions
    // but stick to infinite registers for now
    // register allocation depends on the target
    let vreg_program = VRegProgram::lower(&ir_program);
    println!("{}", vreg_program);
}
