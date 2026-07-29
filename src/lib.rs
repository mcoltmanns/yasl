pub mod tokenizer;
pub mod parser;
pub mod datastructures;
// TODO at this point maybe move the logger to its own file, it's looking a little overgrown

pub mod logger {
    use crate::util::FilePos;
    use crate::util::source::{FileId, SourceMap};
    use colored::Colorize;

    pub enum EventKind {
        Error,
        Warning,
        Info
    }

    #[derive(Default)]
    pub struct LoggerCore {
        events: Vec<LogEvent>,
        error_i: Vec<usize>,
        warning_i: Vec<usize>,
        info_i: Vec<usize>,
    }
    pub struct LogEvent {
        pub kind: EventKind,
        pub msg: String,
        pub pos: FilePos,
    }

    pub trait Logger {
        fn log(&mut self, event: LogEvent);

        fn error(&mut self, msg: &str, pos: FilePos) {
            self.log(LogEvent { kind: EventKind::Error, msg: msg.to_string(), pos });
        }

        fn warning(&mut self, msg: &str, pos: FilePos) {
            self.log(LogEvent { kind: EventKind::Warning, msg: msg.to_string(), pos });
        }

        fn info(&mut self, msg: &str) {
            self.log(LogEvent { kind: EventKind::Info, msg: msg.to_string(), pos: FilePos { fid: FileId(u32::MAX), line: 0, col: 0 } });
        }

        fn has_error(&self) -> bool;

        fn has_warning(&self) -> bool;

        /// Flush the info in this logger to whatever its output is
        fn flush(&self, source_map: &SourceMap);
    }

    #[derive(Default)]
    pub struct StderrLogger {
        core: LoggerCore,
    }
    impl Logger for StderrLogger {
        fn log(&mut self, event: LogEvent) {
            let i = self.core.events.len();
            match event.kind {
                EventKind::Error => {
                    self.core.error_i.push(i)
                }
                EventKind::Warning => {
                    self.core.warning_i.push(i);
                }
                EventKind::Info => {
                    self.core.info_i.push(i);
                }
            }
            self.core.events.push(event);
        }
        fn has_error(&self) -> bool {
            !self.core.error_i.is_empty()
        }
        fn has_warning(&self) -> bool {
            !self.core.warning_i.is_empty()
        }
        fn flush(&self, source_map: &SourceMap) {
            for event in self.core.events.iter() {
                let fname_str = source_map.get_file(event.pos.fid).get_path().as_os_str().to_str().unwrap();
                match event.kind {
                    EventKind::Error => {
                        eprintln!("{}", format!("Error at {}:{}:{}: {}", fname_str, event.pos.line, event.pos.col, event.msg).red())
                    }
                    EventKind::Warning => {
                        eprintln!("{}", format!("Warning at {}:{}:{}: {}", fname_str, event.pos.line, event.pos.col, event.msg).yellow())
                    }
                    EventKind::Info => {
                        eprintln!("{}", event.msg)
                    }
                }
            }
        }
    }
}
pub mod util {
    use std::fmt::Display;
    use source::FileId;

    pub mod source {
        use std::collections::HashMap;
        use std::path::PathBuf;

        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
        pub struct FileId(pub u32);

        #[derive(Default)]
        pub struct SourceFile {
            id: FileId,
            path: PathBuf,
        }
        impl SourceFile {
            fn new(id: FileId, path: PathBuf) -> SourceFile {
                SourceFile { id, path }
            }
            pub fn get_contents(&self) -> std::io::Result<String> {
                std::fs::read_to_string(&self.path)
            }
            pub fn get_path(&self) -> &PathBuf {
                &self.path
            }
        }

        #[derive(Default)]
        pub struct SourceMap {
            files: Vec<SourceFile>,
            by_path: HashMap<PathBuf, FileId>,
        }
        impl SourceMap {
            pub fn add(&mut self, path: PathBuf) -> FileId {
                if let Some(&id) = self.by_path.get(&path) {
                    return id;
                }
                let id = FileId(self.files.len() as u32);
                self.files.push(SourceFile::new(id, path.clone()));
                self.by_path.insert(path, id);
                id
            }
            pub fn get_file(&self, id: FileId) -> &SourceFile {
                &self.files[id.0 as usize]
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FilePos {
        pub fid: source::FileId,
        pub line: usize,
        pub col: usize
    }
    impl FilePos {
        pub fn new(fid: source::FileId, line: usize, col: usize) -> Self {
            FilePos { fid, line, col }
        }
    }
    impl Display for FilePos {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?} {} {}", self.fid, self.line, self.col)
        }
    }
    
    /// Wrapper type for things that have a position in source
    pub struct Positioned<T> {
        pub value: T,
        pub pos: FilePos,
    }
    impl<T> Positioned<T> {
        pub(crate) fn new(value: T, pos: FilePos) -> Positioned<T> {
            Positioned { value, pos }
        }
        pub fn pos(&self) -> &FilePos { &self.pos }
        pub fn line(&self) -> usize { self.pos.line }
        pub fn col(&self) -> usize { self.pos.col }
        pub fn fid(&self) -> FileId { self.pos.fid }
    }
    // implementing deref allows the user to do *Positioned<T> to access T
    impl<T> std::ops::Deref for Positioned<T> {
        type Target = T;
        fn deref(&self) -> &T {
            &self.value
        }
    }
    
    /// Trait for things which can be named
    pub trait Named {
        fn name(&self) -> &String;
    }
}
