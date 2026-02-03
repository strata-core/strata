//! Tests for correct span tracking in the parser.
//!
//! These tests verify that spans point to the correct token positions,
//! especially after consuming tokens with expect()/bump().
//! Also tests parser security limits.

use strata_parse::parse_str;

/// Test that function spans end at the closing brace, not beyond
#[test]
fn test_fn_span_ends_at_closing_brace() {
    let src = "fn foo() { 42 }";
    // src:  fn foo() { 42 }
    // pos:  0123456789...
    // The fn decl should span from 0 to 15 (exclusive) = position of '}'

    let module = parse_str("<test>", src).expect("parse failed");
    let item = &module.items[0];

    if let strata_ast::ast::Item::Fn(decl) = item {
        assert_eq!(decl.span.start, 0);
        assert_eq!(decl.span.end, 15); // End at position 15 (end of '}')
    } else {
        panic!("Expected Fn item");
    }
}

/// Test that let spans end at the semicolon, not the next token
#[test]
fn test_let_span_ends_at_semicolon() {
    let src = "let x = 1; let y = 2;";
    // pos:  0123456789012345678901
    // First let: "let x = 1;" spans 0..10
    // Second let: "let y = 2;" spans 11..21

    let module = parse_str("<test>", src).expect("parse failed");

    if let strata_ast::ast::Item::Let(decl1) = &module.items[0] {
        assert_eq!(decl1.span.start, 0);
        assert_eq!(decl1.span.end, 10);
    }

    if let strata_ast::ast::Item::Let(decl2) = &module.items[1] {
        assert_eq!(decl2.span.start, 11);
        assert_eq!(decl2.span.end, 21);
    }
}

/// Test that call expressions span from callee to closing paren
#[test]
fn test_call_span_ends_at_rparen() {
    let src = "let x = foo(1, 2);";
    // pos:  0         1
    //       012345678901234567
    // "foo(1, 2)" spans 8..17 (end is exclusive, after ')')

    let module = parse_str("<test>", src).expect("parse failed");

    if let strata_ast::ast::Item::Let(decl) = &module.items[0] {
        if let strata_ast::ast::Expr::Call { span, .. } = &decl.value {
            assert_eq!(span.start, 8); // 'f' in foo
            assert_eq!(span.end, 17); // position after ')'
        } else {
            panic!("Expected Call expression");
        }
    }
}

/// Test that if expressions span correctly with and without else
#[test]
fn test_if_span_with_else() {
    let src = "fn f() { if true { 1 } else { 2 } }";
    // The if expression should span from 'if' to '}'

    let module = parse_str("<test>", src).expect("parse failed");

    if let strata_ast::ast::Item::Fn(decl) = &module.items[0] {
        if let Some(tail) = &decl.body.tail {
            if let strata_ast::ast::Expr::If { span, .. } = tail.as_ref() {
                assert_eq!(span.start, 9); // 'i' in if
                assert_eq!(span.end, 33); // end of final '}'
            } else {
                panic!("Expected If expression");
            }
        }
    }
}

/// Test that while loops span correctly
#[test]
fn test_while_span() {
    let src = "fn f() { while true { } }";
    // "while true { }" spans 9..23

    let module = parse_str("<test>", src).expect("parse failed");

    if let strata_ast::ast::Item::Fn(decl) = &module.items[0] {
        if let Some(tail) = &decl.body.tail {
            if let strata_ast::ast::Expr::While { span, .. } = tail.as_ref() {
                assert_eq!(span.start, 9); // 'w' in while
                assert_eq!(span.end, 23); // end of '}'
            } else {
                panic!("Expected While expression");
            }
        }
    }
}

/// Test that return statement spans include the semicolon
#[test]
fn test_return_span_ends_at_semicolon() {
    let src = "fn f() { return 42; }";
    // "return 42;" spans 9..19

    let module = parse_str("<test>", src).expect("parse failed");

    if let strata_ast::ast::Item::Fn(decl) = &module.items[0] {
        if let strata_ast::ast::Stmt::Return { span, .. } = &decl.body.stmts[0] {
            assert_eq!(span.start, 9); // 'r' in return
            assert_eq!(span.end, 19); // end of ';'
        } else {
            panic!("Expected Return statement");
        }
    }
}

/// Test that qualified type paths (A::B::C) parse correctly
#[test]
fn test_parse_qualified_type_path() {
    // Should parse without error (even if type checking rejects it later)
    let src = "let x: Foo::Bar = 1;";
    let module = parse_str("<test>", src).expect("parse failed");

    if let strata_ast::ast::Item::Let(decl) = &module.items[0] {
        let ty = decl.ty.as_ref().expect("expected type annotation");
        if let strata_ast::ast::TypeExpr::Path(segs, _) = ty {
            assert_eq!(segs.len(), 2);
            assert_eq!(segs[0].text, "Foo");
            assert_eq!(segs[1].text, "Bar");
        } else {
            panic!("Expected Path type expression");
        }
    } else {
        panic!("Expected Let item");
    }
}

/// Test that triple-qualified type paths parse correctly
#[test]
fn test_parse_triple_qualified_type_path() {
    let src = "let x: A::B::C = 1;";
    let module = parse_str("<test>", src).expect("parse failed");

    if let strata_ast::ast::Item::Let(decl) = &module.items[0] {
        let ty = decl.ty.as_ref().expect("expected type annotation");
        if let strata_ast::ast::TypeExpr::Path(segs, _) = ty {
            assert_eq!(segs.len(), 3);
            assert_eq!(segs[0].text, "A");
            assert_eq!(segs[1].text, "B");
            assert_eq!(segs[2].text, "C");
        } else {
            panic!("Expected Path type expression");
        }
    } else {
        panic!("Expected Let item");
    }
}
