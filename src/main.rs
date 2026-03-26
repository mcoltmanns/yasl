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

    let tokens = tokenizer::tokenize_program(&src_string);

    let parsed = parser::parse_program(&tokens, &mut logger);

    for s in &parsed {
        println!("{:?}", s);
    }
}
