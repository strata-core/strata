// Integration tests live outside the crate root, so we import from the public API.
use strata_ast::ast::{BinOp, Expr, Item};
use strata_parse::parse_str;

fn parse_expr_only(src: &str) -> Expr {
    let m = parse_str("<mem>", &format!("let x = {src};")).expect("parse ok");
    let Item::Let(ld) = &m.items[0] else {
        panic!("expected Let declaration");
    };
    ld.value.clone()
}

#[test]
fn precedence_and_or_eq_rel_add_mul() {
    let e = parse_expr_only("1 + 2*3 == 7 && 4 < 5 || 0 == 1");

    // top is Or
    if let Expr::Binary {
        op: BinOp::Or,
        lhs,
        rhs,
        ..
    } = e
    {
        // left is And
        if let Expr::Binary {
            op: BinOp::And,
            lhs: and_lhs,
            rhs: and_rhs,
            ..
        } = *lhs
        {
            // left of And is Eq over (1 + (2*3)) == 7
            if let Expr::Binary {
                op: BinOp::Eq,
                lhs: eq_lhs,
                rhs: eq_rhs,
                ..
            } = *and_lhs
            {
                // left of Eq is Add
                assert!(matches!(*eq_lhs, Expr::Binary { op: BinOp::Add, .. }));
                // right of Eq is literal 7
                assert!(matches!(*eq_rhs, Expr::Lit(_, _)));
            } else {
                panic!("left And should be Eq");
            }

            // right of And is Lt (4 < 5)
            assert!(matches!(*and_rhs, Expr::Binary { op: BinOp::Lt, .. }));
        } else {
            panic!("lhs should be And");
        }

        // right of Or is Eq (0 == 1)
        assert!(matches!(*rhs, Expr::Binary { op: BinOp::Eq, .. }));
    } else {
        panic!("top should be Or");
    }
}
