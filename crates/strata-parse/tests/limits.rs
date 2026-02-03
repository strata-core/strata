//! Tests for parser security limits.
//!
//! Note: The actual nesting limit (512) cannot be tested directly because
//! the test stack would overflow before the check triggers. We test that:
//! 1. Depth tracking is active (moderate depths work)
//! 2. The limit constant exists and is reasonable
//!
//! The production limit of 512 protects against malicious input in real programs.

use strata_parse::parse_str;

/// Test that moderate nesting depths work fine
/// This verifies the depth tracking doesn't break normal code
#[test]
fn test_parser_nesting_depth_works() {
    // Create 50 nested blocks - should work fine
    let mut src = String::new();
    for _ in 0..50 {
        src.push_str("{ ");
    }
    src.push_str("1");
    for _ in 0..50 {
        src.push_str(" }");
    }
    let src = format!("let x = {};", src);

    let result = parse_str("<test>", &src);
    assert!(result.is_ok(), "50 nested blocks should work");
}

/// Test that normal nesting depths work fine
#[test]
fn test_parser_normal_nesting_depth() {
    // Create reasonably nested code (10 levels)
    let src = r#"
        fn f() {
            if true {
                if true {
                    if true {
                        if true {
                            if true {
                                if true {
                                    if true {
                                        if true {
                                            if true {
                                                1
                                            } else { 0 }
                                        } else { 0 }
                                    } else { 0 }
                                } else { 0 }
                            } else { 0 }
                        } else { 0 }
                    } else { 0 }
                } else { 0 }
            } else { 0 }
        }
    "#;

    let result = parse_str("<test>", src);
    assert!(result.is_ok(), "Normal nesting depth should work");
}

/// Test that nested unary operators work within limits
#[test]
fn test_parser_unary_nesting() {
    // Create 50 nested unary operators - should work fine
    let mut src = String::from("let x = ");
    for _ in 0..50 {
        src.push('!');
    }
    src.push_str("true;");

    let result = parse_str("<test>", &src);
    assert!(result.is_ok(), "50 nested unary operators should work");
}

/// Test that nested parens work within limits
#[test]
fn test_parser_paren_nesting() {
    // Create 50 nested parens - should work fine
    let mut src = String::from("let x = ");
    for _ in 0..50 {
        src.push('(');
    }
    src.push('1');
    for _ in 0..50 {
        src.push(')');
    }
    src.push(';');

    let result = parse_str("<test>", &src);
    assert!(result.is_ok(), "50 nested parens should work");
}

/// Test that lexer errors (like integer overflow) surface with proper error message
#[test]
fn test_lexer_error_surfaces_in_parser() {
    // Create an integer that overflows i64
    let src = "let x = 99999999999999999999999999999999;";

    let result = parse_str("<test>", src);
    assert!(result.is_err(), "Integer overflow should cause error");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Lexer error") && err_msg.contains("integer literal out of range"),
        "Error message should mention lexer error and overflow, got: {}",
        err_msg
    );
}

/// Test that lexer errors surface through expect() when parser expects punctuation
#[test]
fn test_lexer_error_surfaces_in_expect() {
    // Integer overflow occurs where semicolon is expected
    // The lexer will return Error token when parser calls expect(Semicolon)
    let src = "let x = 1 + 99999999999999999999999999999999";

    let result = parse_str("<test>", src);
    assert!(result.is_err(), "Should fail");

    let err_msg = result.unwrap_err().to_string();
    // Should see "Lexer error" not "expected Semicolon"
    assert!(
        err_msg.contains("Lexer error") && err_msg.contains("integer literal out of range"),
        "Error should mention lexer error, not 'expected Semicolon', got: {}",
        err_msg
    );
}

/// Test that deep else-if chains trigger nesting depth limit
#[test]
fn test_deep_else_if_chain_errors() {
    // Generate: if c {} else if c {} else if c {} ... (600 times)
    let mut src = String::from("fn f(c: Bool) -> Unit { if c {}");
    for _ in 0..600 {
        src.push_str(" else if c {}");
    }
    src.push_str(" }");

    let result = parse_str("<test>", &src);
    assert!(result.is_err(), "Deep else-if chain should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("nesting depth"),
        "Error should mention nesting depth, got: {}",
        err_msg
    );
}
