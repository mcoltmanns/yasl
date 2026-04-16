use yasl::logger;
use yasl::tokenizer;
use yasl::parser;
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

    for s in &parser.statements {
        println!("{}", s);
    }

}
