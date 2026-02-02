use strata_ast::ast::{Block, Expr, Item, Lit, Stmt};
use strata_parse::parse_str;

/// Helper: parse a let statement and return its value expression
fn parse_expr_only(src: &str) -> Expr {
    let m = parse_str("<mem>", &format!("let x = {src};")).expect("parse ok");
    let Item::Let(ld) = &m.items[0] else {
        panic!("expected Let declaration");
    };
    ld.value.clone()
}

/// Helper: parse a function and return its body block
fn parse_fn_body(src: &str) -> Block {
    let m = parse_str("<mem>", src).expect("parse ok");
    let Item::Fn(fd) = &m.items[0] else {
        panic!("expected Fn declaration");
    };
    fd.body.clone()
}

// ============ Block tests ============

#[test]
fn block_with_tail_expression() {
    let e = parse_expr_only("{ 1 + 2 }");
    let Expr::Block(block) = e else {
        panic!("expected Block");
    };
    assert!(block.stmts.is_empty());
    assert!(block.tail.is_some());
    let tail = block.tail.unwrap();
    assert!(matches!(*tail, Expr::Binary { .. }));
}

#[test]
fn block_without_tail_trailing_semicolon() {
    let e = parse_expr_only("{ 1 + 2; }");
    let Expr::Block(block) = e else {
        panic!("expected Block");
    };
    assert_eq!(block.stmts.len(), 1);
    assert!(matches!(&block.stmts[0], Stmt::Expr { .. }));
    assert!(block.tail.is_none());
}

#[test]
fn block_with_statements_and_tail() {
    let e = parse_expr_only("{ let x = 1; let y = 2; x + y }");
    let Expr::Block(block) = e else {
        panic!("expected Block");
    };
    assert_eq!(block.stmts.len(), 2);
    assert!(matches!(&block.stmts[0], Stmt::Let { .. }));
    assert!(matches!(&block.stmts[1], Stmt::Let { .. }));
    assert!(block.tail.is_some());
}

#[test]
fn block_with_statements_no_tail() {
    let e = parse_expr_only("{ let x = 1; let y = 2; x + y; }");
    let Expr::Block(block) = e else {
        panic!("expected Block");
    };
    assert_eq!(block.stmts.len(), 3);
    assert!(block.tail.is_none());
}

#[test]
fn empty_block() {
    let e = parse_expr_only("{ }");
    let Expr::Block(block) = e else {
        panic!("expected Block");
    };
    assert!(block.stmts.is_empty());
    assert!(block.tail.is_none());
}

// ============ Let mut tests ============

#[test]
fn let_mut_statement() {
    let e = parse_expr_only("{ let mut x = 1; x }");
    let Expr::Block(block) = e else {
        panic!("expected Block");
    };
    assert_eq!(block.stmts.len(), 1);
    let Stmt::Let { mutable, name, .. } = &block.stmts[0] else {
        panic!("expected Let statement");
    };
    assert!(*mutable);
    assert_eq!(name.text, "x");
}

#[test]
fn let_immutable_statement() {
    let e = parse_expr_only("{ let x = 1; x }");
    let Expr::Block(block) = e else {
        panic!("expected Block");
    };
    assert_eq!(block.stmts.len(), 1);
    let Stmt::Let { mutable, name, .. } = &block.stmts[0] else {
        panic!("expected Let statement");
    };
    assert!(!*mutable);
    assert_eq!(name.text, "x");
}

#[test]
fn let_mut_with_type_annotation() {
    let e = parse_expr_only("{ let mut x: Int = 1; x }");
    let Expr::Block(block) = e else {
        panic!("expected Block");
    };
    let Stmt::Let {
        mutable, name, ty, ..
    } = &block.stmts[0]
    else {
        panic!("expected Let statement");
    };
    assert!(*mutable);
    assert_eq!(name.text, "x");
    assert!(ty.is_some());
}

// ============ Assignment tests ============

#[test]
fn assignment_statement() {
    let e = parse_expr_only("{ let mut x = 1; x = 2; x }");
    let Expr::Block(block) = e else {
        panic!("expected Block");
    };
    assert_eq!(block.stmts.len(), 2);

    let Stmt::Assign { target, value, .. } = &block.stmts[1] else {
        panic!("expected Assign statement");
    };
    assert_eq!(target.text, "x");
    assert!(matches!(value, Expr::Lit(Lit::Int(2), _)));
}

// ============ Return tests ============

#[test]
fn return_with_value() {
    let body = parse_fn_body("fn foo() { return 42; }");
    assert_eq!(body.stmts.len(), 1);
    let Stmt::Return { value, .. } = &body.stmts[0] else {
        panic!("expected Return statement");
    };
    assert!(value.is_some());
    let v = value.as_ref().unwrap();
    assert!(matches!(v, Expr::Lit(Lit::Int(42), _)));
}

#[test]
fn return_without_value() {
    let body = parse_fn_body("fn foo() { return; }");
    assert_eq!(body.stmts.len(), 1);
    let Stmt::Return { value, .. } = &body.stmts[0] else {
        panic!("expected Return statement");
    };
    assert!(value.is_none());
}

#[test]
fn return_in_middle_of_block() {
    let body = parse_fn_body("fn foo() { let x = 1; return x; let y = 2; }");
    assert_eq!(body.stmts.len(), 3);
    assert!(matches!(&body.stmts[0], Stmt::Let { .. }));
    assert!(matches!(&body.stmts[1], Stmt::Return { .. }));
    assert!(matches!(&body.stmts[2], Stmt::Let { .. }));
}

// ============ If expression tests ============

#[test]
fn if_without_else() {
    let e = parse_expr_only("if true { 1 }");
    let Expr::If {
        cond, then_, else_, ..
    } = e
    else {
        panic!("expected If");
    };
    assert!(matches!(*cond, Expr::Lit(Lit::Bool(true), _)));
    assert!(then_.tail.is_some());
    assert!(else_.is_none());
}

#[test]
fn if_else() {
    let e = parse_expr_only("if true { 1 } else { 2 }");
    let Expr::If {
        cond, then_, else_, ..
    } = e
    else {
        panic!("expected If");
    };
    assert!(matches!(*cond, Expr::Lit(Lit::Bool(true), _)));
    assert!(then_.tail.is_some());
    assert!(else_.is_some());

    let else_expr = else_.unwrap();
    let Expr::Block(else_block) = *else_expr else {
        panic!("expected Block in else");
    };
    assert!(else_block.tail.is_some());
}

#[test]
fn else_if_chain() {
    let e = parse_expr_only("if true { 1 } else if false { 2 } else { 3 }");
    let Expr::If {
        cond, then_, else_, ..
    } = e
    else {
        panic!("expected If");
    };

    assert!(matches!(*cond, Expr::Lit(Lit::Bool(true), _)));
    assert!(then_.tail.is_some());

    // else_ should be another If expression
    let else_expr = else_.expect("expected else");
    let Expr::If {
        cond: cond2,
        then_: then2,
        else_: else2,
        ..
    } = *else_expr
    else {
        panic!("expected nested If in else");
    };

    assert!(matches!(*cond2, Expr::Lit(Lit::Bool(false), _)));
    assert!(then2.tail.is_some());

    // Final else block
    let else2_expr = else2.expect("expected final else");
    assert!(matches!(*else2_expr, Expr::Block(_)));
}

#[test]
fn if_with_complex_condition() {
    let e = parse_expr_only("if x > 5 && y < 10 { 1 } else { 2 }");
    let Expr::If { cond, .. } = e else {
        panic!("expected If");
    };
    assert!(matches!(*cond, Expr::Binary { .. }));
}

#[test]
fn if_with_block_bodies() {
    let e = parse_expr_only("if true { let x = 1; x } else { let y = 2; y }");
    let Expr::If { then_, else_, .. } = e else {
        panic!("expected If");
    };

    assert_eq!(then_.stmts.len(), 1);
    assert!(then_.tail.is_some());

    let else_expr = else_.unwrap();
    let Expr::Block(else_block) = *else_expr else {
        panic!("expected Block in else");
    };
    assert_eq!(else_block.stmts.len(), 1);
    assert!(else_block.tail.is_some());
}

// ============ While loop tests ============

#[test]
fn while_loop_basic() {
    let e = parse_expr_only("while true { 1 }");
    let Expr::While { cond, body, .. } = e else {
        panic!("expected While");
    };
    assert!(matches!(*cond, Expr::Lit(Lit::Bool(true), _)));
    assert!(body.tail.is_some());
}

#[test]
fn while_loop_with_complex_condition() {
    let e = parse_expr_only("while x < 10 { x }");
    let Expr::While { cond, .. } = e else {
        panic!("expected While");
    };
    assert!(matches!(*cond, Expr::Binary { .. }));
}

#[test]
fn while_loop_with_statements() {
    let e = parse_expr_only("while true { let x = 1; x = x + 1; }");
    let Expr::While { body, .. } = e else {
        panic!("expected While");
    };
    assert_eq!(body.stmts.len(), 2);
    assert!(body.tail.is_none());
}

// ============ Function body tests ============

#[test]
fn fn_body_with_statements_and_tail() {
    let body = parse_fn_body("fn add(a: Int, b: Int) -> Int { let sum = a + b; sum }");
    assert_eq!(body.stmts.len(), 1);
    assert!(matches!(&body.stmts[0], Stmt::Let { .. }));
    assert!(body.tail.is_some());
}

#[test]
fn fn_body_with_early_return() {
    let body = parse_fn_body("fn max(a: Int, b: Int) -> Int { if a > b { return a; }; b }");
    assert_eq!(body.stmts.len(), 1);
    assert!(body.tail.is_some());
}

// ============ Nested structures ============

#[test]
fn nested_blocks() {
    let e = parse_expr_only("{ { { 1 } } }");
    let Expr::Block(outer) = e else {
        panic!("expected Block");
    };
    assert!(outer.stmts.is_empty());
    let tail = outer.tail.unwrap();
    let Expr::Block(middle) = *tail else {
        panic!("expected nested Block");
    };
    let tail2 = middle.tail.unwrap();
    let Expr::Block(inner) = *tail2 else {
        panic!("expected nested Block");
    };
    assert!(matches!(*inner.tail.unwrap(), Expr::Lit(Lit::Int(1), _)));
}

#[test]
fn if_inside_while() {
    let e = parse_expr_only("while true { if false { 1 } else { 2 } }");
    let Expr::While { body, .. } = e else {
        panic!("expected While");
    };
    let tail = body.tail.unwrap();
    assert!(matches!(*tail, Expr::If { .. }));
}

#[test]
fn while_inside_if() {
    let e = parse_expr_only("if true { while false { 1 } }");
    let Expr::If { then_, .. } = e else {
        panic!("expected If");
    };
    let tail = then_.tail.unwrap();
    assert!(matches!(*tail, Expr::While { .. }));
}
