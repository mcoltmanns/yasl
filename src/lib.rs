pub mod tokenizer;
pub mod parser;

pub mod logger {
    pub trait Logger {
        fn error(&mut self, msg: &str, line: usize, col: usize);
        fn warn(&mut self, msg: &str, line: usize, col: usize);
        fn info(&mut self, msg: &str);
    }

    pub struct StdoutLogger;
    impl Logger for StdoutLogger {
        fn error(&mut self, msg: &str, line: usize, col: usize) {
            println!("error at {}:{}: {}", line, col, msg);
        }
        fn warn(&mut self, msg: &str, line: usize, col: usize) {
            println!("warning at {}:{}: {}", line, col, msg);
        }
        fn info(&mut self, msg: &str) {
            println!("info: {}", msg);
        }
    }

    pub struct TestLogger {
        pub errors: Vec<String>,
        pub warnings: Vec<String>,
    }
    impl Logger for TestLogger {
        fn error(&mut self, msg: &str, line: usize, col: usize) {
            self.errors.push(format!("{}:{} {}", line, col, msg));
        }
        fn warn(&mut self, msg: &str, line: usize, col: usize) {
            self.warnings.push(format!("{}:{} {}", line, col, msg));
        }
        fn info(&mut self, _msg: &str) {}
    }
}
