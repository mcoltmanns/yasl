use yasl::tokenizer;
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

    let mut t = tokenizer::Tokenizer::new(&src_string);

    let tokens = t.tokenize();

    for token in &tokens {
        println!("{}", token);
    }

    println!("Done")
}
