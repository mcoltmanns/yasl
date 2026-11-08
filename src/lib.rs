pub mod tokenizer;
pub mod parser;
//pub mod typechecker;
//pub mod blocker;
//pub mod flowchecker;
pub mod primitives;
// TODO at this point maybe move the logger to its own file, it's looking a little overgrown

pub mod logger {
    use crate::util::{FilePos, Positioned};
    use crate::util::source::{SourceMap};
    use colored::Colorize;

    pub enum EventKind {
        /// For critical errors about the program (type mismatch, syntax error, etc)
        Error(String),
        /// For non-critical warnings about the program (excessive register use, etc)
        Warning(String),
        /// For general progress or information (reached x compilation stage, etc)
        Info(String),
        /// For program output (things the user specifically asked for)
        Output(String),
    }
    /// See EventKind for more info about log options.
    pub type LogEvent = Positioned<EventKind>;
    impl LogEvent {
        pub fn error(msg: &str, pos: FilePos) -> Self {
            Positioned::new(EventKind::Error(msg.to_string()), pos).into()
        }

        pub fn warning(msg: &str, pos: FilePos) -> Self {
            Positioned::new(EventKind::Warning(msg.to_string()), pos).into()
        }

        pub fn info(msg: &str) -> Self {
            Positioned::new(EventKind::Info(msg.to_string()), FilePos::default())
        }

        pub fn output(msg: &str) -> Self {
            Positioned::new(EventKind::Output(msg.to_string()), FilePos::default())
        }
    }

    /// Loggers are essentially wrappers around a vector of LogEvents.
    /// They are cheap to create. Prefer creating new loggers in subcontexts and returning them,
    /// then calling logger.merge(subcontext logger) from the parent context/method.
    #[derive(Default)]
    pub struct Logbook {
        events: Vec<LogEvent>,
        error_i: Vec<usize>,
        warning_i: Vec<usize>,
        info_i: Vec<usize>,
        output_i: Vec<usize>,
    }
    impl Logbook {
        pub fn new() -> Self {
            Logbook::default()
        }

        pub fn write(&mut self, event: LogEvent) {
            let i = self.events.len();
            match *event {
                EventKind::Error( .. ) => {
                    self.error_i.push(i)
                }
                EventKind::Warning( .. ) => {
                    self.warning_i.push(i);
                }
                EventKind::Info ( .. ) => {
                    self.info_i.push(i);
                }
                EventKind::Output( .. ) => {
                    self.output_i.push(i);
                }
            }
            self.events.push(event);
        }

        pub fn has_error(&self) -> bool {
            !self.error_i.is_empty()
        }

        pub fn has_warning(&self) -> bool {
            !self.warning_i.is_empty()
        }

        pub fn has_info(&self) -> bool {
            !self.info_i.is_empty()
        }

        pub fn has_output(&self) -> bool {
            !self.output_i.is_empty()
        }

        pub fn flush(&self, source_map: &SourceMap) {
            for event in self.events.iter() {
                let fname_str = source_map.get_file(event.pos().fid).get_path().as_os_str().to_str().unwrap();
                match &**event {
                    EventKind::Error(msg) => {
                        eprintln!("{}", format!("Error at {}:{}:{}: {}", fname_str, event.pos().line, event.pos().col, msg).red())
                    }
                    EventKind::Warning(msg) => {
                        eprintln!("{}", format!("Warning at {}:{}:{}: {}", fname_str, event.pos().line, event.pos().col, msg).yellow())
                    }
                    EventKind::Info(msg) => {
                        eprintln!("{}", msg)
                    }
                    EventKind::Output(msg) => {
                        println!("{}", msg)
                    }
                }
            }
        }

        fn absorb(&mut self, mut other: Self) {
            let offset = self.events.len();
            self.events.append(&mut other.events);
            for (dst, src) in [
                (&mut self.error_i, &mut other.error_i),
                (&mut self.warning_i, &mut other.warning_i),
                (&mut self.info_i, &mut other.info_i),
                (&mut self.output_i, &mut other.output_i)
            ] {
                dst.extend(src.drain(..).map(|i| i + offset));
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

    #[derive(Default, Debug, Clone, PartialEq, Eq)]
    pub struct FilePos {
        pub fid: FileId,
        pub line: usize,
        pub col: usize
    }
    impl FilePos {
        pub fn new(fid: FileId, line: usize, col: usize) -> Self {
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
        value: T,
        pos: FilePos,
    }
    impl<T> Positioned<T> {
        pub fn new(value: T, pos: FilePos) -> Positioned<T> {
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
    // implementing DerefMut allows the user to do *Positioned<T> to access T, if the ref is mutable
    impl<T> std::ops::DerefMut for Positioned<T> {
        fn deref_mut(&mut self) -> &mut T { &mut self.value }
    }
}
