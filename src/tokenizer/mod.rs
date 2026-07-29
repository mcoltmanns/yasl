pub mod datastructures;

use datastructures::*;
use crate::util::{FilePos, Positioned};
use crate::util::source::FileId;

/// Implementation of the tokenizer
#[derive(Debug)]
struct Tokenizer {
    /// File id of the source string
    source_id: FileId,
    /// Source string
    source: String,
    /// Current index into the source string
    pos: usize,
    /// Current line in the source string
    line: usize,
    /// Current character in the current line in the source string
    col: usize
}
impl Tokenizer {
    /// Initialize a Tokenizer over a given source string
    fn new(source_id: FileId, source: String) -> Self {
        Tokenizer { source_id, source, pos: 0, line: 1, col: 1 }
    }

    /// Tokenize this Tokenizer's string
    fn run(&mut self) -> Vec<Positioned<Token>> {
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

    /// Construct and consume the next token in the source string
    fn consume_token(&mut self) -> Positioned<Token> {
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
        let tok = Positioned::new(Token::from(tok_src), FilePos::new(self.source_id, self.line, self.col));
        self.advance_times(tok_len);
        tok
    }

    /// Get the next character in the source string
    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    /// Get the next character in the source string, skipping n characters
    fn peek_ahead(&self, skip: usize) -> Option<char> {
        self.source[self.pos + skip..].chars().next()
    }

    /// Get the next word (sequence of n chars) in the source string
    fn peek_word(&self, len: usize) -> &str {
        &self.source[self.pos..self.pos + len]
    }

    /// Advance the cursor one position in the source string, updating line/col/pos as you go
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

    /// Advance n times
    fn advance_times(&mut self, times: usize) {
        for _ in 0..times {
            self.advance();
        }
    }

    /// Advance until you encounter a non-whitespace character
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

/// Generate a token array from a source string.
/// Tokens are positioned.
pub fn tokenize(source: String, source_id: FileId) -> Vec<Positioned<Token>> {
    let mut tokenizer = Tokenizer::new(source_id, source);
    tokenizer.run()
}
