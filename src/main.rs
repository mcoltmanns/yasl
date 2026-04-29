use yasl::logger;
use yasl::logger::Logger;
use yasl::procedure::Procedure;
use yasl::tokenizer;
use yasl::parser;
use yasl::procedure;
use yasl::statement;
use std::fs;

fn main() {
    println!("This is yasl {}", env!("CARGO_PKG_VERSION"));

    let mut logger = logger::StdoutLogger::new();

    let src_path = std::path::Path::new("./test.yas");
    let src_string = match fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(err) => {
            panic!("Could not open file: {err:?}")
        }
    };

    // tokenize the input
    let mut tokenizer = tokenizer::Tokenizer::new(&src_string);
    let tokens = tokenizer.run();

    // parse the input into statements
    let mut parser = parser::Parser::new(tokens, &mut logger);
    parser.parse_tokens();

    // now we have the statements, and they're at least syntactically valid
    // we can do the first pass to build the procedure table
    // procedure table maps procedure names to procedures
    // construct procedure table
    let mut procedure_table = procedure::ProcedureTable::new();
    let mut signature_table = procedure::SignatureTable::new();

    let mut current_proc: Option<Procedure> = None;
    let mut current_statements = vec![];
    for s in &parser.statements {
        match s.kind() {
            statement::StatementKind::Proc { name , t_in, t_out } => {
                if let Some(mut prev_proc) = current_proc {
                    prev_proc.set_statements(current_statements);
                    current_statements = vec![];
                    signature_table.insert(prev_proc.name().clone(), (prev_proc.get_intypes().clone(), prev_proc.get_outtypes().clone()));
                    procedure_table.insert(prev_proc.name().clone(), prev_proc);
                }
                if procedure_table.contains_key(name) {
                    logger.error(format!("procedure \"{}\" defined twice", name), s.pos().line, s.pos().col);
                }
                current_proc = Some(Procedure::new(name.clone(), t_in.clone(), t_out.clone(), vec![]));
            }
            _ => {
                match &current_proc {
                    Some(_) => {
                        current_statements.push(s.clone());
                    }
                    None => {
                        logger.warning("statement outside of procedure is unreachable".to_string(), s.pos().line, s.pos().col);
                    }
                }
            }
        }
    }
    if let Some(mut prev_proc) = current_proc {
        prev_proc.set_statements(current_statements);
        signature_table.insert(prev_proc.name().clone(), (prev_proc.get_intypes().clone(), prev_proc.get_outtypes().clone()));
        procedure_table.insert(prev_proc.name().to_string(), prev_proc);
    }

    if !procedure_table.contains_key("main") {
        logger.error("no main procedure defined".to_string(), 0, 0);
    }

    // build blocks for each procedure table
    for p in procedure_table.values_mut() {
        p.build_jumps_and_blocks(&mut logger);
    }

    // check types for each procedure
    for p in procedure_table.values_mut() {
        for i in 0..p.get_blocks().len() {
            p.check_block(i, &signature_table, &mut logger);
        }
    }

    for p in procedure_table.values() {
        println!("{} {:?} {:?}", p.name(), p.get_intypes(), p.get_outtypes());
        for s in p.get_statements() {
            println!("  {}", s);
        }
        for (i, b) in p.get_blocks().iter().enumerate() {
            println!("  Basic block {} begins at statement {} and has length {}", i, b.start, b.length);
            println!("    Unresolved inputs are: {:?}", b.entry_stack);
            println!("    Unresolved outputs are: {:?}", b.exit_stack);
            println!("    Constraints are: {:?}", b.constraints);
        }
    }

    // generally speaking we try to continue through and give as many errors as possible to
    // inform the developer
    // but emitting code that has produced errors is undefined behavior
    // so exit before emission if errors were produced
    if logger.has_error() {
        println!("compilation failed with errors");
        return
    }

    // emit code
    unimplemented!()
}
