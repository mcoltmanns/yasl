use crate::{datastructures::token::Token, util::FilePos};

#[derive(Debug)]
pub struct Tokenizer {
    source_name: String,
    source: String,
    pos: usize,
    line: usize,
    col: usize
}

impl Tokenizer {
    pub fn new(source_name: String, source: String) -> Self {
        Tokenizer { source_name, source, pos: 0, line: 1, col: 1 }
    }

    pub fn run(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.consume_whitespace();

            let t = self.consume_token();
            let done = t.is_eof();
            tokens.push(t);
            if done { break; }
        }
        tokens
    }

    fn consume_token(&mut self) -> Token {
        // find out how long the next word is
        let mut tok_len: usize = 0;
        loop {
            if self.peek_ahead(tok_len).is_none_or(|c| c.is_whitespace()) {
                break;
            }
            tok_len += 1;
        }
        let tok_src = if tok_len == 0 { "" } else { self.peek_word(tok_len) };
        // grab the slice which contains that word and turn it into a token
        let tok = Token::new(FilePos::new(&self.source_name, self.line, self.col), tok_src);
        self.advance_times(tok_len);
        tok
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

