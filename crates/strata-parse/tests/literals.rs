use strata_ast::ast::{Expr, Item, Lit};
use strata_parse::parse_str;

#[test]
fn ints_floats_bools_nil_string_escapes() {
    let m = parse_str(
        "<mem>",
        r#"let a = 42; let b = 3.5; let c = true; let d = nil; let e = "hi\n\"there\"";"#,
    )
    .unwrap();
    let take = |i: usize| -> &Expr {
        let Item::Let(ld) = &m.items[i] else {
            panic!("expected Let declaration");
        };
        &ld.value
    };

    assert!(matches!(take(0), Expr::Lit(Lit::Int(42), _)));
    assert!(matches!(take(1), Expr::Lit(Lit::Float(f), _) if (*f - 3.5).abs() < 1e-9));
    assert!(matches!(take(2), Expr::Lit(Lit::Bool(true), _)));
    assert!(matches!(take(3), Expr::Lit(Lit::Nil, _)));
    assert!(matches!(take(4), Expr::Lit(Lit::Str(s), _) if s == "hi\n\"there\""));
}
