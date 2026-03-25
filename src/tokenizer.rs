use std::fmt::Display;

#[derive(PartialEq, Debug, Clone)]
pub enum TokenKind {
    Unknown,
    Eof,

    Const,
    Push,
    Pop,
    Dup,
    Swap,
    Add,
    Sub,
    Div,
    Mult,
    Mod,
    Inc,
    Dec,
    And,
    Or,
    Xor,
    Bsl,
    Bsr,
    Rol,
    Ror,
    Eq,
    Neq,
    Lt,
    Leq,
    Gt,
    Geq,
    Load,
    Store,
    Label,
    Jump,
    Jumpif,
    Call,
    Ret,
    Cast,
    Conv,

    // these are all literals
    // tokenizer does not parse them, only extract
    Name(String),
    Decimal(String),
    Hex(String),
    Bin(String),
    Float(String),

    // these are types
    // payload denotes width
    I(u8),
    U(u8),
    F(u8),
    Ptr
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    line: usize,
    col: usize,
}
impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {}:{}", self.kind, self.line, self.col)
    }
}

#[derive(Debug)]
pub struct Tokenizer<'a> {
    source: &'a str,
    pos: usize,
    line: usize,
    col: usize
}

impl<'a> Tokenizer<'a> {
    pub fn new(source: &'a str) -> Self {
        Tokenizer { source, pos: 0, line: 1, col: 1 }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.consume_whitespace();

            let t = self.consume_token();
            tokens.push(t);       
            if tokens.last().unwrap().kind == TokenKind::Eof {
                break;
            }
        }
        tokens
    }

    fn consume_token(&mut self) -> Token {
        // first make sure we're not at eof
        if self.peek().is_none() {
            return self.construct_token(TokenKind::Eof)
        }
        // then find out how long the next word is
        let mut tok_len: usize = 1;
        loop {
            if self.peek_ahead(tok_len).is_none_or(|c| c.is_whitespace()) {
                break;
            }
            tok_len += 1;
        }
        // then match that word to a keyword, or turn it into a literal
        let kind = match self.peek_word(tok_len) {
            "const" => TokenKind::Const,
            "push" => TokenKind::Push,
            "pop" => TokenKind::Pop,
            "dup" => TokenKind::Dup,
            "swap" => TokenKind::Swap,
            "add" => TokenKind::Add,
            "sub" => TokenKind::Sub,
            "div" => TokenKind::Div,
            "mult" => TokenKind::Mult,
            "mod" => TokenKind::Mod,
            "inc" => TokenKind::Inc,
            "dec" => TokenKind::Dec,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "xor" => TokenKind::Xor,
            "bsl" => TokenKind::Bsl,
            "bsr" => TokenKind::Bsr,
            "rol" => TokenKind::Rol,
            "ror" => TokenKind::Ror,
            "eq" => TokenKind::Eq,
            "neq" => TokenKind::Neq,
            "lt" => TokenKind::Lt,
            "leq" => TokenKind::Leq,
            "gt" => TokenKind::Gt,
            "geq" => TokenKind::Geq,
            "load" => TokenKind::Load,
            "store" => TokenKind::Store,
            "label" => TokenKind::Label,
            "jump" => TokenKind::Jump,
            "jumpif" => TokenKind::Jumpif,
            "call" => TokenKind::Call,
            "ret" => TokenKind::Ret,
            "cast" => TokenKind::Cast,
            "conv" => TokenKind::Conv,
            "ptr" => TokenKind::Ptr,
            "i8" => TokenKind::I(8),
            "i16" => TokenKind::I(16),
            "i32" => TokenKind::I(32),
            "i64" => TokenKind::I(64),
            "u8" => TokenKind::U(8),
            "u16" => TokenKind::U(16),
            "u32" => TokenKind::U(32),
            "u64" => TokenKind::U(64),
            "f16" => TokenKind::F(16),
            "f32" => TokenKind::F(32),
            "f64" => TokenKind::F(64),
            w => {
                let t = self.construct_name_or_literal(w);
                self.advance_times(tok_len);
                return t;
            }
        };
        // have to construct before advance here so the positions are correct
        let t = self.construct_token(kind);
        self.advance_times(tok_len);
        t
    }

    fn construct_token(&self, kind: TokenKind) -> Token {
        Token { kind, line: self.line, col: self.col }
    }

    fn construct_name_or_literal(&self, word: &str) -> Token {
        // literals all start with - or any digit
        // so if the first letter of the word is alphabetical, it is a name
        let first = word.chars().next();
        if first.is_some_and(|c| c.is_alphabetic() || c == '_') {
            return self.construct_token(TokenKind::Name(word.to_string()));
        }
        // if the first letter is not numeric, we don't know what this token is
        else if first.is_some_and(|c| !c.is_numeric() && c != '-' ) {
            return self.construct_token(TokenKind::Unknown);
        }
        // otherwise, it is a number
        // the tokenizer doesn't actually parse numbers
        // but tokens are distinguished based on the type of number they are
        let kind = match word.get(0..2) {
            Some("0x") => TokenKind::Hex(word[2..].to_string()),
            Some("0b") => TokenKind::Bin(word[2..].to_string()),
            Some("0f") => TokenKind::Float(word[2..].to_string()),
            _ => TokenKind::Decimal(word.to_string())
        };
        // this does allow for bad numbers like 0b1234
        // but it is the parser's job to worry about those
        self.construct_token(kind)
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn peek_ahead(&self, skip: usize) -> Option<char> {
        self.source[self.pos + skip..].chars().next()
    }

    fn peek_word(&self, len: usize) -> &str {
        &self.source[self.pos..self.pos + len]
    }

    fn advance(&mut self) {
        let c = self.peek();
        match c {
            Some('\n') => {
                // if advancing past a newline, reset column and increment line
                self.line += 1;
                self.col = 1;
                self.pos += 1;
            }
            Some(_) => {
                // otherwise just increment column
                self.col += 1;
                self.pos += 1;
            }
            // if no more characters, don't do anything
            _ => {}
        }
    }

    fn advance_times(&mut self, times: usize) {
        for _ in 0..times {
            self.advance();
        }
    }

    fn consume_whitespace(&mut self) {
        loop {
            // consume as much whitespace as we can
            while self.peek().is_some_and(|c| c.is_whitespace()) {
                self.advance()
            }
            // then try to consume a //
            if let Some('/') = self.peek() && let Some('/') = self.peek_ahead(1) {
                self.advance();
                self.advance();
                // then go till newline or end
                while self.peek().is_some_and(|c| c != '\n') {
                    self.advance();
                }
                // this leaves us exactly on the \n, if there was one
                // but that's ok because we loop back up and advance past it before break
            }
            else {
                break;
            }
        }
    }
}

