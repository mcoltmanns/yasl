pub mod tokenizer;
pub mod parser;
pub mod blocker;

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
    }

    pub struct StdoutLogger;
    impl Logger for StdoutLogger {
        fn log(&mut self, event: LogEvent) {
            match event.kind {
                EventKind::Error => println!("error at {}:{}: {}", event.line, event.col, event.msg),
                EventKind::Warning => println!("warning at {}:{}: {}", event.line, event.col, event.msg),
                EventKind::Info => println!("info: {}", event.msg),
            }
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
