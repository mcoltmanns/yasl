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

// --- const ---

#[test]
fn const_i8() {
    let stmts = parse("const x i8 10");
    assert_eq!(stmts, vec![Statement::Const {
        name: "x".to_string(),
        value: Literal { bits: 10, kind: DType::I8 }
    }]);
}

#[test]
fn const_negative() {
    let stmts = parse("const x i8 -1");
    assert_eq!(stmts, vec![Statement::Const {
        name: "x".to_string(),
        value: Literal { bits: -1i8 as u64, kind: DType::I8 }
    }]);
}

#[test]
fn const_hex() {
    let stmts = parse("const x u8 0xFF");
    assert_eq!(stmts, vec![Statement::Const {
        name: "x".to_string(),
        value: Literal { bits: 255, kind: DType::U8 }
    }]);
}

#[test]
fn const_binary() {
    let stmts = parse("const x u8 0b00001111");
    assert_eq!(stmts, vec![Statement::Const {
        name: "x".to_string(),
        value: Literal { bits: 15, kind: DType::U8 }
    }]);
}

#[test]
fn const_overflow() {
    // 128 doesn't fit in i8
    assert!(parse_errs("const x i8 128").len() == 1);
}

#[test]
fn const_followed_by_label() {
    // regression test for the eaten-label bug
    let stmts = parse("const x i8 10\nlabel foo");
    assert_eq!(stmts, vec![
        Statement::Const { name: "x".to_string(), value: Literal { bits: 10, kind: DType::I8 } },
        Statement::Label { name: "foo".to_string() },
    ]);
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

// --- push ---

#[test]
fn push_const_name() {
    let stmts = parse("const x i8 1\npush x");
    assert_eq!(stmts[1], Statement::PushConst { name: "x".to_string() });
}

#[test]
fn push_literal() {
    let stmts = parse("push i32 42");
    assert_eq!(stmts, vec![Statement::PushLiteral {
        value: Literal { bits: 42, kind: DType::I32 }
    }]);
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

// --- full program ---

#[test]
fn fibonacci_program() {
    let src = "
        const ten i8 10
        label main
            push ten
            call fib
            ret
        label fib
            dup
            push i32 1
            gt
            jumpif fib_recurse
            ret
    ";
    let stmts = parse(src);
    assert_eq!(stmts[0], Statement::Const { name: "ten".to_string(), value: Literal { bits: 10, kind: DType::I8 } });
    assert_eq!(stmts[1], Statement::Label { name: "main".to_string() });
    assert_eq!(stmts[2], Statement::PushConst { name: "ten".to_string() });
    assert_eq!(stmts[3], Statement::Call { dest: "fib".to_string() });
    assert_eq!(stmts[4], Statement::Ret);
    assert_eq!(stmts[5], Statement::Label { name: "fib".to_string() });
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
