//! Integration tests for effect system type checking
//!
//! Post-009: All functions and extern declarations with concrete effects must
//! have matching capability parameters. These tests validate effect inference,
//! checking, and propagation with capabilities properly threaded through.

use strata_parse::parse_str;
use strata_types::TypeChecker;

/// Helper: parse and type-check, expect success
fn check_ok(src: &str) {
    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();
    checker
        .check_module(&module)
        .unwrap_or_else(|e| panic!("expected OK but got error: {e}"));
}

/// Helper: parse and type-check, expect failure
fn check_err(src: &str) -> String {
    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();
    let err = checker
        .check_module(&module)
        .expect_err("expected type error but got OK");
    format!("{err}")
}

// ============================================================================
// POSITIVE TESTS - Valid programs with effects
// ============================================================================

#[test]
fn pure_function_no_annotation() {
    // Pure function with no effect annotation should type check fine
    check_ok("fn add(x: Int, y: Int) -> Int { x + y }");
}

#[test]
fn pure_function_with_empty_effects() {
    // Explicit empty effect set means pure
    check_ok("fn add(x: Int, y: Int) -> Int & {} { x + y }");
}

#[test]
fn pure_function_with_explicit_effects_superset() {
    // Declaring effects that aren't used is OK (superset allowed),
    // but now requires the matching capability parameter
    check_ok("fn add(fs: FsCap, x: Int, y: Int) -> Int & {Fs} { x + y }");
}

#[test]
fn extern_fn_with_effects() {
    // Extern fn declaration with effects requires matching cap param
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
    "#,
    );
}

#[test]
fn extern_fn_explicit_pure() {
    // Extern fn with explicit empty effects (pure)
    check_ok(
        r#"
        extern fn pure_calc(x: Int) -> Int & {};
    "#,
    );
}

#[test]
fn extern_fn_missing_effects_error() {
    // Extern fn without effect annotation → error
    let err = check_err(
        r#"
        extern fn bad_extern(x: Int) -> Int;
    "#,
    );
    assert!(
        err.contains("must declare") || err.contains("effect"),
        "Expected missing effects error, got: {err}"
    );
}

#[test]
fn call_effectful_extern_from_effectful_fn() {
    // Function with {Fs} calls extern fn with {Fs} → OK
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn load(fs: FsCap, p: String) -> String & {Fs} { read_file(fs, p) }
    "#,
    );
}

#[test]
fn call_effectful_extern_from_superset_fn() {
    // Function declares {Fs, Net} and calls fn with just {Fs} → OK (superset)
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn load(fs: FsCap, net: NetCap, p: String) -> String & {Fs, Net} { read_file(fs, p) }
    "#,
    );
}

#[test]
fn call_pure_extern_from_pure_fn() {
    // Pure fn calls pure extern → OK
    check_ok(
        r#"
        extern fn pure_calc(x: Int) -> Int & {};
        fn do_calc(x: Int) -> Int { pure_calc(x) }
    "#,
    );
}

#[test]
fn unannotated_fn_calling_effectful_extern() {
    // Unannotated function calling effectful extern → effects inferred.
    // The inferred {Fs} effect requires FsCap.
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn load(fs: FsCap, p: String) -> String { read_file(fs, p) }
    "#,
    );
}

#[test]
fn multiple_effects_accumulate() {
    // Function calls both {Fs} and {Net} externs → needs both effects and caps
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        extern fn fetch(net: NetCap, url: String) -> String & {Net};
        fn both(fs: FsCap, net: NetCap, p: String, u: String) -> String & {Fs, Net} {
            let _a = read_file(fs, p);
            fetch(net, u)
        }
    "#,
    );
}

#[test]
fn transitive_effects() {
    // a() calls b() which has {Fs} → a() inferred {Fs}, then called from c() with {Fs}
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn b(fs: FsCap, p: String) -> String & {Fs} { read_file(fs, p) }
        fn a(fs: FsCap, p: String) -> String & {Fs} { b(fs, p) }
    "#,
    );
}

#[test]
fn effectful_fn_called_in_let() {
    // Let binding can call effectful functions if enclosing fn has effects
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn process(fs: FsCap, p: String) -> Int & {Fs} {
            let _data = read_file(fs, p);
            42
        }
    "#,
    );
}

#[test]
fn effect_order_independence() {
    // {Net, Fs} should be the same as {Fs, Net}
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        extern fn fetch(net: NetCap, url: String) -> String & {Net};
        fn both(fs: FsCap, net: NetCap, p: String, u: String) -> String & {Net, Fs} {
            let _a = read_file(fs, p);
            fetch(net, u)
        }
    "#,
    );
}

#[test]
fn duplicate_effects_in_annotation() {
    // Duplicate effects in annotation are OK (set semantics)
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn load(fs: FsCap, p: String) -> String & {Fs, Fs} { read_file(fs, p) }
    "#,
    );
}

#[test]
fn pure_fn_calling_pure_fn() {
    // Pure calling pure with explicit & {} annotations
    check_ok(
        r#"
        fn add(x: Int, y: Int) -> Int & {} { x + y }
        fn double(x: Int) -> Int & {} { add(x, x) }
    "#,
    );
}

#[test]
fn recursive_fn_with_effects() {
    // Recursive function with effects
    check_ok(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        fn countdown(fs: FsCap, n: Int) -> () & {Fs} {
            if n > 0 {
                log(fs, "tick");
                countdown(fs, n - 1)
            } else {
                ()
            }
        }
    "#,
    );
}

#[test]
fn adt_constructors_are_pure() {
    // Enum constructors should be pure — no effect propagation
    check_ok(
        r#"
        enum Option<T> { Some(T), None }
        fn make_some(x: Int) -> Option<Int> & {} { Option::Some(x) }
    "#,
    );
}

#[test]
fn all_five_effects() {
    // All five effect names are recognized, each with matching cap
    check_ok(
        r#"
        extern fn e1(fs: FsCap) -> () & {Fs};
        extern fn e2(net: NetCap) -> () & {Net};
        extern fn e3(time: TimeCap) -> () & {Time};
        extern fn e4(rand: RandCap) -> () & {Rand};
        extern fn e5(ai: AiCap) -> () & {Ai};
        fn all(fs: FsCap, net: NetCap, time: TimeCap, rand: RandCap, ai: AiCap) -> () & {Fs, Net, Time, Rand, Ai} {
            e1(fs);
            e2(net);
            e3(time);
            e4(rand);
            e5(ai)
        }
    "#,
    );
}

// ============================================================================
// NEGATIVE TESTS - Invalid programs that should fail
// ============================================================================

#[test]
fn pure_fn_calling_effectful_extern_error() {
    // Pure function (& {}) calls effectful extern → effect mismatch error.
    // The function has FsCap to satisfy arity, but declares pure effects.
    let err = check_err(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn bad(fs: FsCap) -> String & {} { read_file(fs, "test") }
    "#,
    );
    assert!(
        err.contains("effect") || err.contains("Effect"),
        "Expected effect error, got: {err}"
    );
}

#[test]
fn insufficient_effects_error() {
    // Function declares {Net} but calls {Fs} → error
    let err = check_err(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn bad(fs: FsCap, net: NetCap, p: String) -> String & {Net} { read_file(fs, p) }
    "#,
    );
    assert!(
        err.contains("effect") || err.contains("Effect"),
        "Expected effect error, got: {err}"
    );
}

#[test]
fn unknown_effect_name_error() {
    // Unknown effect name should give an error
    let err = check_err(
        r#"
        fn bad() -> Int & {Foo} { 0 }
    "#,
    );
    assert!(
        err.contains("Unknown effect") || err.contains("unknown effect"),
        "Expected unknown effect error, got: {err}"
    );
}

#[test]
fn extern_fn_unknown_effect_error() {
    // Unknown effect name on extern fn
    let err = check_err(
        r#"
        extern fn bad() -> Int & {BadEffect};
    "#,
    );
    assert!(
        err.contains("Unknown effect") || err.contains("unknown effect"),
        "Expected unknown effect error, got: {err}"
    );
}

#[test]
fn extern_fn_with_effects_missing_cap_error() {
    // Extern fn with {Fs} effect but no FsCap → error
    let err = check_err(
        r#"
        extern fn bad(path: String) -> String & {Fs};
    "#,
    );
    assert!(
        err.contains("FsCap") || err.contains("capability"),
        "Expected missing capability error on extern, got: {err}"
    );
}

#[test]
fn fn_with_effects_missing_cap_error() {
    // Function with declared {Fs} effect but no FsCap → error
    let err = check_err(
        r#"
        fn bad(x: Int) -> Int & {Fs} { x + 1 }
    "#,
    );
    assert!(
        err.contains("FsCap") || err.contains("capability"),
        "Expected missing capability error, got: {err}"
    );
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn never_plus_effects_diverging_body() {
    // A function that always returns still has its effects checked
    check_ok(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        fn diverge_with_effect(fs: FsCap) -> Int & {Fs} {
            log(fs, "bye");
            return 0;
        }
    "#,
    );
}

#[test]
fn empty_effect_set_equals_pure() {
    // & {} is the same as having no annotation for pure functions
    check_ok(
        r#"
        fn f1(x: Int) -> Int & {} { x + 1 }
        fn f2(x: Int) -> Int { f1(x) }
    "#,
    );
}

#[test]
fn match_arms_with_effectful_calls() {
    // Effects in match arms should propagate to enclosing function
    check_ok(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        enum Option<T> { Some(T), None }
        fn process(fs: FsCap, opt: Option<Int>) -> Int & {Fs} {
            match opt {
                Option::Some(x) => {
                    log(fs, "found");
                    x
                }
                Option::None => {
                    log(fs, "missing");
                    0
                }
            }
        }
    "#,
    );
}

#[test]
fn match_arms_effectful_error_in_pure() {
    // Match arms with effects in a pure function should fail
    let err = check_err(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        enum Option<T> { Some(T), None }
        fn process(fs: FsCap, opt: Option<Int>) -> Int & {} {
            match opt {
                Option::Some(x) => {
                    log(fs, "found");
                    x
                }
                Option::None => 0
            }
        }
    "#,
    );
    assert!(
        err.contains("effect") || err.contains("Effect"),
        "Expected effect error, got: {err}"
    );
}

#[test]
fn if_branches_with_effects() {
    // Effects from both if branches should propagate
    check_ok(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        extern fn fetch(net: NetCap, url: String) -> String & {Net};
        fn do_something(fs: FsCap, net: NetCap, flag: Bool) -> () & {Fs, Net} {
            if flag {
                log(fs, "yes");
            } else {
                let _r = fetch(net, "http://example.com");
            }
        }
    "#,
    );
}

#[test]
fn while_loop_with_effects() {
    // Effects inside while loop body should propagate
    check_ok(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        fn loop_with_log(fs: FsCap, n: Int) -> () & {Fs} {
            let mut i = 0;
            while i < n {
                log(fs, "iteration");
                i = i + 1;
            }
        }
    "#,
    );
}

#[test]
fn higher_order_effectful_fn() {
    // Passing an effectful function to a higher-order function.
    // Using single-arg extern so HOF pattern f(x) works.
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};
        fn apply(f, x) { f(x) }
        fn use_it(fs: FsCap) -> String & {Fs} { apply(do_fs, fs) }
    "#,
    );
}

#[test]
fn fn_no_return_type_with_effects_annotation() {
    // Effects annotation with no explicit return type
    check_ok(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        fn do_log(fs: FsCap, msg: String) & {Fs} { log(fs, msg) }
    "#,
    );
}

#[test]
fn nested_function_calls_accumulate_effects() {
    // Nested calls should accumulate effects from all callees
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        extern fn send(net: NetCap, url: String, data: String) -> () & {Net};
        fn read_and_send(fs: FsCap, net: NetCap, path: String, url: String) -> () & {Fs, Net} {
            send(net, url, read_file(fs, path))
        }
    "#,
    );
}

// ============================================================================
// C1 REGRESSION TESTS - Effect variable instantiation
// ============================================================================

#[test]
fn polymorphic_fn_called_with_different_effect_callees() {
    // C1 regression: `apply` is polymorphic. Each call with a different-effect
    // callee must get FRESH effect vars. Using single-arg externs for HOF compat.
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};
        extern fn do_net(net: NetCap) -> String & {Net};
        fn apply(f, x) { f(x) }
        fn use_both(fs: FsCap, net: NetCap) -> () & {Fs, Net} {
            let _a = apply(do_fs, fs);
            let _b = apply(do_net, net);
        }
    "#,
    );
}

#[test]
fn polymorphic_fn_different_effects_insufficient_declaration() {
    // Same as above but with insufficient declared effects — should error
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};
        extern fn do_net(net: NetCap) -> String & {Net};
        fn apply(f, x) { f(x) }
        fn use_both(fs: FsCap, net: NetCap) -> () & {Fs} {
            let _a = apply(do_fs, fs);
            let _b = apply(do_net, net);
        }
    "#,
    );
    assert!(
        err.contains("effect") || err.contains("Effect"),
        "Expected effect error, got: {err}"
    );
}

// ============================================================================
// MLR-1 REGRESSION TESTS - Effect laundering through HOF / let-generalization
// ============================================================================

#[test]
fn mlr1_effect_laundering_via_top_level_fn() {
    // MLR-1 adapted: A "launder" function that has its own effect and also calls
    // its callback. A pure function calling launder must fail with effect error.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> () & {Fs};
        extern fn do_net(net: NetCap) -> () & {Net};

        fn launder(fs: FsCap, f, x) & {Fs} {
            do_fs(fs);
            f(x)
        }

        fn pure_backdoor(fs: FsCap, net: NetCap) -> () & {} {
            launder(fs, do_net, net);
            ()
        }
    "#,
    );
    assert!(
        err.contains("effect") || err.contains("Effect"),
        "Expected effect error from laundered effects, got: {err}"
    );
}

#[test]
fn mlr1_pure_let_bound_value_remains_pure() {
    // A pure let binding in a pure function should remain valid.
    check_ok(
        r#"
        fn add(x: Int, y: Int) -> Int & {} { x + y }
        fn use_add() -> Int & {} {
            let result = add(1, 2);
            result
        }
    "#,
    );
}

#[test]
fn mlr1_effectful_let_in_effectful_fn() {
    // A let binding calling an effectful function is fine if the enclosing fn has effects.
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn load(fs: FsCap) -> String & {Fs} {
            let data = read_file(fs, "test.txt");
            data
        }
    "#,
    );
}

#[test]
fn mlr1_top_level_let_with_effects_propagation() {
    // Top-level let binding referencing an effectful function:
    // the bound value carries the effect in its type.
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};
        fn apply(f, x) { f(x) }
        fn use_it(fs: FsCap) -> String & {Fs} {
            let result = apply(do_fs, fs);
            result
        }
    "#,
    );
}

#[test]
fn mlr1_effect_propagation_through_multiple_hof_calls() {
    // Effects must accumulate across multiple HOF calls.
    // A pure function calling apply with effectful callbacks must fail.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> () & {Fs};
        extern fn do_net(net: NetCap) -> () & {Net};
        fn apply(f, x) { f(x) }
        fn pure_backdoor(fs: FsCap, net: NetCap) -> () & {} {
            let _a = apply(do_fs, fs);
            let _b = apply(do_net, net);
            ()
        }
    "#,
    );
    assert!(
        err.contains("effect") || err.contains("Effect"),
        "Expected effect error from accumulated HOF effects, got: {err}"
    );
}
