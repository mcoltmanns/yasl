use yasl::logger;
use yasl::logger::Logger;
//use yasl::logger::Logger;
//use yasl::procedure::Procedure;
//use yasl::regmachine::convert_proc_table;
use yasl::tokenizer;
use yasl::parser;
//use yasl::procedure;
//use yasl::statement;
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
            panic!("Could not open file: {err:?}")
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
        return
    }

    //// now we have the statements, and they're at least syntactically valid
    //// we can do the first pass to build the procedure table
    //// procedure table maps procedure names to procedures
    //// construct procedure table
    //let mut procedure_table = procedure::ProcedureTable::new();
    //let mut signature_table = procedure::SignatureTable::new();

    //let mut current_proc: Option<Procedure> = None;
    //let mut current_statements = vec![];
    //for s in &parser.statements {
    //    match s.kind() {
    //        statement::StatementKind::Proc { name , t_in, t_out } => {
    //            if let Some(mut prev_proc) = current_proc {
    //                prev_proc.set_statements(current_statements);
    //                current_statements = vec![];
    //                signature_table.insert(prev_proc.name().clone(), (prev_proc.get_intypes().clone(), prev_proc.get_outtypes().clone()));
    //                procedure_table.insert(prev_proc.name().clone(), prev_proc);
    //            }
    //            if procedure_table.contains_key(name) {
    //                logger.error(format!("procedure \"{}\" defined twice", name), s.pos().line, s.pos().col);
    //            }
    //            current_proc = Some(Procedure::new(name.clone(), s.pos().clone(), t_in.clone(), t_out.clone(), vec![]));
    //        }
    //        _ => {
    //            match &current_proc {
    //                Some(_) => {
    //                    current_statements.push(s.clone());
    //                }
    //                None => {
    //                    logger.warning("unreachable code".to_string(), s.pos().line, s.pos().col);
    //                }
    //            }
    //        }
    //    }
    //}
    //if let Some(mut prev_proc) = current_proc {
    //    prev_proc.set_statements(current_statements);
    //    signature_table.insert(prev_proc.name().clone(), (prev_proc.get_intypes().clone(), prev_proc.get_outtypes().clone()));
    //    procedure_table.insert(prev_proc.name().to_string(), prev_proc);
    //}

    //if !procedure_table.contains_key("main") {
    //    logger.error("no main procedure defined".to_string(), 0, 0);
    //}

    //// procedure-local analysis
    //// build procedure jump table and blocks
    //// and simulate type stack within the procedure
    //for p in procedure_table.values_mut() {
    //    // build jump table
    //    p.build_jumps_and_blocks(&mut logger);
    //    // link blocks in procedure
    //    p.link_blocks(&mut logger);
    //    // block-local type analysis
    //    for i in 0..p.get_blocks().len() {
    //        p.compute_block_pushes_and_pops(i, &signature_table, &mut logger);
    //    }
    //    // procedure-local type resolution
    //    // because we have fixpoints for types in a procedure signature, all type
    //    // resolution can take place at the procedural level
    //    // this also checks for non-returning procedures (ensures all reachable blocks without
    //    // successors have a return statement)
    //    p.resolve_types(&mut logger);
    //}

    //for p in procedure_table.values() {
    //    println!("{} {:?} {:?}", p.name(), p.get_intypes(), p.get_outtypes());
    //    for s in p.get_statements() {
    //        println!("  {}", s);
    //    }
    //    for (i, b) in p.get_blocks().iter().enumerate() {
    //        println!("  Basic block {} begins at statement {} and has length {}", i, b.start, b.length);
    //        println!("    Predecessors are: {:?}", b.predecessors);
    //        println!("    Successors are: {:?}", b.successors);
    //        println!("    Requires: {:?}", b.pops);
    //        println!("    Leaves: {:?}", b.pushes);
    //    }
    //}

    //// generally speaking we try to continue through and give as many errors as possible to
    //// inform the developer
    //// but emitting code that has produced errors is undefined behavior
    //// so exit before emission if errors were produced
    //if logger.has_error() {
    //    println!("compilation failed with errors");
    //    return
    //}

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
