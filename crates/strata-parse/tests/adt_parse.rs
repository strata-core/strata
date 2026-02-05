// Tests for ADT (struct/enum) and pattern matching parsing
// Phase 1 of Issue 007

use strata_ast::ast::{EnumDef, Expr, Item, Pat, StructDef, TypeExpr, VariantFields};
use strata_parse::parse_str;

/// Helper: parse and get the first item as a StructDef
fn parse_struct(src: &str) -> StructDef {
    let m = parse_str("<mem>", src).expect("parse ok");
    match &m.items[0] {
        Item::Struct(s) => s.clone(),
        _ => panic!("expected Struct"),
    }
}

/// Helper: parse and get the first item as an EnumDef
fn parse_enum(src: &str) -> EnumDef {
    let m = parse_str("<mem>", src).expect("parse ok");
    match &m.items[0] {
        Item::Enum(e) => e.clone(),
        _ => panic!("expected Enum"),
    }
}

/// Helper: parse an expression from a let statement
fn parse_expr(src: &str) -> Expr {
    let m = parse_str("<mem>", &format!("let x = {};", src)).expect("parse ok");
    let Item::Let(ld) = &m.items[0] else {
        panic!("expected Let");
    };
    ld.value.clone()
}

// ============ Struct Parsing Tests ============

#[test]
fn parse_struct_simple() {
    let s = parse_struct("struct Point { x: Int, y: Int }");
    assert_eq!(s.name.text, "Point");
    assert!(s.type_params.is_empty());
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[0].name.text, "x");
    assert_eq!(s.fields[1].name.text, "y");
}

#[test]
fn parse_struct_empty() {
    let s = parse_struct("struct Empty {}");
    assert_eq!(s.name.text, "Empty");
    assert!(s.type_params.is_empty());
    assert!(s.fields.is_empty());
}

#[test]
fn parse_struct_single_field() {
    let s = parse_struct("struct Wrapper { value: Int }");
    assert_eq!(s.name.text, "Wrapper");
    assert_eq!(s.fields.len(), 1);
    assert_eq!(s.fields[0].name.text, "value");
}

#[test]
fn parse_struct_trailing_comma() {
    let s = parse_struct("struct Point { x: Int, y: Int, }");
    assert_eq!(s.fields.len(), 2);
}

#[test]
fn parse_struct_generic_single() {
    let s = parse_struct("struct Box<T> { value: T }");
    assert_eq!(s.name.text, "Box");
    assert_eq!(s.type_params.len(), 1);
    assert_eq!(s.type_params[0].text, "T");
}

#[test]
fn parse_struct_generic_multiple() {
    let s = parse_struct("struct Pair<A, B> { fst: A, snd: B }");
    assert_eq!(s.name.text, "Pair");
    assert_eq!(s.type_params.len(), 2);
    assert_eq!(s.type_params[0].text, "A");
    assert_eq!(s.type_params[1].text, "B");
}

// ============ Enum Parsing Tests ============

#[test]
fn parse_enum_unit_variants() {
    let e = parse_enum("enum Color { Red, Green, Blue }");
    assert_eq!(e.name.text, "Color");
    assert!(e.type_params.is_empty());
    assert_eq!(e.variants.len(), 3);
    assert_eq!(e.variants[0].name.text, "Red");
    assert!(matches!(e.variants[0].fields, VariantFields::Unit));
}

#[test]
fn parse_enum_tuple_variant() {
    let e = parse_enum("enum Option<T> { Some(T), None }");
    assert_eq!(e.name.text, "Option");
    assert_eq!(e.type_params.len(), 1);
    assert_eq!(e.type_params[0].text, "T");
    assert_eq!(e.variants.len(), 2);

    // Some(T)
    assert_eq!(e.variants[0].name.text, "Some");
    let VariantFields::Tuple(ref tys) = e.variants[0].fields else {
        panic!("expected tuple variant");
    };
    assert_eq!(tys.len(), 1);

    // None
    assert_eq!(e.variants[1].name.text, "None");
    assert!(matches!(e.variants[1].fields, VariantFields::Unit));
}

#[test]
fn parse_enum_multiple_tuple_fields() {
    let e = parse_enum("enum Result<T, E> { Ok(T), Err(E) }");
    assert_eq!(e.type_params.len(), 2);
    assert_eq!(e.variants.len(), 2);
}

#[test]
fn parse_enum_trailing_comma() {
    let e = parse_enum("enum Bool { True, False, }");
    assert_eq!(e.variants.len(), 2);
}

// ============ Type Parsing Tests ============

#[test]
fn parse_type_simple() {
    let m = parse_str("<mem>", "fn test(x: Int) {}").expect("parse ok");
    let Item::Fn(f) = &m.items[0] else {
        panic!("expected fn");
    };
    let ty = f.params[0].ty.as_ref().unwrap();
    assert!(matches!(ty, TypeExpr::Path(segs, _) if segs.len() == 1 && segs[0].text == "Int"));
}

#[test]
fn parse_type_generic() {
    let m = parse_str("<mem>", "fn test(x: Option<Int>) {}").expect("parse ok");
    let Item::Fn(f) = &m.items[0] else {
        panic!("expected fn");
    };
    let ty = f.params[0].ty.as_ref().unwrap();
    let TypeExpr::App { base, args, .. } = ty else {
        panic!("expected App");
    };
    assert_eq!(base.len(), 1);
    assert_eq!(base[0].text, "Option");
    assert_eq!(args.len(), 1);
}

#[test]
fn parse_type_nested_generic() {
    let m = parse_str("<mem>", "fn test(x: Option<Option<Int>>) {}").expect("parse ok");
    let Item::Fn(f) = &m.items[0] else {
        panic!("expected fn");
    };
    let ty = f.params[0].ty.as_ref().unwrap();
    let TypeExpr::App { base, args, .. } = ty else {
        panic!("expected App");
    };
    assert_eq!(base[0].text, "Option");
    let TypeExpr::App {
        base: inner_base, ..
    } = &args[0]
    else {
        panic!("expected nested App");
    };
    assert_eq!(inner_base[0].text, "Option");
}

#[test]
fn parse_type_tuple() {
    let m = parse_str("<mem>", "fn test(x: (Int, Bool)) {}").expect("parse ok");
    let Item::Fn(f) = &m.items[0] else {
        panic!("expected fn");
    };
    let ty = f.params[0].ty.as_ref().unwrap();
    let TypeExpr::Tuple(elems, _) = ty else {
        panic!("expected Tuple type");
    };
    assert_eq!(elems.len(), 2);
}

#[test]
fn parse_type_empty_tuple() {
    let m = parse_str("<mem>", "fn test(x: ()) {}").expect("parse ok");
    let Item::Fn(f) = &m.items[0] else {
        panic!("expected fn");
    };
    let ty = f.params[0].ty.as_ref().unwrap();
    let TypeExpr::Tuple(elems, _) = ty else {
        panic!("expected Tuple type");
    };
    assert!(elems.is_empty());
}

// ============ Match Expression Tests ============

#[test]
fn parse_match_simple() {
    let e = parse_expr("match x { 0 => true, _ => false }");
    let Expr::Match { arms, .. } = e else {
        panic!("expected Match");
    };
    assert_eq!(arms.len(), 2);
}

#[test]
fn parse_match_with_wildcard() {
    let e = parse_expr("match x { _ => 0 }");
    let Expr::Match { arms, .. } = e else {
        panic!("expected Match");
    };
    assert!(matches!(arms[0].pat, Pat::Wildcard(_)));
}

#[test]
fn parse_match_with_ident_pattern() {
    let e = parse_expr("match x { y => y }");
    let Expr::Match { arms, .. } = e else {
        panic!("expected Match");
    };
    let Pat::Ident(id) = &arms[0].pat else {
        panic!("expected Ident pattern");
    };
    assert_eq!(id.text, "y");
}

#[test]
fn parse_match_with_literal_patterns() {
    let e = parse_expr("match x { 0 => a, 1 => b, _ => c }");
    let Expr::Match { arms, .. } = e else {
        panic!("expected Match");
    };
    assert_eq!(arms.len(), 3);
    assert!(matches!(&arms[0].pat, Pat::Literal(..)));
    assert!(matches!(&arms[1].pat, Pat::Literal(..)));
    assert!(matches!(&arms[2].pat, Pat::Wildcard(_)));
}

#[test]
fn parse_match_variant_pattern() {
    let e = parse_expr("match x { Option::Some(y) => y, Option::None => 0 }");
    let Expr::Match { arms, .. } = e else {
        panic!("expected Match");
    };
    assert_eq!(arms.len(), 2);

    // First arm: Option::Some(y)
    let Pat::Variant { path, fields, .. } = &arms[0].pat else {
        panic!("expected Variant pattern");
    };
    assert_eq!(path.as_str(), "Option::Some");
    assert_eq!(fields.len(), 1);

    // Second arm: Option::None
    let Pat::Variant { path, fields, .. } = &arms[1].pat else {
        panic!("expected Variant pattern");
    };
    assert_eq!(path.as_str(), "Option::None");
    assert!(fields.is_empty());
}

#[test]
fn parse_match_tuple_pattern() {
    let e = parse_expr("match x { (a, b) => a }");
    let Expr::Match { arms, .. } = e else {
        panic!("expected Match");
    };
    let Pat::Tuple(elems, _) = &arms[0].pat else {
        panic!("expected Tuple pattern");
    };
    assert_eq!(elems.len(), 2);
}

#[test]
fn parse_match_nested_pattern() {
    let e = parse_expr("match x { Option::Some(Option::Some(y)) => y, _ => 0 }");
    let Expr::Match { arms, .. } = e else {
        panic!("expected Match");
    };
    let Pat::Variant { fields, .. } = &arms[0].pat else {
        panic!("expected Variant pattern");
    };
    let Pat::Variant { path, .. } = &fields[0] else {
        panic!("expected nested Variant pattern");
    };
    assert_eq!(path.as_str(), "Option::Some");
}

#[test]
fn parse_match_trailing_comma() {
    let e = parse_expr("match x { 0 => a, _ => b, }");
    let Expr::Match { arms, .. } = e else {
        panic!("expected Match");
    };
    assert_eq!(arms.len(), 2);
}

// ============ Tuple Expression Tests ============

#[test]
fn parse_tuple_expr_simple() {
    let e = parse_expr("(1, 2)");
    let Expr::Tuple { elems, .. } = e else {
        panic!("expected Tuple");
    };
    assert_eq!(elems.len(), 2);
}

#[test]
fn parse_tuple_expr_triple() {
    let e = parse_expr("(1, 2, 3)");
    let Expr::Tuple { elems, .. } = e else {
        panic!("expected Tuple");
    };
    assert_eq!(elems.len(), 3);
}

#[test]
fn parse_tuple_expr_trailing_comma() {
    let e = parse_expr("(1, 2,)");
    let Expr::Tuple { elems, .. } = e else {
        panic!("expected Tuple");
    };
    assert_eq!(elems.len(), 2);
}

#[test]
fn parse_tuple_expr_empty() {
    let e = parse_expr("()");
    let Expr::Tuple { elems, .. } = e else {
        panic!("expected Tuple");
    };
    assert!(elems.is_empty());
}

#[test]
fn parse_paren_not_tuple() {
    // Single expression in parens is not a tuple
    let e = parse_expr("(1)");
    let Expr::Paren { .. } = e else {
        panic!("expected Paren, not Tuple");
    };
}

// ============ Path Expression Tests ============

#[test]
fn parse_path_expr_qualified() {
    let e = parse_expr("Option::None");
    let Expr::PathExpr(path) = e else {
        panic!("expected PathExpr");
    };
    assert_eq!(path.as_str(), "Option::None");
}

// ============ Struct Expression Tests ============

#[test]
fn parse_struct_expr_qualified() {
    let e = parse_expr("Foo::Bar { x: 1 }");
    let Expr::StructExpr { path, fields, .. } = e else {
        panic!("expected StructExpr");
    };
    assert_eq!(path.as_str(), "Foo::Bar");
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name.text, "x");
}

#[test]
fn parse_struct_expr_multiple_fields() {
    let e = parse_expr("Mod::Point { x: 1, y: 2 }");
    let Expr::StructExpr { fields, .. } = e else {
        panic!("expected StructExpr");
    };
    assert_eq!(fields.len(), 2);
}

// Note: Unqualified struct expressions (like `Point { x: 1 }`) are currently
// not supported to avoid ambiguity with blocks. Use qualified paths instead.
