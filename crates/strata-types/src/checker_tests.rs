// crates/strata-types/src/checker_tests.rs
// Comprehensive tests for the type checker

use super::checker::{TypeChecker, TypeError};
use strata_ast::ast::{
    BinOp, Block, Expr, FieldInit, FnDecl, Ident, Item, LetDecl, Lit, MatchArm, Module, Param, Pat,
    PatField, Path, Stmt, TypeExpr, UnOp,
};
use strata_ast::span::Span;

/// Helper to create a dummy span
fn sp() -> Span {
    Span { start: 0, end: 0 }
}

/// Helper to create an identifier
fn ident(name: &str) -> Ident {
    Ident {
        text: name.to_string(),
        span: sp(),
    }
}

/// Helper to create a type annotation
fn ty_int() -> TypeExpr {
    TypeExpr::Path(vec![ident("Int")], sp())
}

fn ty_bool() -> TypeExpr {
    TypeExpr::Path(vec![ident("Bool")], sp())
}

fn ty_string() -> TypeExpr {
    TypeExpr::Path(vec![ident("String")], sp())
}

// ============================================================================
// POSITIVE TESTS - Valid programs that should type check
// ============================================================================

#[test]
fn test_literal_int() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Lit(Lit::Int(42), sp());
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_literal_float() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Lit(Lit::Float(3.5), sp());
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::float());
}

#[test]
fn test_literal_bool() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Lit(Lit::Bool(true), sp());
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::bool_());
}

#[test]
fn test_literal_string() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Lit(Lit::Str("hello".to_string()), sp());
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::string());
}

#[test]
fn test_literal_nil() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Lit(Lit::Nil, sp());
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::unit());
}

#[test]
fn test_int_addition() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
        op: BinOp::Add,
        rhs: Box::new(Expr::Lit(Lit::Int(2), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_float_addition_not_yet_supported() {
    // Float arithmetic is not yet supported - arithmetic is Int-only for now
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Float(1.5), sp())),
        op: BinOp::Add,
        rhs: Box::new(Expr::Lit(Lit::Float(2.5), sp())),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_bool_and() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Bool(true), sp())),
        op: BinOp::And,
        rhs: Box::new(Expr::Lit(Lit::Bool(false), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::bool_());
}

#[test]
fn test_int_comparison() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
        op: BinOp::Lt,
        rhs: Box::new(Expr::Lit(Lit::Int(2), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::bool_());
}

#[test]
fn test_equality() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
        op: BinOp::Eq,
        rhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::bool_());
}

#[test]
fn test_unary_not() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Unary {
        op: UnOp::Not,
        expr: Box::new(Expr::Lit(Lit::Bool(true), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::bool_());
}

#[test]
fn test_unary_neg_int() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Unary {
        op: UnOp::Neg,
        expr: Box::new(Expr::Lit(Lit::Int(42), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_unary_neg_float_not_yet_supported() {
    // Float negation is not yet supported - negation is Int-only for now
    let mut tc = TypeChecker::new();
    let expr = Expr::Unary {
        op: UnOp::Neg,
        expr: Box::new(Expr::Lit(Lit::Float(3.5), sp())),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_parenthesized_expr() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Paren {
        inner: Box::new(Expr::Lit(Lit::Int(42), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_let_without_annotation() {
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Let(LetDecl {
            name: ident("x"),
            ty: None,
            value: Expr::Lit(Lit::Int(42), sp()),
            span: sp(),
        })],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_let_with_matching_annotation() {
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Let(LetDecl {
            name: ident("x"),
            ty: Some(ty_int()),
            value: Expr::Lit(Lit::Int(42), sp()),
            span: sp(),
        })],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_variable_reference() {
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Let(LetDecl {
                name: ident("x"),
                ty: None,
                value: Expr::Lit(Lit::Int(42), sp()),
                span: sp(),
            }),
            Item::Let(LetDecl {
                name: ident("y"),
                ty: None,
                value: Expr::Var(ident("x")),
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_multiple_lets() {
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Let(LetDecl {
                name: ident("x"),
                ty: Some(ty_int()),
                value: Expr::Lit(Lit::Int(1), sp()),
                span: sp(),
            }),
            Item::Let(LetDecl {
                name: ident("y"),
                ty: None,
                value: Expr::Binary {
                    lhs: Box::new(Expr::Lit(Lit::Int(2), sp())),
                    op: BinOp::Add,
                    rhs: Box::new(Expr::Lit(Lit::Int(3), sp())),
                    span: sp(),
                },
                span: sp(),
            }),
            Item::Let(LetDecl {
                name: ident("z"),
                ty: None,
                value: Expr::Binary {
                    lhs: Box::new(Expr::Lit(Lit::Bool(true), sp())),
                    op: BinOp::And,
                    rhs: Box::new(Expr::Lit(Lit::Bool(false), sp())),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

// ============================================================================
// NEGATIVE TESTS - Invalid programs that should fail
// ============================================================================

#[test]
fn test_type_mismatch_int_plus_bool() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
        op: BinOp::Add,
        rhs: Box::new(Expr::Lit(Lit::Bool(true), sp())),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_type_mismatch_string_multiply() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Str("hi".to_string()), sp())),
        op: BinOp::Mul,
        rhs: Box::new(Expr::Lit(Lit::Int(5), sp())),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_type_mismatch_int_float_addition() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
        op: BinOp::Add,
        rhs: Box::new(Expr::Lit(Lit::Float(2.5), sp())),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_annotation_mismatch() {
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Let(LetDecl {
            name: ident("x"),
            ty: Some(ty_bool()),
            value: Expr::Lit(Lit::Int(123), sp()),
            span: sp(),
        })],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_unknown_variable() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Var(ident("unknown"));
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::UnknownVariable { .. }
    ));
}

#[test]
fn test_unary_not_on_int() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Unary {
        op: UnOp::Not,
        expr: Box::new(Expr::Lit(Lit::Int(42), sp())),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_unary_neg_on_bool() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Unary {
        op: UnOp::Neg,
        expr: Box::new(Expr::Lit(Lit::Bool(true), sp())),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_logical_and_on_ints() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
        op: BinOp::And,
        rhs: Box::new(Expr::Lit(Lit::Int(2), sp())),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_comparison_different_types() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
        op: BinOp::Lt,
        rhs: Box::new(Expr::Lit(Lit::Float(2.0), sp())),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_function_call_unknown_function() {
    // Calling an unknown function produces UnknownVariable
    let mut tc = TypeChecker::new();
    let expr = Expr::Call {
        callee: Box::new(Expr::Var(ident("foo"))),
        args: vec![],
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::UnknownVariable { .. }
    ));
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_complex_expression() {
    let mut tc = TypeChecker::new();
    // (1 + 2) * 3
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Paren {
            inner: Box::new(Expr::Binary {
                lhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
                op: BinOp::Add,
                rhs: Box::new(Expr::Lit(Lit::Int(2), sp())),
                span: sp(),
            }),
            span: sp(),
        }),
        op: BinOp::Mul,
        rhs: Box::new(Expr::Lit(Lit::Int(3), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_all_arithmetic_ops() {
    let mut tc = TypeChecker::new();
    for op in [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div] {
        let expr = Expr::Binary {
            lhs: Box::new(Expr::Lit(Lit::Int(10), sp())),
            op,
            rhs: Box::new(Expr::Lit(Lit::Int(5), sp())),
            span: sp(),
        };
        let ty = tc.infer_expr(&expr).unwrap();
        assert_eq!(ty, crate::infer::ty::Ty::int());
    }
}

#[test]
fn test_all_comparison_ops() {
    let mut tc = TypeChecker::new();
    for op in [BinOp::Lt, BinOp::Le, BinOp::Gt, BinOp::Ge] {
        let expr = Expr::Binary {
            lhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
            op,
            rhs: Box::new(Expr::Lit(Lit::Int(2), sp())),
            span: sp(),
        };
        let ty = tc.infer_expr(&expr).unwrap();
        assert_eq!(ty, crate::infer::ty::Ty::bool_());
    }
}

#[test]
fn test_all_equality_ops() {
    let mut tc = TypeChecker::new();
    for op in [BinOp::Eq, BinOp::Ne] {
        let expr = Expr::Binary {
            lhs: Box::new(Expr::Lit(Lit::Bool(true), sp())),
            op,
            rhs: Box::new(Expr::Lit(Lit::Bool(false), sp())),
            span: sp(),
        };
        let ty = tc.infer_expr(&expr).unwrap();
        assert_eq!(ty, crate::infer::ty::Ty::bool_());
    }
}

// ============================================================================
// BLOCK AND CONTROL FLOW TESTS
// ============================================================================

#[test]
fn test_block_tail_type() {
    // { let x = 1; x + 1 } has type Int
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![Stmt::Let {
            mutable: false,
            pat: Pat::Ident(ident("x")),
            ty: None,
            value: Expr::Lit(Lit::Int(1), sp()),
            span: sp(),
        }],
        tail: Some(Box::new(Expr::Binary {
            lhs: Box::new(Expr::Var(ident("x"))),
            op: BinOp::Add,
            rhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
            span: sp(),
        })),
        span: sp(),
    });
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_block_no_tail_unit() {
    // { let x = 1; x + 1; } has type Unit
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![
            Stmt::Let {
                mutable: false,
                pat: Pat::Ident(ident("x")),
                ty: None,
                value: Expr::Lit(Lit::Int(1), sp()),
                span: sp(),
            },
            Stmt::Expr {
                expr: Expr::Binary {
                    lhs: Box::new(Expr::Var(ident("x"))),
                    op: BinOp::Add,
                    rhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
                    span: sp(),
                },
                span: sp(),
            },
        ],
        tail: None,
        span: sp(),
    });
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::unit());
}

#[test]
fn test_if_else_unify() {
    // if true { 1 } else { 2 } has type Int
    let mut tc = TypeChecker::new();
    let expr = Expr::If {
        cond: Box::new(Expr::Lit(Lit::Bool(true), sp())),
        then_: Block {
            stmts: vec![],
            tail: Some(Box::new(Expr::Lit(Lit::Int(1), sp()))),
            span: sp(),
        },
        else_: Some(Box::new(Expr::Block(Block {
            stmts: vec![],
            tail: Some(Box::new(Expr::Lit(Lit::Int(2), sp()))),
            span: sp(),
        }))),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_if_else_mismatch_error() {
    // if true { 1 } else { "str" } should fail
    let mut tc = TypeChecker::new();
    let expr = Expr::If {
        cond: Box::new(Expr::Lit(Lit::Bool(true), sp())),
        then_: Block {
            stmts: vec![],
            tail: Some(Box::new(Expr::Lit(Lit::Int(1), sp()))),
            span: sp(),
        },
        else_: Some(Box::new(Expr::Block(Block {
            stmts: vec![],
            tail: Some(Box::new(Expr::Lit(Lit::Str("str".to_string()), sp()))),
            span: sp(),
        }))),
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_if_no_else_unit() {
    // if true { } is ok (Unit)
    let mut tc = TypeChecker::new();
    let expr = Expr::If {
        cond: Box::new(Expr::Lit(Lit::Bool(true), sp())),
        then_: Block {
            stmts: vec![],
            tail: None,
            span: sp(),
        },
        else_: None,
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::unit());
}

#[test]
fn test_if_no_else_nonunit_error() {
    // if true { 1 } fails (no else, non-Unit then)
    let mut tc = TypeChecker::new();
    let expr = Expr::If {
        cond: Box::new(Expr::Lit(Lit::Bool(true), sp())),
        then_: Block {
            stmts: vec![],
            tail: Some(Box::new(Expr::Lit(Lit::Int(1), sp()))),
            span: sp(),
        },
        else_: None,
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_if_cond_must_be_bool() {
    // if 1 { } fails
    let mut tc = TypeChecker::new();
    let expr = Expr::If {
        cond: Box::new(Expr::Lit(Lit::Int(1), sp())),
        then_: Block {
            stmts: vec![],
            tail: None,
            span: sp(),
        },
        else_: None,
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_while_returns_unit() {
    // while false { 1 } has type Unit
    let mut tc = TypeChecker::new();
    let expr = Expr::While {
        cond: Box::new(Expr::Lit(Lit::Bool(false), sp())),
        body: Block {
            stmts: vec![],
            tail: Some(Box::new(Expr::Lit(Lit::Int(1), sp()))),
            span: sp(),
        },
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::unit());
}

#[test]
fn test_while_cond_must_be_bool() {
    // while 1 { } fails
    let mut tc = TypeChecker::new();
    let expr = Expr::While {
        cond: Box::new(Expr::Lit(Lit::Int(1), sp())),
        body: Block {
            stmts: vec![],
            tail: None,
            span: sp(),
        },
        span: sp(),
    };
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_mutable_assign_ok() {
    // { let mut x = 1; x = 2; x } is ok
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![
            Stmt::Let {
                mutable: true,
                pat: Pat::Ident(ident("x")),
                ty: None,
                value: Expr::Lit(Lit::Int(1), sp()),
                span: sp(),
            },
            Stmt::Assign {
                target: ident("x"),
                value: Expr::Lit(Lit::Int(2), sp()),
                span: sp(),
            },
        ],
        tail: Some(Box::new(Expr::Var(ident("x")))),
        span: sp(),
    });
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_immutable_assign_error() {
    // { let x = 1; x = 2; } fails
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![
            Stmt::Let {
                mutable: false,
                pat: Pat::Ident(ident("x")),
                ty: None,
                value: Expr::Lit(Lit::Int(1), sp()),
                span: sp(),
            },
            Stmt::Assign {
                target: ident("x"),
                value: Expr::Lit(Lit::Int(2), sp()),
                span: sp(),
            },
        ],
        tail: None,
        span: sp(),
    });
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::ImmutableAssignment { .. }
    ));
}

#[test]
fn test_assign_type_mismatch_error() {
    // { let mut x = 1; x = "str"; } fails
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![
            Stmt::Let {
                mutable: true,
                pat: Pat::Ident(ident("x")),
                ty: None,
                value: Expr::Lit(Lit::Int(1), sp()),
                span: sp(),
            },
            Stmt::Assign {
                target: ident("x"),
                value: Expr::Lit(Lit::Str("str".to_string()), sp()),
                span: sp(),
            },
        ],
        tail: None,
        span: sp(),
    });
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

#[test]
fn test_nested_scopes() {
    // { let x = 1; { let x = true; x }; x } - inner x is Bool, outer x is Int
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![
            Stmt::Let {
                mutable: false,
                pat: Pat::Ident(ident("x")),
                ty: None,
                value: Expr::Lit(Lit::Int(1), sp()),
                span: sp(),
            },
            Stmt::Expr {
                expr: Expr::Block(Block {
                    stmts: vec![Stmt::Let {
                        mutable: false,
                        pat: Pat::Ident(ident("x")),
                        ty: None,
                        value: Expr::Lit(Lit::Bool(true), sp()),
                        span: sp(),
                    }],
                    tail: Some(Box::new(Expr::Var(ident("x")))),
                    span: sp(),
                }),
                span: sp(),
            },
        ],
        tail: Some(Box::new(Expr::Var(ident("x")))),
        span: sp(),
    });
    let ty = tc.infer_expr(&expr).unwrap();
    // The outer x is Int
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

// ============================================================================
// ADT REGISTRATION TESTS (Phase 3)
// ============================================================================

use strata_ast::ast::{EnumDef, Field, StructDef, Variant, VariantFields};

/// Helper to create a struct def
fn make_struct(name: &str, type_params: &[&str], fields: Vec<Field>) -> StructDef {
    StructDef {
        name: ident(name),
        type_params: type_params.iter().map(|s| ident(s)).collect(),
        fields,
        span: sp(),
    }
}

/// Helper to create a field
fn make_field(name: &str, ty: TypeExpr) -> Field {
    Field {
        name: ident(name),
        ty,
        span: sp(),
    }
}

/// Helper to create an enum def
fn make_enum(name: &str, type_params: &[&str], variants: Vec<Variant>) -> EnumDef {
    EnumDef {
        name: ident(name),
        type_params: type_params.iter().map(|s| ident(s)).collect(),
        variants,
        span: sp(),
    }
}

/// Helper to create a unit variant
fn make_unit_variant(name: &str) -> Variant {
    Variant {
        name: ident(name),
        fields: VariantFields::Unit,
        span: sp(),
    }
}

/// Helper to create a tuple variant
fn make_tuple_variant(name: &str, fields: Vec<TypeExpr>) -> Variant {
    Variant {
        name: ident(name),
        fields: VariantFields::Tuple(fields),
        span: sp(),
    }
}

/// Helper to create an ADT type reference (e.g., Point or Option<T>)
fn ty_adt(name: &str) -> TypeExpr {
    TypeExpr::Path(vec![ident(name)], sp())
}

/// Helper to create a generic ADT type (e.g., Option<Int>)
fn ty_generic(name: &str, args: Vec<TypeExpr>) -> TypeExpr {
    TypeExpr::App {
        base: vec![ident(name)],
        args,
        span: sp(),
    }
}

#[test]
fn test_adt_register_simple_struct() {
    // struct Point { x: Int, y: Int }
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Struct(make_struct(
            "Point",
            &[],
            vec![make_field("x", ty_int()), make_field("y", ty_int())],
        ))],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
    assert!(tc.adt_registry().contains("Point"));
}

#[test]
fn test_adt_register_generic_struct() {
    // struct Pair<T, U> { first: T, second: U }
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Struct(make_struct(
            "Pair",
            &["T", "U"],
            vec![
                make_field("first", ty_adt("T")),
                make_field("second", ty_adt("U")),
            ],
        ))],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
    let adt = tc.adt_registry().get("Pair").unwrap();
    assert_eq!(adt.arity(), 2);
}

#[test]
fn test_adt_register_simple_enum() {
    // enum Status { Active, Inactive }
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Enum(make_enum(
            "Status",
            &[],
            vec![make_unit_variant("Active"), make_unit_variant("Inactive")],
        ))],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
    assert!(tc.adt_registry().contains("Status"));
}

#[test]
fn test_adt_register_option_enum() {
    // enum Option<T> { Some(T), None }
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Enum(make_enum(
            "Option",
            &["T"],
            vec![
                make_tuple_variant("Some", vec![ty_adt("T")]),
                make_unit_variant("None"),
            ],
        ))],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
    let adt = tc.adt_registry().get("Option").unwrap();
    assert_eq!(adt.arity(), 1);
    assert!(adt.is_enum());
}

#[test]
fn test_adt_duplicate_type_error() {
    // struct Point {}
    // struct Point {} // duplicate!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct("Point", &[], vec![])),
            Item::Struct(make_struct("Point", &[], vec![])),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::DuplicateType { .. }
    ));
}

#[test]
fn test_adt_duplicate_struct_enum_error() {
    // struct Foo {}
    // enum Foo { A } // duplicate!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct("Foo", &[], vec![])),
            Item::Enum(make_enum("Foo", &[], vec![make_unit_variant("A")])),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::DuplicateType { .. }
    ));
}

#[test]
fn test_adt_capability_in_struct_field_error() {
    // struct Bad { cap: NetCap } // forbidden!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Struct(make_struct(
            "Bad",
            &[],
            vec![make_field("cap", ty_adt("NetCap"))],
        ))],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::CapabilityInAdt { .. }
    ));
}

#[test]
fn test_adt_capability_in_nested_type_error() {
    // First register Option
    // struct Bad { opt: Option<FsCap> } // forbidden - nested cap!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Struct(make_struct(
                "Bad",
                &[],
                vec![make_field(
                    "opt",
                    ty_generic("Option", vec![ty_adt("FsCap")]),
                )],
            )),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::CapabilityInAdt { .. }
    ));
}

#[test]
fn test_adt_capability_in_enum_variant_error() {
    // enum Bad { HasCap(TimeCap) } // forbidden!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Enum(make_enum(
            "Bad",
            &[],
            vec![make_tuple_variant("HasCap", vec![ty_adt("TimeCap")])],
        ))],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::CapabilityInAdt { .. }
    ));
}

#[test]
fn test_adt_enum_constructors_registered() {
    // enum Status { Active, Inactive }
    // Constructors Status::Active and Status::Inactive should be in environment
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Enum(make_enum(
            "Status",
            &[],
            vec![make_unit_variant("Active"), make_unit_variant("Inactive")],
        ))],
        span: sp(),
    };
    tc.check_module(&module).unwrap();

    // Check constructors are available by trying to use them
    // (We can't directly access env, but we can verify via infer)
    // For now, just verify the ADT is registered correctly
    let adt = tc.adt_registry().get("Status").unwrap();
    assert!(adt.find_variant("Active").is_some());
    assert!(adt.find_variant("Inactive").is_some());
}

#[test]
fn test_adt_ty_from_type_expr_builtin() {
    // Ensure builtin types still work
    let tc = TypeChecker::new();
    let ty = tc
        .ty_from_type_expr(&TypeExpr::Path(vec![ident("Int")], sp()))
        .unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_adt_ty_from_type_expr_user_defined() {
    // After registering Point, ty_from_type_expr should recognize it
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Struct(make_struct("Point", &[], vec![]))],
        span: sp(),
    };
    tc.check_module(&module).unwrap();

    let ty = tc
        .ty_from_type_expr(&TypeExpr::Path(vec![ident("Point")], sp()))
        .unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::adt0("Point"));
}

#[test]
fn test_adt_ty_from_type_expr_generic() {
    // Option<Int> should work after registering Option
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Enum(make_enum(
            "Option",
            &["T"],
            vec![
                make_tuple_variant("Some", vec![ty_adt("T")]),
                make_unit_variant("None"),
            ],
        ))],
        span: sp(),
    };
    tc.check_module(&module).unwrap();

    let ty = tc
        .ty_from_type_expr(&ty_generic("Option", vec![ty_int()]))
        .unwrap();
    assert_eq!(
        ty,
        crate::infer::ty::Ty::adt("Option", vec![crate::infer::ty::Ty::int()])
    );
}

#[test]
fn test_adt_ty_from_type_expr_unknown_type_error() {
    let tc = TypeChecker::new();
    let result = tc.ty_from_type_expr(&TypeExpr::Path(vec![ident("Unknown")], sp()));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::UnknownType { .. }));
}

#[test]
fn test_adt_ty_from_type_expr_wrong_arity_error() {
    // Option without type arg should fail (Option requires 1 arg)
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Enum(make_enum(
            "Option",
            &["T"],
            vec![
                make_tuple_variant("Some", vec![ty_adt("T")]),
                make_unit_variant("None"),
            ],
        ))],
        span: sp(),
    };
    tc.check_module(&module).unwrap();

    let result = tc.ty_from_type_expr(&TypeExpr::Path(vec![ident("Option")], sp()));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::WrongTypeArgCount { .. }
    ));
}

#[test]
fn test_adt_ty_from_type_expr_tuple() {
    // (Int, Bool) should produce Tuple type
    let tc = TypeChecker::new();
    let ty = tc
        .ty_from_type_expr(&TypeExpr::Tuple(vec![ty_int(), ty_bool()], sp()))
        .unwrap();
    assert_eq!(
        ty,
        crate::infer::ty::Ty::tuple(vec![
            crate::infer::ty::Ty::int(),
            crate::infer::ty::Ty::bool_()
        ])
    );
}

#[test]
fn test_adt_ty_from_type_expr_empty_tuple_is_unit() {
    // () should produce Unit
    let tc = TypeChecker::new();
    let ty = tc
        .ty_from_type_expr(&TypeExpr::Tuple(vec![], sp()))
        .unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::unit());
}

#[test]
fn test_adt_ty_from_type_expr_single_element_unwrapped() {
    // (Int) should just be Int (not a 1-tuple)
    let tc = TypeChecker::new();
    let ty = tc
        .ty_from_type_expr(&TypeExpr::Tuple(vec![ty_int()], sp()))
        .unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

// ============================================================================
// PHASE 4: MATCH TYPE INFERENCE TESTS
// ============================================================================

/// Helper to create an Ident pattern
fn pat_ident(name: &str) -> Pat {
    Pat::Ident(ident(name))
}

/// Helper to create a wildcard pattern
fn pat_wildcard() -> Pat {
    Pat::Wildcard(sp())
}

/// Helper to create a tuple pattern
fn pat_tuple(pats: Vec<Pat>) -> Pat {
    Pat::Tuple(pats, sp())
}

/// Helper to create a variant pattern (e.g., Some(x))
fn pat_variant(type_name: &str, variant_name: &str, pats: Vec<Pat>) -> Pat {
    Pat::Variant {
        path: Path {
            segments: vec![ident(type_name), ident(variant_name)],
            span: sp(),
        },
        fields: pats,
        span: sp(),
    }
}

/// Helper to create a unit variant pattern (e.g., None)
fn pat_unit_variant(type_name: &str, variant_name: &str) -> Pat {
    Pat::Variant {
        path: Path {
            segments: vec![ident(type_name), ident(variant_name)],
            span: sp(),
        },
        fields: vec![],
        span: sp(),
    }
}

/// Helper to create a struct pattern
fn pat_struct(type_name: &str, fields: Vec<(&str, Pat)>) -> Pat {
    Pat::Struct {
        path: Path {
            segments: vec![ident(type_name)],
            span: sp(),
        },
        fields: fields
            .into_iter()
            .map(|(name, pat)| PatField {
                name: ident(name),
                pat,
                span: sp(),
            })
            .collect(),
        span: sp(),
    }
}

/// Helper to create a match arm
fn make_arm(pat: Pat, body: Expr) -> MatchArm {
    MatchArm {
        pat,
        body,
        span: sp(),
    }
}

/// Helper to create a tuple expression
fn expr_tuple(elems: Vec<Expr>) -> Expr {
    Expr::Tuple { elems, span: sp() }
}

/// Helper to create a struct expression
fn expr_struct(type_name: &str, fields: Vec<(&str, Expr)>) -> Expr {
    Expr::StructExpr {
        path: Path {
            segments: vec![ident(type_name)],
            span: sp(),
        },
        fields: fields
            .into_iter()
            .map(|(name, value)| FieldInit {
                name: ident(name),
                value,
                span: sp(),
            })
            .collect(),
        span: sp(),
    }
}

/// Helper to create an enum variant constructor call (e.g., Option::Some(42))
fn expr_variant_call(type_name: &str, variant_name: &str, args: Vec<Expr>) -> Expr {
    Expr::Call {
        callee: Box::new(Expr::PathExpr(Path {
            segments: vec![ident(type_name), ident(variant_name)],
            span: sp(),
        })),
        args,
        span: sp(),
    }
}

/// Helper to create a unit variant reference (e.g., Option::None)
#[allow(dead_code)]
fn expr_unit_variant(type_name: &str, variant_name: &str) -> Expr {
    Expr::PathExpr(Path {
        segments: vec![ident(type_name), ident(variant_name)],
        span: sp(),
    })
}

/// Helper to create a match expression
fn expr_match(scrutinee: Expr, arms: Vec<MatchArm>) -> Expr {
    Expr::Match {
        scrutinee: Box::new(scrutinee),
        arms,
        span: sp(),
    }
}

// ---------------------------------------------------------------------------
// Match on Option (Some/None patterns)
// ---------------------------------------------------------------------------

#[test]
fn test_option_without_pattern() {
    // Test that we can register Option and use it in a function signature
    // without pattern matching (diagnositc test)
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(Expr::Lit(Lit::Int(0), sp()))), // Just return 0
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_variant_pattern_direct() {
    // Test variant pattern checking via check_module
    // This ensures that the constructor schemes are properly allocated
    // with fresh type variables that don't conflict with inference.
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("opt")),
                        vec![
                            make_arm(
                                pat_unit_variant("Option", "None"),
                                Expr::Lit(Lit::Int(0), sp()),
                            ),
                            // Add wildcard to make match exhaustive
                            make_arm(pat_wildcard(), Expr::Lit(Lit::Int(1), sp())),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_match_with_literal_pattern() {
    // Test match with literal patterns (not variant patterns)
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Fn(FnDecl {
            name: ident("test"),
            params: vec![Param {
                name: ident("x"),
                ty: Some(ty_int()),
                span: sp(),
            }],
            ret_ty: Some(ty_int()),
            effects: None,
            body: Block {
                stmts: vec![],
                tail: Some(Box::new(expr_match(
                    Expr::Var(ident("x")),
                    vec![
                        make_arm(
                            Pat::Literal(Lit::Int(0), sp()),
                            Expr::Lit(Lit::Int(0), sp()),
                        ),
                        make_arm(pat_wildcard(), Expr::Lit(Lit::Int(1), sp())),
                    ],
                ))),
                span: sp(),
            },
            span: sp(),
        })],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_match_option_some_none() {
    // enum Option<T> { Some(T), None }
    // fn test(opt: Option<Int>) -> Int {
    //     match opt {
    //         Option::Some(x) => x,
    //         Option::None => 0
    //     }
    // }
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("opt")),
                        vec![
                            make_arm(
                                pat_variant("Option", "Some", vec![pat_ident("x")]),
                                Expr::Var(ident("x")),
                            ),
                            make_arm(
                                pat_unit_variant("Option", "None"),
                                Expr::Lit(Lit::Int(0), sp()),
                            ),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_match_arm_type_mismatch() {
    // Match arms return different types - should fail
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: None,
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("opt")),
                        vec![
                            make_arm(
                                pat_variant("Option", "Some", vec![pat_ident("x")]),
                                Expr::Var(ident("x")), // Int
                            ),
                            make_arm(
                                pat_unit_variant("Option", "None"),
                                Expr::Lit(Lit::Str("none".to_string()), sp()), // String - mismatch!
                            ),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

// ---------------------------------------------------------------------------
// Nested patterns
// ---------------------------------------------------------------------------

#[test]
fn test_match_nested_tuple_pattern() {
    // match ((1, 2), 3) { ((a, b), c) => a + b + c }
    let mut tc = TypeChecker::new();
    let expr = expr_match(
        expr_tuple(vec![
            expr_tuple(vec![
                Expr::Lit(Lit::Int(1), sp()),
                Expr::Lit(Lit::Int(2), sp()),
            ]),
            Expr::Lit(Lit::Int(3), sp()),
        ]),
        vec![make_arm(
            pat_tuple(vec![
                pat_tuple(vec![pat_ident("a"), pat_ident("b")]),
                pat_ident("c"),
            ]),
            Expr::Binary {
                lhs: Box::new(Expr::Binary {
                    lhs: Box::new(Expr::Var(ident("a"))),
                    op: BinOp::Add,
                    rhs: Box::new(Expr::Var(ident("b"))),
                    span: sp(),
                }),
                op: BinOp::Add,
                rhs: Box::new(Expr::Var(ident("c"))),
                span: sp(),
            },
        )],
    );
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_match_nested_variant_pattern() {
    // enum Option<T> { Some(T), None }
    // match Some((1, 2)) { Some((a, b)) => a + b, None => 0 }
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        expr_variant_call(
                            "Option",
                            "Some",
                            vec![expr_tuple(vec![
                                Expr::Lit(Lit::Int(1), sp()),
                                Expr::Lit(Lit::Int(2), sp()),
                            ])],
                        ),
                        vec![
                            make_arm(
                                pat_variant(
                                    "Option",
                                    "Some",
                                    vec![pat_tuple(vec![pat_ident("a"), pat_ident("b")])],
                                ),
                                Expr::Binary {
                                    lhs: Box::new(Expr::Var(ident("a"))),
                                    op: BinOp::Add,
                                    rhs: Box::new(Expr::Var(ident("b"))),
                                    span: sp(),
                                },
                            ),
                            make_arm(
                                pat_unit_variant("Option", "None"),
                                Expr::Lit(Lit::Int(0), sp()),
                            ),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

// ---------------------------------------------------------------------------
// Pattern type mismatches
// ---------------------------------------------------------------------------

#[test]
fn test_pattern_tuple_arity_mismatch() {
    // match (1, 2) { (a, b, c) => 0 } - wrong tuple arity
    let mut tc = TypeChecker::new();
    let expr = expr_match(
        expr_tuple(vec![
            Expr::Lit(Lit::Int(1), sp()),
            Expr::Lit(Lit::Int(2), sp()),
        ]),
        vec![make_arm(
            pat_tuple(vec![pat_ident("a"), pat_ident("b"), pat_ident("c")]),
            Expr::Lit(Lit::Int(0), sp()),
        )],
    );
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
}

#[test]
fn test_pattern_variant_arity_mismatch() {
    // enum Option<T> { Some(T), None }
    // match Some(1) { Some(a, b) => 0, None => 0 } - Some takes 1 arg, not 2
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        expr_variant_call("Option", "Some", vec![Expr::Lit(Lit::Int(1), sp())]),
                        vec![
                            make_arm(
                                pat_variant("Option", "Some", vec![pat_ident("a"), pat_ident("b")]),
                                Expr::Lit(Lit::Int(0), sp()),
                            ),
                            make_arm(
                                pat_unit_variant("Option", "None"),
                                Expr::Lit(Lit::Int(0), sp()),
                            ),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
}

#[test]
fn test_pattern_unknown_variant() {
    // enum Option<T> { Some(T), None }
    // match x { Option::Unknown => 0 } - Unknown is not a variant
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("opt")),
                        vec![make_arm(
                            pat_unit_variant("Option", "Unknown"),
                            Expr::Lit(Lit::Int(0), sp()),
                        )],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::UnknownVariant { .. }
    ));
}

// ---------------------------------------------------------------------------
// Duplicate binding errors
// ---------------------------------------------------------------------------

#[test]
fn test_pattern_duplicate_binding() {
    // match (1, 2) { (x, x) => x } - x bound twice!
    let mut tc = TypeChecker::new();
    let expr = expr_match(
        expr_tuple(vec![
            Expr::Lit(Lit::Int(1), sp()),
            Expr::Lit(Lit::Int(2), sp()),
        ]),
        vec![make_arm(
            pat_tuple(vec![pat_ident("x"), pat_ident("x")]),
            Expr::Var(ident("x")),
        )],
    );
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::DuplicateField { .. }
    ));
}

#[test]
fn test_pattern_duplicate_binding_nested() {
    // match ((1, 2), 3) { ((a, b), a) => a } - a bound twice in nested pattern
    let mut tc = TypeChecker::new();
    let expr = expr_match(
        expr_tuple(vec![
            expr_tuple(vec![
                Expr::Lit(Lit::Int(1), sp()),
                Expr::Lit(Lit::Int(2), sp()),
            ]),
            Expr::Lit(Lit::Int(3), sp()),
        ]),
        vec![make_arm(
            pat_tuple(vec![
                pat_tuple(vec![pat_ident("a"), pat_ident("b")]),
                pat_ident("a"), // duplicate!
            ]),
            Expr::Var(ident("a")),
        )],
    );
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::DuplicateField { .. }
    ));
}

// ---------------------------------------------------------------------------
// Tuple construction/destructuring
// ---------------------------------------------------------------------------

#[test]
fn test_tuple_construction() {
    // (1, true, "hello") has type (Int, Bool, String)
    let mut tc = TypeChecker::new();
    let expr = expr_tuple(vec![
        Expr::Lit(Lit::Int(1), sp()),
        Expr::Lit(Lit::Bool(true), sp()),
        Expr::Lit(Lit::Str("hello".to_string()), sp()),
    ]);
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(
        ty,
        crate::infer::ty::Ty::tuple(vec![
            crate::infer::ty::Ty::int(),
            crate::infer::ty::Ty::bool_(),
            crate::infer::ty::Ty::string()
        ])
    );
}

#[test]
fn test_tuple_destructuring_basic() {
    // match (1, 2) { (a, b) => a + b }
    let mut tc = TypeChecker::new();
    let expr = expr_match(
        expr_tuple(vec![
            Expr::Lit(Lit::Int(1), sp()),
            Expr::Lit(Lit::Int(2), sp()),
        ]),
        vec![make_arm(
            pat_tuple(vec![pat_ident("a"), pat_ident("b")]),
            Expr::Binary {
                lhs: Box::new(Expr::Var(ident("a"))),
                op: BinOp::Add,
                rhs: Box::new(Expr::Var(ident("b"))),
                span: sp(),
            },
        )],
    );
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_tuple_arity_limit() {
    // Tuples with more than 8 elements should fail
    let mut tc = TypeChecker::new();
    let expr = expr_tuple(vec![
        Expr::Lit(Lit::Int(1), sp()),
        Expr::Lit(Lit::Int(2), sp()),
        Expr::Lit(Lit::Int(3), sp()),
        Expr::Lit(Lit::Int(4), sp()),
        Expr::Lit(Lit::Int(5), sp()),
        Expr::Lit(Lit::Int(6), sp()),
        Expr::Lit(Lit::Int(7), sp()),
        Expr::Lit(Lit::Int(8), sp()),
        Expr::Lit(Lit::Int(9), sp()), // 9th element - too many!
    ]);
    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    // The error should be about arity limit
    assert!(matches!(
        result.unwrap_err(),
        TypeError::ArityMismatch { .. }
    ));
}

// ---------------------------------------------------------------------------
// Struct construction
// ---------------------------------------------------------------------------

#[test]
fn test_struct_construction() {
    // struct Point { x: Int, y: Int }
    // Point { x: 1, y: 2 } has type Point
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct(
                "Point",
                &[],
                vec![make_field("x", ty_int()), make_field("y", ty_int())],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![],
                ret_ty: Some(ty_adt("Point")),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_struct(
                        "Point",
                        vec![
                            ("x", Expr::Lit(Lit::Int(1), sp())),
                            ("y", Expr::Lit(Lit::Int(2), sp())),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_struct_missing_field_error() {
    // struct Point { x: Int, y: Int }
    // Point { x: 1 } - missing y!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct(
                "Point",
                &[],
                vec![make_field("x", ty_int()), make_field("y", ty_int())],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![],
                ret_ty: Some(ty_adt("Point")),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_struct(
                        "Point",
                        vec![("x", Expr::Lit(Lit::Int(1), sp()))],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::MissingField { .. }
    ));
}

#[test]
fn test_struct_unknown_field_error() {
    // struct Point { x: Int, y: Int }
    // Point { x: 1, y: 2, z: 3 } - z doesn't exist!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct(
                "Point",
                &[],
                vec![make_field("x", ty_int()), make_field("y", ty_int())],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![],
                ret_ty: Some(ty_adt("Point")),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_struct(
                        "Point",
                        vec![
                            ("x", Expr::Lit(Lit::Int(1), sp())),
                            ("y", Expr::Lit(Lit::Int(2), sp())),
                            ("z", Expr::Lit(Lit::Int(3), sp())),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::UnknownField { .. }
    ));
}

#[test]
fn test_struct_field_type_mismatch() {
    // struct Point { x: Int, y: Int }
    // Point { x: "hello", y: 2 } - x should be Int!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct(
                "Point",
                &[],
                vec![make_field("x", ty_int()), make_field("y", ty_int())],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![],
                ret_ty: Some(ty_adt("Point")),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_struct(
                        "Point",
                        vec![
                            ("x", Expr::Lit(Lit::Str("hello".to_string()), sp())),
                            ("y", Expr::Lit(Lit::Int(2), sp())),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TypeError::Mismatch { .. }));
}

// ---------------------------------------------------------------------------
// Match with wildcard pattern
// ---------------------------------------------------------------------------

#[test]
fn test_match_wildcard_pattern() {
    // match 1 { _ => 0 }
    let mut tc = TypeChecker::new();
    let expr = expr_match(
        Expr::Lit(Lit::Int(1), sp()),
        vec![make_arm(pat_wildcard(), Expr::Lit(Lit::Int(0), sp()))],
    );
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_match_ident_pattern_binds_value() {
    // match 42 { x => x }
    let mut tc = TypeChecker::new();
    let expr = expr_match(
        Expr::Lit(Lit::Int(42), sp()),
        vec![make_arm(pat_ident("x"), Expr::Var(ident("x")))],
    );
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

// ---------------------------------------------------------------------------
// Struct patterns
// ---------------------------------------------------------------------------

#[test]
fn test_struct_pattern_destructuring() {
    // struct Point { x: Int, y: Int }
    // fn test(p: Point) -> Int {
    //     match p { Point { x, y } => x + y }
    // }
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct(
                "Point",
                &[],
                vec![make_field("x", ty_int()), make_field("y", ty_int())],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("p"),
                    ty: Some(ty_adt("Point")),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("p")),
                        vec![make_arm(
                            pat_struct("Point", vec![("x", pat_ident("x")), ("y", pat_ident("y"))]),
                            Expr::Binary {
                                lhs: Box::new(Expr::Var(ident("x"))),
                                op: BinOp::Add,
                                rhs: Box::new(Expr::Var(ident("y"))),
                                span: sp(),
                            },
                        )],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

// ===========================================================================
// Phase 5: Exhaustiveness and Redundancy Tests
// ===========================================================================

#[test]
fn test_exhaustive_option_both_variants() {
    // enum Option<T> { Some(T), None }
    // match opt { Some(x) => x, None => 0 } - exhaustive
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("opt")),
                        vec![
                            make_arm(
                                pat_variant("Option", "Some", vec![pat_ident("x")]),
                                Expr::Var(ident("x")),
                            ),
                            make_arm(
                                pat_unit_variant("Option", "None"),
                                Expr::Lit(Lit::Int(0), sp()),
                            ),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_non_exhaustive_option_some_only_error() {
    // enum Option<T> { Some(T), None }
    // match opt { Some(x) => x } - missing None!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("opt")),
                        vec![make_arm(
                            pat_variant("Option", "Some", vec![pat_ident("x")]),
                            Expr::Var(ident("x")),
                        )],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, TypeError::NonExhaustiveMatch { witness, .. } if witness.contains("None"))
    );
}

#[test]
fn test_non_exhaustive_option_none_only_error() {
    // enum Option<T> { Some(T), None }
    // match opt { None => 0 } - missing Some!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("opt")),
                        vec![make_arm(
                            pat_unit_variant("Option", "None"),
                            Expr::Lit(Lit::Int(0), sp()),
                        )],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, TypeError::NonExhaustiveMatch { witness, .. } if witness.contains("Some"))
    );
}

#[test]
fn test_exhaustive_bool_both_values() {
    // match b { true => 1, false => 0 } - exhaustive
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Fn(FnDecl {
            name: ident("test"),
            params: vec![Param {
                name: ident("b"),
                ty: Some(ty_bool()),
                span: sp(),
            }],
            ret_ty: Some(ty_int()),
            effects: None,
            body: Block {
                stmts: vec![],
                tail: Some(Box::new(expr_match(
                    Expr::Var(ident("b")),
                    vec![
                        make_arm(
                            Pat::Literal(Lit::Bool(true), sp()),
                            Expr::Lit(Lit::Int(1), sp()),
                        ),
                        make_arm(
                            Pat::Literal(Lit::Bool(false), sp()),
                            Expr::Lit(Lit::Int(0), sp()),
                        ),
                    ],
                ))),
                span: sp(),
            },
            span: sp(),
        })],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_non_exhaustive_bool_true_only_error() {
    // match b { true => 1 } - missing false!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Fn(FnDecl {
            name: ident("test"),
            params: vec![Param {
                name: ident("b"),
                ty: Some(ty_bool()),
                span: sp(),
            }],
            ret_ty: Some(ty_int()),
            effects: None,
            body: Block {
                stmts: vec![],
                tail: Some(Box::new(expr_match(
                    Expr::Var(ident("b")),
                    vec![make_arm(
                        Pat::Literal(Lit::Bool(true), sp()),
                        Expr::Lit(Lit::Int(1), sp()),
                    )],
                ))),
                span: sp(),
            },
            span: sp(),
        })],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, TypeError::NonExhaustiveMatch { witness, .. } if witness.contains("false"))
    );
}

#[test]
fn test_exhaustive_with_wildcard() {
    // match opt { Some(x) => x, _ => 0 } - wildcard covers None
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("opt")),
                        vec![
                            make_arm(
                                pat_variant("Option", "Some", vec![pat_ident("x")]),
                                Expr::Var(ident("x")),
                            ),
                            make_arm(pat_wildcard(), Expr::Lit(Lit::Int(0), sp())),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_redundant_arm_after_wildcard_error() {
    // match opt { _ => 0, None => 1 } - None is unreachable!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Option",
                &["T"],
                vec![
                    make_tuple_variant("Some", vec![ty_adt("T")]),
                    make_unit_variant("None"),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("opt"),
                    ty: Some(ty_generic("Option", vec![ty_int()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("opt")),
                        vec![
                            make_arm(pat_wildcard(), Expr::Lit(Lit::Int(0), sp())),
                            make_arm(
                                pat_unit_variant("Option", "None"),
                                Expr::Lit(Lit::Int(1), sp()),
                            ),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::UnreachablePattern { arm_index: 1, .. }
    ));
}

#[test]
fn test_redundant_bool_after_both_covered() {
    // match b { true => 1, false => 0, true => 2 } - third arm unreachable
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Fn(FnDecl {
            name: ident("test"),
            params: vec![Param {
                name: ident("b"),
                ty: Some(ty_bool()),
                span: sp(),
            }],
            ret_ty: Some(ty_int()),
            effects: None,
            body: Block {
                stmts: vec![],
                tail: Some(Box::new(expr_match(
                    Expr::Var(ident("b")),
                    vec![
                        make_arm(
                            Pat::Literal(Lit::Bool(true), sp()),
                            Expr::Lit(Lit::Int(1), sp()),
                        ),
                        make_arm(
                            Pat::Literal(Lit::Bool(false), sp()),
                            Expr::Lit(Lit::Int(0), sp()),
                        ),
                        make_arm(
                            Pat::Literal(Lit::Bool(true), sp()),
                            Expr::Lit(Lit::Int(2), sp()),
                        ),
                    ],
                ))),
                span: sp(),
            },
            span: sp(),
        })],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::UnreachablePattern { arm_index: 2, .. }
    ));
}

#[test]
fn test_int_literal_needs_wildcard() {
    // match n { 0 => "zero", 1 => "one" } - missing other ints!
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Fn(FnDecl {
            name: ident("test"),
            params: vec![Param {
                name: ident("n"),
                ty: Some(ty_int()),
                span: sp(),
            }],
            ret_ty: Some(ty_string()),
            effects: None,
            body: Block {
                stmts: vec![],
                tail: Some(Box::new(expr_match(
                    Expr::Var(ident("n")),
                    vec![
                        make_arm(
                            Pat::Literal(Lit::Int(0), sp()),
                            Expr::Lit(Lit::Str("zero".to_string()), sp()),
                        ),
                        make_arm(
                            Pat::Literal(Lit::Int(1), sp()),
                            Expr::Lit(Lit::Str("one".to_string()), sp()),
                        ),
                    ],
                ))),
                span: sp(),
            },
            span: sp(),
        })],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::NonExhaustiveMatch { .. }
    ));
}

#[test]
fn test_int_literal_with_wildcard_exhaustive() {
    // match n { 0 => "zero", _ => "other" } - exhaustive with wildcard
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Fn(FnDecl {
            name: ident("test"),
            params: vec![Param {
                name: ident("n"),
                ty: Some(ty_int()),
                span: sp(),
            }],
            ret_ty: Some(ty_string()),
            effects: None,
            body: Block {
                stmts: vec![],
                tail: Some(Box::new(expr_match(
                    Expr::Var(ident("n")),
                    vec![
                        make_arm(
                            Pat::Literal(Lit::Int(0), sp()),
                            Expr::Lit(Lit::Str("zero".to_string()), sp()),
                        ),
                        make_arm(
                            pat_wildcard(),
                            Expr::Lit(Lit::Str("other".to_string()), sp()),
                        ),
                    ],
                ))),
                span: sp(),
            },
            span: sp(),
        })],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_result_enum_exhaustive() {
    // enum Result<T, E> { Ok(T), Err(E) }
    // match r { Ok(x) => x, Err(_) => -1 } - exhaustive
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Enum(make_enum(
                "Result",
                &["T", "E"],
                vec![
                    make_tuple_variant("Ok", vec![ty_adt("T")]),
                    make_tuple_variant("Err", vec![ty_adt("E")]),
                ],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("r"),
                    ty: Some(ty_generic("Result", vec![ty_int(), ty_string()])),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("r")),
                        vec![
                            make_arm(
                                pat_variant("Result", "Ok", vec![pat_ident("x")]),
                                Expr::Var(ident("x")),
                            ),
                            make_arm(
                                pat_variant("Result", "Err", vec![pat_wildcard()]),
                                Expr::Lit(Lit::Int(-1), sp()),
                            ),
                        ],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_struct_single_constructor_exhaustive() {
    // struct Point { x: Int, y: Int }
    // match p { Point { x, y } => x + y } - exhaustive (single constructor)
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct(
                "Point",
                &[],
                vec![make_field("x", ty_int()), make_field("y", ty_int())],
            )),
            Item::Fn(FnDecl {
                name: ident("test"),
                params: vec![Param {
                    name: ident("p"),
                    ty: Some(ty_adt("Point")),
                    span: sp(),
                }],
                ret_ty: Some(ty_int()),
                effects: None,
                body: Block {
                    stmts: vec![],
                    tail: Some(Box::new(expr_match(
                        Expr::Var(ident("p")),
                        vec![make_arm(
                            pat_struct("Point", vec![("x", pat_ident("x")), ("y", pat_ident("y"))]),
                            Expr::Binary {
                                lhs: Box::new(Expr::Var(ident("x"))),
                                op: BinOp::Add,
                                rhs: Box::new(Expr::Var(ident("y"))),
                                span: sp(),
                            },
                        )],
                    ))),
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}

#[test]
fn test_multiple_redundant_arms() {
    // match b { true => 1, false => 0, _ => 2, false => 3 }
    // Both _ and the second false are redundant, but we report first
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![Item::Fn(FnDecl {
            name: ident("test"),
            params: vec![Param {
                name: ident("b"),
                ty: Some(ty_bool()),
                span: sp(),
            }],
            ret_ty: Some(ty_int()),
            effects: None,
            body: Block {
                stmts: vec![],
                tail: Some(Box::new(expr_match(
                    Expr::Var(ident("b")),
                    vec![
                        make_arm(
                            Pat::Literal(Lit::Bool(true), sp()),
                            Expr::Lit(Lit::Int(1), sp()),
                        ),
                        make_arm(
                            Pat::Literal(Lit::Bool(false), sp()),
                            Expr::Lit(Lit::Int(0), sp()),
                        ),
                        make_arm(pat_wildcard(), Expr::Lit(Lit::Int(2), sp())),
                        make_arm(
                            Pat::Literal(Lit::Bool(false), sp()),
                            Expr::Lit(Lit::Int(3), sp()),
                        ),
                    ],
                ))),
                span: sp(),
            },
            span: sp(),
        })],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    // First redundant arm is the wildcard at index 2
    assert!(matches!(
        result.unwrap_err(),
        TypeError::UnreachablePattern { arm_index: 2, .. }
    ));
}

// Helper function to create a nested generic type like Result<Option<Int>, String>
#[allow(dead_code)]
fn ty_nested_generic(
    name: &str,
    args: Vec<strata_ast::ast::TypeExpr>,
) -> strata_ast::ast::TypeExpr {
    strata_ast::ast::TypeExpr::App {
        base: vec![ident(name)],
        args,
        span: sp(),
    }
}

// ============ Destructuring Let Tests ============

#[test]
fn test_destructuring_let_tuple() {
    // let (a, b) = (1, 2); a + b
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![Stmt::Let {
            mutable: false,
            pat: Pat::Tuple(vec![Pat::Ident(ident("a")), Pat::Ident(ident("b"))], sp()),
            ty: None,
            value: Expr::Tuple {
                elems: vec![Expr::Lit(Lit::Int(1), sp()), Expr::Lit(Lit::Int(2), sp())],
                span: sp(),
            },
            span: sp(),
        }],
        tail: Some(Box::new(Expr::Binary {
            lhs: Box::new(Expr::Var(ident("a"))),
            op: BinOp::Add,
            rhs: Box::new(Expr::Var(ident("b"))),
            span: sp(),
        })),
        span: sp(),
    });
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_destructuring_let_nested_tuple() {
    // let ((a, b), c) = ((1, 2), 3); a + b + c
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![Stmt::Let {
            mutable: false,
            pat: Pat::Tuple(
                vec![
                    Pat::Tuple(vec![Pat::Ident(ident("a")), Pat::Ident(ident("b"))], sp()),
                    Pat::Ident(ident("c")),
                ],
                sp(),
            ),
            ty: None,
            value: Expr::Tuple {
                elems: vec![
                    Expr::Tuple {
                        elems: vec![Expr::Lit(Lit::Int(1), sp()), Expr::Lit(Lit::Int(2), sp())],
                        span: sp(),
                    },
                    Expr::Lit(Lit::Int(3), sp()),
                ],
                span: sp(),
            },
            span: sp(),
        }],
        tail: Some(Box::new(Expr::Binary {
            lhs: Box::new(Expr::Binary {
                lhs: Box::new(Expr::Var(ident("a"))),
                op: BinOp::Add,
                rhs: Box::new(Expr::Var(ident("b"))),
                span: sp(),
            }),
            op: BinOp::Add,
            rhs: Box::new(Expr::Var(ident("c"))),
            span: sp(),
        })),
        span: sp(),
    });
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_destructuring_let_wildcard() {
    // let _ = 5; 42
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![Stmt::Let {
            mutable: false,
            pat: Pat::Wildcard(sp()),
            ty: None,
            value: Expr::Lit(Lit::Int(5), sp()),
            span: sp(),
        }],
        tail: Some(Box::new(Expr::Lit(Lit::Int(42), sp()))),
        span: sp(),
    });
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::int());
}

#[test]
fn test_destructuring_let_refutable_pattern_error() {
    // let Option::Some(x) = ...; should fail (refutable pattern)
    use strata_ast::ast::{EnumDef, Variant, VariantFields as VF};

    let mut tc = TypeChecker::new();

    // Register Option enum
    let option_enum = EnumDef {
        name: ident("Option"),
        type_params: vec![ident("T")],
        variants: vec![
            Variant {
                name: ident("Some"),
                fields: VF::Tuple(vec![TypeExpr::Path(vec![ident("T")], sp())]),
                span: sp(),
            },
            Variant {
                name: ident("None"),
                fields: VF::Unit,
                span: sp(),
            },
        ],
        span: sp(),
    };
    tc.check_module(&Module {
        items: vec![Item::Enum(option_enum)],
        span: sp(),
    })
    .unwrap();

    // Try: let Option::Some(x) = Option::Some(42);
    let expr = Expr::Block(Block {
        stmts: vec![Stmt::Let {
            mutable: false,
            pat: Pat::Variant {
                path: Path {
                    segments: vec![ident("Option"), ident("Some")],
                    span: sp(),
                },
                fields: vec![Pat::Ident(ident("x"))],
                span: sp(),
            },
            ty: None,
            value: Expr::Call {
                callee: Box::new(Expr::PathExpr(Path {
                    segments: vec![ident("Option"), ident("Some")],
                    span: sp(),
                })),
                args: vec![Expr::Lit(Lit::Int(42), sp())],
                span: sp(),
            },
            span: sp(),
        }],
        tail: Some(Box::new(Expr::Var(ident("x")))),
        span: sp(),
    });

    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::RefutablePattern { .. }
    ));
}

#[test]
fn test_destructuring_let_literal_error() {
    // let 0 = 0; should fail (refutable pattern - literals)
    let mut tc = TypeChecker::new();
    let expr = Expr::Block(Block {
        stmts: vec![Stmt::Let {
            mutable: false,
            pat: Pat::Literal(Lit::Int(0), sp()),
            ty: None,
            value: Expr::Lit(Lit::Int(0), sp()),
            span: sp(),
        }],
        tail: Some(Box::new(Expr::Lit(Lit::Int(42), sp()))),
        span: sp(),
    });

    let result = tc.infer_expr(&expr);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::RefutablePattern { .. }
    ));
}

// ============================================================================
// CAPABILITY IN BINDING TESTS
// ============================================================================

#[test]
fn test_capability_in_let_binding_direct() {
    // struct NetCap {}
    // let x = NetCap {};  // should fail - binding holds a capability
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct("NetCap", &[], vec![])),
            Item::Let(LetDecl {
                name: ident("x"),
                ty: None,
                value: expr_struct("NetCap", vec![]),
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err(), "expected error but got Ok");
    let err = result.unwrap_err();
    assert!(
        matches!(err, TypeError::CapabilityInBinding { .. }),
        "expected CapabilityInBinding but got: {:?}",
        err
    );
}

#[test]
fn test_capability_in_let_binding_tuple() {
    // struct NetCap {}
    // let x = (1, NetCap {});  // should fail - tuple contains a capability
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct("NetCap", &[], vec![])),
            Item::Let(LetDecl {
                name: ident("x"),
                ty: None,
                value: Expr::Tuple {
                    elems: vec![Expr::Lit(Lit::Int(1), sp()), expr_struct("NetCap", vec![])],
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    let result = tc.check_module(&module);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TypeError::CapabilityInBinding { .. }
    ));
}

#[test]
fn test_non_capability_in_let_binding_ok() {
    // struct Safe {}
    // let x = (1, Safe {});  // should pass - no capability
    let mut tc = TypeChecker::new();
    let module = Module {
        items: vec![
            Item::Struct(make_struct("Safe", &[], vec![])),
            Item::Let(LetDecl {
                name: ident("x"),
                ty: None,
                value: Expr::Tuple {
                    elems: vec![Expr::Lit(Lit::Int(1), sp()), expr_struct("Safe", vec![])],
                    span: sp(),
                },
                span: sp(),
            }),
        ],
        span: sp(),
    };
    assert!(tc.check_module(&module).is_ok());
}
