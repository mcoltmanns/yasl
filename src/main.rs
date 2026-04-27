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

    let mut logger = logger::StdoutLogger;

    let src_path = std::path::Path::new("./test.yas");
    let src_string = match fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(err) => {
            panic!("Could not open file: {err:?}")
        }
    };

    let mut tokenizer = tokenizer::Tokenizer::new(&src_string);
    let tokens = tokenizer.run();

    let mut parser = parser::Parser::new(tokens, &mut logger);
    parser.parse_tokens();

    //for s in &parser.statements {
    //    println!("{}", s);
    //}

    // now we have the statements, and they're at least syntactically valid
    // we can do the first pass to build the procedure table
    // procedure table maps procedure names to procedures
    println!("building procedure table");
    let mut procedure_table = procedure::ProcedureTable::new();

    let mut current_proc: Option<Procedure> = None;
    let mut current_statements = vec![];
    for s in &parser.statements {
        match s.kind() {
            statement::StatementKind::Proc { name , t_in, t_out } => {
                if let Some(mut prev_proc) = current_proc {
                    prev_proc.set_statements(current_statements);
                    current_statements = vec![];
                    procedure_table.insert(prev_proc.name().clone(), prev_proc);
                }
                if procedure_table.contains_key(name) {
                    logger.error(format!("procedure '{}' defined twice", name), s.pos().line, s.pos().col);
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
        procedure_table.insert(prev_proc.name().to_string(), prev_proc);
    }

    if !procedure_table.contains_key("main") {
        logger.error("no main procedure defined".to_string(), 0, 0);
    }

    for p in procedure_table.values() {
        println!("{} {:?} {:?}", p.name(), p.get_intypes(), p.get_outtypes());
        for s in p.statements() {
            println!("  {}", s);
        }
    }

    // now the procedure table is built, and we know input/output types
    // now for each procedure we can do type/jump checking
    // the procedures themselves handle this, we just need to call it on each one
    // the first pass over a procedure builds the jump table and blocks it out
    // the second pass does typechecking
}
