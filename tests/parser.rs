use yasl::{logger, parser::*, tokenizer::*};

fn parse(src: &str) -> Vec<Statement> {
    let mut logger = logger::TestLogger { errors: vec![], warnings: vec![] };
    let tokens = tokenize_program(src);
    parse_program(&tokens, &mut logger)
}

fn parse_errs(src: &str) -> Vec<String> {
    let mut logger = logger::TestLogger { errors: vec![], warnings: vec![] };
    let tokens = tokenize_program(src);
    parse_program(&tokens, &mut logger);
    logger.errors
}

// --- label ---

#[test]
fn label() {
    let stmts = parse("label main");
    assert_eq!(stmts, vec![Statement::Label { name: "main".to_string() }]);
}

#[test]
fn label_no_name() {
    assert!(parse_errs("label").len() == 1);
}

// --- implicit statements ---

#[test]
fn implicit_statements() {
    let stmts = parse("add\nsub\ndup\nswap\nret");
    assert_eq!(stmts, vec![
        Statement::Add,
        Statement::Sub,
        Statement::Dup,
        Statement::Swap,
        Statement::Ret,
    ]);
}

// --- jump / call ---

#[test]
fn jump() {
    let stmts = parse("jump foo");
    assert_eq!(stmts, vec![Statement::Jump { dest: "foo".to_string() }]);
}

#[test]
fn jumpif() {
    let stmts = parse("jumpif foo");
    assert_eq!(stmts, vec![Statement::Jumpif { dest: "foo".to_string() }]);
}

#[test]
fn call() {
    let stmts = parse("call foo");
    assert_eq!(stmts, vec![Statement::Call { dest: "foo".to_string() }]);
}

// --- comments ---

#[test]
fn comments_ignored() {
    let stmts = parse("// this is a comment\nlabel main");
    assert_eq!(stmts, vec![Statement::Label { name: "main".to_string() }]);
}

#[test]
fn inline_comment() {
    let stmts = parse("label main // entry point");
    assert_eq!(stmts, vec![Statement::Label { name: "main".to_string() }]);
}
