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

/// Test that recursive functions cannot be used polymorphically within their own body.
/// This is the key soundness test for function generalization timing.
#[test]
fn test_recursive_fn_not_polymorphic() {
    // This MUST fail: f calls itself with different types (Bool and Int)
    // within its own body. If f were polymorphic during self-reference,
    // both calls would succeed (unsound). With correct monomorphic placeholder,
    // both calls must unify x to the same type, causing a mismatch.
    let src = r#"
        fn f(x) {
            let y = f(true);
            let z = f(1);
            x
        }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    let result = checker.check_module(&module);
    assert!(
        result.is_err(),
        "Expected type error: recursive fn should not be polymorphic in its own body"
    );

    // Verify it's a type mismatch between Bool and Int
    if let Err(e) = result {
        let err_msg = format!("{}", e);
        assert!(
            err_msg.contains("Bool") || err_msg.contains("Int"),
            "Error should mention Bool/Int mismatch: {}",
            err_msg
        );
    }
}

/// Test that mutually recursive functions with consistent types still work.
#[test]
fn test_mutually_recursive_consistent() {
    // This should succeed: even and odd call each other with consistent Int type
    let src = r#"
        fn even(n: Int) -> Bool {
            if n == 0 {
                true
            } else {
                odd(n - 1)
            }
        }
        fn odd(n: Int) -> Bool {
            if n == 0 {
                false
            } else {
                even(n - 1)
            }
        }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(
        checker.check_module(&module).is_ok(),
        "Mutually recursive functions with consistent types should type check"
    );
}

/// Test that closures with return statements inside nested scopes work correctly.
#[test]
fn test_closure_nested_scope_return() {
    // This should succeed: return inside a nested block
    let src = r#"
        fn foo(x: Int) -> Int {
            if x > 0 {
                return x + 1;
            };
            0
        }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(
        checker.check_module(&module).is_ok(),
        "Return inside nested scope should type check"
    );
}

// ============ Soundness tests for Never/diverging branches ============

/// Test that mismatched branches with one diverging FAILS.
/// This was a soundness bug: `if cond { return 42; } else { "string" }`
/// should NOT typecheck because the else branch is String, not Int.
#[test]
fn test_if_never_branch_mismatch_error() {
    let src = r#"
        fn f(cond: Bool) -> Int {
            if cond { return 42; } else { "string" }
        }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    let result = checker.check_module(&module);
    assert!(
        result.is_err(),
        "Mismatched branches (Int return vs String) should fail type check"
    );
}

/// Test that both branches diverging is OK.
#[test]
fn test_if_never_both_branches_diverge() {
    let src = r#"
        fn f(cond: Bool) -> Int {
            if cond { return 1; } else { return 2; }
        }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(
        checker.check_module(&module).is_ok(),
        "Both branches returning should be OK"
    );
}

/// Test that one branch diverges, other matches return type is OK.
#[test]
fn test_if_never_then_diverges_else_used() {
    let src = r#"
        fn f(cond: Bool) -> Int {
            if cond { return 42; } else { 100 }
        }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(
        checker.check_module(&module).is_ok(),
        "Diverging then with matching else should be OK"
    );
}

/// Test that else diverges, then matches return type is OK.
#[test]
fn test_if_never_else_diverges_then_used() {
    let src = r#"
        fn f(cond: Bool) -> Int {
            if cond { 100 } else { return 42; }
        }
    "#;

    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();

    assert!(
        checker.check_module(&module).is_ok(),
        "Diverging else with matching then should be OK"
    );
}
