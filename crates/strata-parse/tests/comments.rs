use strata_ast::ast::{Expr, Item, Lit};
use strata_parse::parse_str;

#[test]
fn line_comments_and_ws_are_ignored() {
    let src = r#"
// leading comment
let a = 1; // inline
let b = (  // split
  2
) * 3;
// tail
"#;
    let m = parse_str("<mem>", src).unwrap();
    let take = |i| -> &Expr {
        let Item::Let(ld) = &m.items[i] else {
            panic!("expected Let declaration");
        };
        &ld.value
    };
    assert!(matches!(take(0), Expr::Lit(Lit::Int(1), _)));
    assert!(matches!(take(1), Expr::Binary { .. }));
}
