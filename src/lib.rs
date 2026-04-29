pub mod tokenizer;
pub mod parser;
pub mod statement;
pub mod procedure;
pub mod basicblock;

pub mod logger {
    pub enum EventKind {
        Error,
        Warning,
        Info
    }

    pub struct LogEvent {
        pub kind: EventKind,
        pub msg: String,
        pub line: usize,
        pub col: usize
    }

    pub trait Logger {
        fn log(&mut self, event: LogEvent);

        fn error(&mut self, msg: String, line: usize, col: usize) {
            self.log(LogEvent { kind: EventKind::Error, msg, line, col });
        }

        fn warning(&mut self, msg: String, line: usize, col: usize) {
            self.log(LogEvent { kind: EventKind::Warning, msg, line, col });
        }

        fn info(&mut self, msg: String) {
            self.log(LogEvent { kind: EventKind::Info, msg, line: 0, col: 0 });
        }

        fn has_error(self) -> bool;

        fn has_warning(self) -> bool;
    }

    pub struct StdoutLogger {
        errored: bool,
        warned: bool,
    }
    impl StdoutLogger {
        pub fn new() -> StdoutLogger {
            StdoutLogger { errored: false, warned: false }
        }
    }
    impl Default for StdoutLogger {
        fn default() -> Self {
            Self::new()
        }
    }
    impl Logger for StdoutLogger {
        fn log(&mut self, event: LogEvent) {
            match event.kind {
                EventKind::Error => {
                    println!("error at {}:{}: {}", event.line, event.col, event.msg);
                    self.errored = true;
                }
                EventKind::Warning => { 
                    println!("warning at {}:{}: {}", event.line, event.col, event.msg);
                    self.warned = true;
                }
                EventKind::Info => println!("info: {}", event.msg),
            }
        }

        fn has_error(self) -> bool {
            self.errored
        }

        fn has_warning(self) -> bool {
            self.warned
        }
    }

    pub struct TestLogger {
        pub errors: Vec<String>,
        pub warnings: Vec<String>,
    }
    impl Logger for TestLogger {
        fn log(&mut self, event: LogEvent) {
            match event.kind {
                EventKind::Error => self.errors.push(format!("error at {}:{}: {}", event.line, event.col, event.msg)),
                EventKind::Warning => self.warnings.push(format!("warning at {}:{}: {}", event.line, event.col, event.msg)),
                EventKind::Info => {}
            }
        }

        fn has_error(self) -> bool {
            self.errors.len() > 0
        }

        fn has_warning(self) -> bool {
            self.warnings.len() > 0
        }
    }
}

pub mod util {
    use std::fmt::Display;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FilePos {
        pub _name: String,
        pub line: usize,
        pub col: usize
    }
    impl Display for FilePos {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} {}", self.line, self.col)
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Positioned<T> {
        pub content: T,
        pub pos: FilePos
    }
    // don't impl display! we want users to implement own display traits because this prints super
    // ugly
}
