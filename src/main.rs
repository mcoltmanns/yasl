use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use std::process::exit;
use petgraph::{Direction, EdgeDirection};
use petgraph::visit::IntoNeighborsDirected;
use yasl::{parser, tokenizer};
use yasl::irs::stack::procedure::StackProcedure;
use yasl::logger::{LogEvent, Logbook};
use yasl::primitives::BasicBlock;
use yasl::util::Positioned;
use yasl::util::source::SourceMap;

fn main() {
    println!("This is yasl {}", env!("CARGO_PKG_VERSION"));

    let mut logger = Logbook::default();

    let args: Vec<String> = env::args().collect();
    let src_name = &args[1];

    let mut src_map = SourceMap::default();
    let src_id = src_map.add(PathBuf::from(src_name));
    let src_file = src_map.get_file(src_id);
    let src_string = src_file.get_contents().unwrap();

    // tokenize the input
    let tokens = tokenizer::tokenize(src_string, src_id);

    let bail = |logger: &mut Logbook| -> ! {
        logger.flush(&src_map);
        exit(1)
    };

    // parse the input
    let mut stackprogram = parser::parse(tokens, &mut logger).unwrap_or_else(
        |e| {
            logger.write(e);
            bail(&mut logger)
        });

    // for each procedure, build its flow graph and check flow
    for proc in stackprogram.get_procedures_mut() {
        // build the flow graph and get all the blocks in the procedure
        let all_set: HashSet<BasicBlock> = proc.build_flow_graph().unwrap_or_else(
            |e| {
                logger.write(e);
                bail(&mut logger);
            }
        ).node_weights().cloned().collect();
        // get the blocks that are reachable
        let reachable_set: HashSet<BasicBlock> = proc.reachable_blocks().iter().cloned().collect();
        // then blocks which are in all but not in reachable are dead code
        for block in all_set.difference(&reachable_set) {
            let first_st_pos = proc.statements_in_block(*block)[0].pos().clone();
            logger.write(LogEvent::warning("unreachable code", first_st_pos));
        }
    }

    // then check flow between procedures
    // build flow graph
    let all_proc_is: HashSet<usize> = stackprogram.build_flow_graph().cloned().unwrap_or_else(
        |e| {
            logger.write(e);
            bail(&mut logger);
        }
    ).node_weights().cloned().collect();
    let reachable_proc_is: HashSet<usize> = stackprogram.reachable_procedure_is().iter().cloned().collect();
    for proc_i in all_proc_is.difference(&reachable_proc_is) {
        let proc_pos = stackprogram.get_procedures()[*proc_i].pos().clone();
        logger.write(LogEvent::warning("unreachable code", proc_pos));
    }
    logger.flush(&src_map);

    // now we are ready to typecheck!

    // for every procedure, link the blocks and build the jump table
    // make sure all blocks are reachable

    /*
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
