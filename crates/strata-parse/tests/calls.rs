use strata_ast::ast::{Expr, Item};
use strata_parse::parse_str;

#[test]
fn call_binds_tighter_than_infix() {
    let m = parse_str("<mem>", "let x = f(1) + 2; let y = f(1 + 2);").unwrap();
    let v = |idx: usize| -> &Expr {
        let Item::Let(ld) = &m.items[idx] else {
            panic!("expected Let declaration");
        };
        &ld.value
    };

    // x = (f(1)) + 2
    match v(0) {
        Expr::Binary { lhs, .. } => assert!(matches!(**lhs, Expr::Call { .. })),
        _ => panic!("x lhs should be a call"),
    }
    // y = f((1 + 2))
    match v(1) {
        Expr::Call { args, .. } => assert!(matches!(args[0], Expr::Binary { .. })),
        _ => panic!("y should be a call"),
    }
}

#[test]
fn chained_calls() {
    let m = parse_str("<mem>", "let z = f(g(1), h(2, 3));").unwrap();
    let Item::Let(ld) = &m.items[0] else {
        panic!("expected Let declaration");
    };
    match &ld.value {
        Expr::Call { args, .. } => {
            assert!(matches!(args[0], Expr::Call { .. }));
            assert!(matches!(args[1], Expr::Call { .. }));
        }
        _ => panic!("expected top-level call"),
    }
}
