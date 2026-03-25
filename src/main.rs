use yasl::tokenizer;
use yasl::parser;
use std::fs;

fn main() {
    println!("This is yasl {}", env!("CARGO_PKG_VERSION"));

    let src_path = std::path::Path::new("./test.yas");
    let src_string = match fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(err) => {
            panic!("Could not open file: {err:?}")
        }
    };

    let tokens = tokenizer::tokenize(&src_string);

    for t in &tokens {
        println!("{}", t);
    }

    parser::parse_program(&tokens);

    println!("Done")
}
