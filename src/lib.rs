pub mod tokenizer;
pub mod parser;
pub mod datastructures;
pub mod regmachine;
pub mod target;

// TODO at this point maybe move the logger to its own file, it's looking a little overgrown

pub mod logger {
    use crate::util::FilePos;

    pub enum EventKind {
        Error,
        Warning,
        Info
    }

    pub struct LogEvent {
        pub kind: EventKind,
        pub msg: String,
        pub file: String,
        pub line: usize,
        pub col: usize
    }

    pub trait Logger {
        fn log(&mut self, event: LogEvent);

        fn error(&mut self, msg: &str, pos: FilePos) {
            self.log(LogEvent { kind: EventKind::Error, msg: msg.to_string(), file: pos.name, line: pos.line, col: pos.col });
        }

        fn warning(&mut self, msg: &str, pos: FilePos) {
            self.log(LogEvent { kind: EventKind::Warning, msg: msg.to_string(), file: pos.name, line: pos.line, col: pos.col });
        }

        fn info(&mut self, msg: &str) {
            self.log(LogEvent { kind: EventKind::Info, msg: msg.to_string(), file: "INFO".to_string(), line: 0, col: 0 });
        }

        fn has_error(&self) -> bool;

        fn has_warning(&self) -> bool;
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
                    println!("error at {}:{}:{}: {}", event.file, event.line, event.col, event.msg);
                    self.errored = true;
                }
                EventKind::Warning => { 
                    println!("warning at {}:{}:{}: {}", event.file, event.line, event.col, event.msg);
                    self.warned = true;
                }
                EventKind::Info => println!("info: {}", event.msg),
            }
        }

        fn has_error(&self) -> bool {
            self.errored
        }

        fn has_warning(&self) -> bool {
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
                EventKind::Error => self.errors.push(format!("error at {}:{}:{}: {}", event.file, event.line, event.col, event.msg)),
                EventKind::Warning => self.warnings.push(format!("warning at {}:{}:{}: {}", event.file, event.line, event.col, event.msg)),
                EventKind::Info => {}
            }
        }

        fn has_error(&self) -> bool {
            !self.errors.is_empty()
        }

        fn has_warning(&self) -> bool {
            !self.warnings.is_empty()
        }
    }
}

// TODO this probably doesn't need to be its own module
pub mod util {
    use std::fmt::Display;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FilePos {
        pub name: String,
        pub line: usize,
        pub col: usize
    }
    impl FilePos {
        pub fn new(name: &str, line: usize, col: usize) -> Self {
            FilePos { name: name.to_string(), line, col }
        }
    }
    impl Display for FilePos {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} {} {}", self.name, self.line, self.col)
        }
    }

    // this trait is only useful for polymorphism over all positioned things
    // probably only good for error reporting? keep it in anyway, it's not much extra work
    trait Positionable {
        fn pos(&self) -> &FilePos;
        fn line(&self) -> usize;
        fn col(&self) -> usize;
        fn name(&self) -> &String;
    }
    /// Wrapper type for things that have a position in source
    pub struct Positioned<T> {
        value: T,
        pos: FilePos,
    }
    impl<T> Positioned<T> {
        fn new(value: T, pos: FilePos) -> Positioned<T> {
            Positioned { value, pos }
        }
    }
    // implementing deref allows the user to do *Positioned<T> to access T
    impl<T> std::ops::Deref for Positioned<T> {
        type Target = T;
        fn deref(&self) -> &T {
            &self.value
        }
    }
    impl<T> Positionable for Positioned<T> {
        fn pos(&self) -> &FilePos { &self.pos }
        fn line(&self) -> usize { self.pos.line }
        fn col(&self) -> usize { self.pos.col }
        fn name(&self) -> &String { &self.pos.name }
    }
}
