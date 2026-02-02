// crates/strata-types/src/checker_tests.rs
// Comprehensive tests for the type checker

use super::checker::{TypeChecker, TypeError};
use strata_ast::ast::{BinOp, Expr, Ident, Item, LetDecl, Lit, Module, TypeExpr, UnOp};
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

fn ty_float() -> TypeExpr {
    TypeExpr::Path(vec![ident("Float")], sp())
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
    let expr = Expr::Lit(Lit::Float(3.14), sp());
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
fn test_float_addition() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Lit(Lit::Float(1.5), sp())),
        op: BinOp::Add,
        rhs: Box::new(Expr::Lit(Lit::Float(2.5), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::float());
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
fn test_unary_neg_float() {
    let mut tc = TypeChecker::new();
    let expr = Expr::Unary {
        op: UnOp::Neg,
        expr: Box::new(Expr::Lit(Lit::Float(3.14), sp())),
        span: sp(),
    };
    let ty = tc.infer_expr(&expr).unwrap();
    assert_eq!(ty, crate::infer::ty::Ty::float());
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
fn test_function_call_not_implemented() {
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
        TypeError::NotImplemented { .. }
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
