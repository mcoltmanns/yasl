use yasl::tokenizer::*;

fn tokenize_kinds(source: &str) -> Vec<TokenKind> {
    let mut tokens: Vec<TokenKind> = tokenize(source).iter().map(|t| t.kind.clone()).collect();
    tokens.remove(tokens.len()  - 1);
    tokens
}

// --- Keywords ---

#[test]
fn keyword_push() {
    assert_eq!(tokenize_kinds("push"), vec![TokenKind::Push]);
}

#[test]
fn keyword_pop() {
    assert_eq!(tokenize_kinds("pop"), vec![TokenKind::Pop]);
}

#[test]
fn keyword_add() {
    assert_eq!(tokenize_kinds("add"), vec![TokenKind::Add]);
}

#[test]
fn multiple_keywords() {
    assert_eq!(tokenize_kinds("push pop dup"), vec![
        TokenKind::Push,
        TokenKind::Pop,
        TokenKind::Dup,
    ]);
}

// --- Types ---

#[test]
fn type_i32() {
    assert_eq!(tokenize_kinds("i32"), vec![TokenKind::I(32)]);
}

#[test]
fn type_f64() {
    assert_eq!(tokenize_kinds("f64"), vec![TokenKind::F(64)]);
}

#[test]
fn type_ptr() {
    assert_eq!(tokenize_kinds("ptr"), vec![TokenKind::Ptr]);
}

// --- Names ---

#[test]
fn name() {
    assert_eq!(tokenize_kinds("my_label"), vec![TokenKind::Name("my_label".to_string())]);
}

#[test]
fn name_with_digits() {
    assert_eq!(tokenize_kinds("loop1"), vec![TokenKind::Name("loop1".to_string())]);
}

#[test]
fn name_underscore_prefix() {
    assert_eq!(tokenize_kinds("_start"), vec![TokenKind::Name("_start".to_string())]);
}

#[test]
fn uppercase_is_name() {
    // uppercase keywords are parsed as names, and are allowed as such
    // up to parser to reject incorrect keyword/name appearance
    assert_eq!(tokenize_kinds("PUSH"), vec![TokenKind::Name("PUSH".to_string())]);
    assert_eq!(tokenize_kinds("Add"), vec![TokenKind::Name("Add".to_string())]);
}

// --- Number literals ---

#[test]
fn integer_literal() {
    assert_eq!(tokenize_kinds("42"), vec![TokenKind::Decimal("42".to_string())]);
}

#[test]
fn negative_integer_literal() {
    assert_eq!(tokenize_kinds("-42"), vec![TokenKind::Decimal("-42".to_string())]);
}

#[test]
fn hex_literal() {
    assert_eq!(tokenize_kinds("0xFF"), vec![TokenKind::Hex("FF".to_string())]);
}

#[test]
fn bin_literal() {
    assert_eq!(tokenize_kinds("0b1010"), vec![TokenKind::Bin("1010".to_string())]);
}

#[test]
fn float_literal() {
    assert_eq!(tokenize_kinds("3.14"), vec![TokenKind::Decimal("3.14".to_string())]);
}

// --- Mixed ---

#[test]
fn keyword_type_literal() {
    assert_eq!(tokenize_kinds("push i32 42"), vec![
        TokenKind::Push,
        TokenKind::I(32),
        TokenKind::Decimal("42".to_string()),
    ]);
}

// --- Comments ---

#[test]
fn comment_produces_no_tokens() {
    assert_eq!(tokenize_kinds("// this is a comment"), vec![]);
}

#[test]
fn comment_does_not_consume_next_line() {
    assert_eq!(tokenize_kinds("// comment\npop"), vec![TokenKind::Pop]);
}

// --- Unknown ---

#[test]
fn unknown_symbol() {
    assert!(tokenize_kinds("@").contains(&TokenKind::Unknown));
    assert!(tokenize_kinds("#").contains(&TokenKind::Unknown));
    assert!(tokenize_kinds("$").contains(&TokenKind::Unknown));
}
