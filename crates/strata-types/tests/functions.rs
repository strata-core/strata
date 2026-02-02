//! Integration tests for function type checking

use strata_parse::parse_str;
use strata_types::TypeChecker;

#[test]
fn simple_function_declaration() {
    let src = r#"
        fn add(x: Int, y: Int) -> Int { x + y }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn function_with_inferred_params() {
    let src = r#"
        fn double(x) -> Int { x + x }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn function_with_inferred_return() {
    let src = r#"
        fn identity(x: Int) { x }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn function_call_in_let() {
    let src = r#"
        fn add(x: Int, y: Int) -> Int { x + y }
        let result = add(1, 2);
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn polymorphic_identity_function() {
    let src = r#"
        fn identity(x) { x }
        let a = identity(42);
        let b = identity(true);
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn higher_order_function() {
    let src = r#"
        fn apply(f, x) { f(x) }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn type_error_wrong_arg_count() {
    let src = r#"
        fn add(x: Int, y: Int) -> Int { x + y }
        let result = add(1);
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_err());
}

#[test]
fn type_error_wrong_return_type() {
    let src = r#"
        fn bad() -> Int { true }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_err());
}

#[test]
fn type_error_wrong_arg_type() {
    let src = r#"
        fn add(x: Int, y: Int) -> Int { x + y }
        let result = add(true, 2);
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_err());
}

#[test]
fn multiple_functions() {
    let src = r#"
        fn double(x: Int) -> Int { x + x }
        fn triple(x: Int) -> Int { x + x + x }
        let a = double(5);
        let b = triple(3);
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn type_error_unknown_variable() {
    let src = r#"
        fn uses_unknown() -> Int {
            unknownVar + 1
        }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    let result = checker.check_module(&module);
    assert!(result.is_err());

    // Verify it's specifically an UnknownVariable error
    match result {
        Err(e) => {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("Unknown variable"));
            assert!(err_msg.contains("unknownVar"));
        }
        Ok(_) => panic!("Expected error for unknown variable, but got success"),
    }
}

#[test]
fn type_error_bool_arithmetic() {
    let src = r#"
        let bad = true + true;
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    let result = checker.check_module(&module);
    assert!(result.is_err());

    // Verify it's a type mismatch (Bool vs Int)
    match result {
        Err(e) => {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("Bool") || err_msg.contains("Int"));
        }
        Ok(_) => panic!("Expected error for true + true, but got success"),
    }
}

#[test]
fn type_error_neg_bool() {
    let src = r#"
        let bad = -true;
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    let result = checker.check_module(&module);
    assert!(result.is_err());
}

#[test]
fn forward_reference_works() {
    let src = r#"
        fn f() -> Int { g() }
        fn g() -> Int { 42 }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    // Should succeed - g is predeclared in pass 1
    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn mutual_recursion_predeclared() {
    let src = r#"
        fn even(n: Int) -> Bool { 
            true  // Simplified - full recursion later
        }
        fn odd(n: Int) -> Bool { 
            false  // Simplified - full recursion later
        }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    // Should succeed - both functions predeclared
    assert!(checker.check_module(&module).is_ok());
}
