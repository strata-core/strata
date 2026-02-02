use strata_ast::ast::{BinOp, Expr, Item};
use strata_parse::parse_str;

fn val(m: &str) -> Expr {
    let module = parse_str("<mem>", &format!("let v = {m};")).unwrap();
    let Item::Let(ld) = &module.items[0] else {
        panic!("expected Let declaration");
    };
    ld.value.clone()
}

#[test]
fn arithmetic_relational_equality_logical_layers() {
    // 1 + 2*3 == 7 && 4 < 5 || 0 == 1
    let e = val("1 + 2*3 == 7 && 4 < 5 || 0 == 1");
    // top is Or
    if let Expr::Binary {
        op: BinOp::Or,
        lhs,
        rhs,
        ..
    } = e
    {
        // lhs is And
        assert!(matches!(*lhs, Expr::Binary { op: BinOp::And, .. }));
        // rhs is Eq
        assert!(matches!(*rhs, Expr::Binary { op: BinOp::Eq, .. }));
    } else {
        panic!("top should be Or");
    }
}

#[test]
fn equality_is_left_associative() {
    // With our (lbp=5, rbp=6) design, this parses left-assoc: (1==1)==1
    let e = val("1 == 1 == 1");
    if let Expr::Binary {
        op: BinOp::Eq,
        lhs,
        rhs,
        ..
    } = e
    {
        assert!(matches!(*lhs, Expr::Binary { op: BinOp::Eq, .. }));
        assert!(matches!(*rhs, Expr::Lit(_, _)));
    } else {
        panic!("top should be Eq");
    }
}
