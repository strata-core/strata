use strata_parse::parse_str;

/// Helper: parse source, expect parse failure
fn parse_err(src: &str) {
    assert!(
        parse_str("<test>", src).is_err(),
        "expected parse error for: {src}"
    );
}

#[test]
fn fn_with_effect_annotation() {
    let module = parse_str("<test>", "fn f() -> Int & {Fs} { 0 }").unwrap();
    let item = &module.items[0];
    match item {
        strata_ast::ast::Item::Fn(decl) => {
            let effects = decl.effects.as_ref().expect("should have effects");
            assert_eq!(effects.len(), 1);
            assert_eq!(effects[0].text, "Fs");
        }
        _ => panic!("expected fn"),
    }
}

#[test]
fn fn_without_effect_annotation() {
    let module = parse_str("<test>", "fn f() -> Int { 0 }").unwrap();
    match &module.items[0] {
        strata_ast::ast::Item::Fn(decl) => {
            assert!(decl.effects.is_none());
        }
        _ => panic!("expected fn"),
    }
}

#[test]
fn fn_with_empty_effect_annotation() {
    let module = parse_str("<test>", "fn f() -> Int & {} { 0 }").unwrap();
    match &module.items[0] {
        strata_ast::ast::Item::Fn(decl) => {
            let effects = decl.effects.as_ref().expect("should have effects");
            assert_eq!(effects.len(), 0);
        }
        _ => panic!("expected fn"),
    }
}

#[test]
fn fn_with_multiple_effects() {
    let module = parse_str("<test>", "fn f() -> Int & {Net, Fs, Time} { 0 }").unwrap();
    match &module.items[0] {
        strata_ast::ast::Item::Fn(decl) => {
            let effects = decl.effects.as_ref().expect("should have effects");
            assert_eq!(effects.len(), 3);
            assert_eq!(effects[0].text, "Net");
            assert_eq!(effects[1].text, "Fs");
            assert_eq!(effects[2].text, "Time");
        }
        _ => panic!("expected fn"),
    }
}

#[test]
fn fn_with_trailing_comma_effects() {
    let module = parse_str("<test>", "fn f() -> Int & {Fs, Net,} { 0 }").unwrap();
    match &module.items[0] {
        strata_ast::ast::Item::Fn(decl) => {
            let effects = decl.effects.as_ref().expect("should have effects");
            assert_eq!(effects.len(), 2);
        }
        _ => panic!("expected fn"),
    }
}

#[test]
fn extern_fn_parsed() {
    let module = parse_str("<test>", "extern fn read(path: String) -> String & {Fs};").unwrap();
    match &module.items[0] {
        strata_ast::ast::Item::ExternFn(decl) => {
            assert_eq!(decl.name.text, "read");
            assert_eq!(decl.params.len(), 1);
            let effects = decl.effects.as_ref().expect("should have effects");
            assert_eq!(effects.len(), 1);
            assert_eq!(effects[0].text, "Fs");
        }
        _ => panic!("expected extern fn"),
    }
}

#[test]
fn extern_fn_no_effects() {
    let module = parse_str("<test>", "extern fn pure_fn() -> Int;").unwrap();
    match &module.items[0] {
        strata_ast::ast::Item::ExternFn(decl) => {
            assert_eq!(decl.name.text, "pure_fn");
            assert!(decl.effects.is_none());
        }
        _ => panic!("expected extern fn"),
    }
}

#[test]
fn fn_no_return_type_with_effects() {
    // Effects can appear after the implicit return type
    let module = parse_str("<test>", "fn f() & {Fs} { 0 }").unwrap();
    match &module.items[0] {
        strata_ast::ast::Item::Fn(decl) => {
            assert!(decl.ret_ty.is_none());
            let effects = decl.effects.as_ref().expect("should have effects");
            assert_eq!(effects.len(), 1);
            assert_eq!(effects[0].text, "Fs");
        }
        _ => panic!("expected fn"),
    }
}

#[test]
fn fn_effect_missing_braces_error() {
    // & Fs without braces should fail
    parse_err("fn f() -> Int & Fs { 0 }");
}

#[test]
fn fn_type_with_effects() {
    // fn(Int) -> Int & {Fs} as a type annotation
    let module = parse_str("<test>", "let x: fn(Int) -> Int & {Fs} = f;").unwrap();
    // Just check it parses - the type annotation contains Arrow with effects
    match &module.items[0] {
        strata_ast::ast::Item::Let(decl) => {
            assert!(decl.ty.is_some());
        }
        _ => panic!("expected let"),
    }
}
