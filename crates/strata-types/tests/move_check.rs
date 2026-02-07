//! Integration tests for the move checker (affine single-use enforcement).
//!
//! Capabilities are affine (at-most-once). The move checker validates this
//! as a post-inference pass — it does not modify unification or inference.

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
// POSITIVE TESTS — Valid single-use patterns
// ============================================================================

#[test]
fn single_use_in_call() {
    // Capability used exactly once in a function call — valid
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn single_use(fs: FsCap) -> () & {Fs} {
            use_cap(fs)
        }
    "#,
    );
}

#[test]
fn unused_capability_is_ok() {
    // Affine = at-most-once, not exactly-once. Dropping is fine.
    check_ok(
        r#"
        fn unused(fs: FsCap) -> () & {} {
            ()
        }
    "#,
    );
}

#[test]
fn branch_both_use() {
    // Both branches use fs — consistent, valid
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn branch_both(fs: FsCap, c: Bool) -> () & {Fs} {
            if c { use_cap(fs) } else { use_cap(fs) }
        }
    "#,
    );
}

#[test]
fn branch_neither_use_then_use() {
    // Neither branch uses fs, so it's alive after. Then used once — valid.
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn branch_neither(fs: FsCap, c: Bool) -> () & {Fs} {
            if c { () } else { () };
            use_cap(fs)
        }
    "#,
    );
}

#[test]
fn unrestricted_types_unaffected() {
    // Int is unrestricted — can be used any number of times
    check_ok(
        r#"
        fn multi_use(x: Int) -> Int & {} {
            x + x + x
        }
    "#,
    );
}

#[test]
fn unrestricted_in_branches() {
    // Unrestricted types work freely in both branches
    check_ok(
        r#"
        fn branch_int(x: Int, c: Bool) -> Int & {} {
            if c { x } else { x }
        }
    "#,
    );
}

#[test]
fn unrestricted_in_loop() {
    // Unrestricted types work freely inside loops
    check_ok(
        r#"
        fn loop_int(x: Int) -> Int & {} {
            let mut sum = 0;
            while sum < 10 {
                sum = sum + x;
            };
            sum
        }
    "#,
    );
}

#[test]
fn multiple_capabilities_independent() {
    // Two different capabilities used independently — valid
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        extern fn fetch(net: NetCap, url: String) -> String & {Net};
        fn both(fs: FsCap, net: NetCap) -> () & {Fs, Net} {
            read_file(fs, "a.txt");
            fetch(net, "http://example.com");
            ()
        }
    "#,
    );
}

#[test]
fn match_arms_consistent_use() {
    // All match arms use the capability — consistent, valid
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        enum Option<T> { Some(T), None }
        fn match_use(opt: Option<Int>, fs: FsCap) -> () & {Fs} {
            match opt {
                Option::Some(x) => use_cap(fs),
                Option::None => use_cap(fs),
            }
        }
    "#,
    );
}

#[test]
fn match_arms_consistent_no_use() {
    // No arm uses the capability — consistent. Use after match is valid.
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        enum Option<T> { Some(T), None }
        fn match_no_use(opt: Option<Int>, fs: FsCap) -> () & {Fs} {
            match opt {
                Option::Some(x) => (),
                Option::None => (),
            };
            use_cap(fs)
        }
    "#,
    );
}

#[test]
fn recursive_single_use_per_branch() {
    // Recursive call: each branch uses the cap at most once — valid
    check_ok(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        fn countdown(fs: FsCap, n: Int) -> () & {Fs} {
            if n > 0 {
                countdown(fs, n - 1)
            } else {
                log(fs, "done")
            }
        }
    "#,
    );
}

#[test]
fn cap_passed_to_single_extern() {
    // Direct pass-through to a single extern call — valid
    check_ok(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        fn wrapper(fs: FsCap) -> () & {Fs} {
            log(fs, "hello")
        }
    "#,
    );
}

#[test]
fn multiple_caps_each_used_once() {
    // Three caps, each used exactly once in sequence — valid
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> () & {Fs};
        extern fn do_net(net: NetCap) -> () & {Net};
        extern fn do_time(time: TimeCap) -> () & {Time};
        fn all_three(fs: FsCap, net: NetCap, time: TimeCap) -> () & {Fs, Net, Time} {
            do_fs(fs);
            do_net(net);
            do_time(time)
        }
    "#,
    );
}

#[test]
fn cap_used_in_nested_if() {
    // Cap used deep in nested if — each execution path uses it at most once
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn nested(fs: FsCap, a: Bool, b: Bool) -> () & {Fs} {
            if a {
                if b {
                    use_cap(fs)
                } else {
                    use_cap(fs)
                }
            } else {
                use_cap(fs)
            }
        }
    "#,
    );
}

// ============================================================================
// LET-BINDING TRANSFER TESTS — capability ownership flows through let
// ============================================================================

#[test]
fn let_transfer_then_use() {
    // fs transferred to a via let; a used once — valid
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn let_transfer(fs: FsCap) -> () & {Fs} {
            let a = fs;
            use_cap(a)
        }
    "#,
    );
}

#[test]
fn shadow_transfer() {
    // fs transferred to new binding with same name — valid
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn shadow(fs: FsCap) -> () & {Fs} {
            let fs = fs;
            use_cap(fs)
        }
    "#,
    );
}

#[test]
fn let_transfer_drop_original() {
    // fs transferred to a; a is unused (dropped) — valid (affine = at-most-once)
    check_ok(
        r#"
        fn let_drop(fs: FsCap) -> () & {} {
            let a = fs;
            ()
        }
    "#,
    );
}

#[test]
fn rebind_then_use_original_error() {
    // fs transferred to a; then fs used again — error (already consumed by let)
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn rebind_bad(fs: FsCap) -> () & {Fs} {
            let a = fs;
            use_cap(fs)
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use error after let transfer, got: {err}"
    );
}

#[test]
fn let_transfer_chain() {
    // fs -> a -> b, then b used — valid chain of transfers
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn chain(fs: FsCap) -> () & {Fs} {
            let a = fs;
            let b = a;
            use_cap(b)
        }
    "#,
    );
}

#[test]
fn let_transfer_chain_use_middle_error() {
    // fs -> a -> b; then a used — error (a was consumed by let b = a)
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn chain_bad(fs: FsCap) -> () & {Fs} {
            let a = fs;
            let b = a;
            use_cap(a)
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected error using consumed intermediate, got: {err}"
    );
}

// ============================================================================
// SHADOWING TESTS — binding_types must track by generation, not just name
// ============================================================================

#[test]
fn shadow_affine_with_unrestricted_still_tracks() {
    // Shadow FsCap with Int, then use original FsCap twice.
    // The original fs (gen1) is consumed by `let a = fs;`
    // Then fs is shadowed with Int (gen2).
    // Then we use a (FsCap) twice — MUST fail.
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn shadow_bad(fs: FsCap) -> () & {Fs} {
            let a = fs;
            let fs = 42;
            use_cap(a);
            use_cap(a)
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use error after shadow, got: {err}"
    );
}

// ============================================================================
// MATCH PATTERN BINDING TESTS — pattern-bound caps must be tracked as affine
// ============================================================================

#[test]
fn match_pattern_single_use_ok() {
    // Capability bound via match pattern, used once — valid
    check_ok(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn match_bind(fs: FsCap) -> () & {Fs} {
            match fs {
                x => use_cap(x),
            }
        }
    "#,
    );
}

#[test]
fn match_pattern_double_use_error() {
    // Capability bound via match pattern, used twice — MUST fail
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn match_double(fs: FsCap) -> () & {Fs} {
            match fs {
                x => { use_cap(x); use_cap(x) },
            }
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use error via match pattern, got: {err}"
    );
}

#[test]
fn match_tuple_destructure_double_use_error() {
    // Capability extracted from tuple via match, used twice — MUST fail
    let err = check_err(
        r#"
        extern fn use_fs(fs: FsCap) -> () & {Fs};
        extern fn use_net(net: NetCap) -> () & {Net};
        fn tuple_match(fs: FsCap, net: NetCap) -> () & {Fs, Net} {
            let pair = (fs, net);
            match pair {
                (a, b) => { use_fs(a); use_fs(a) },
            }
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use error via tuple destructure, got: {err}"
    );
}

// ============================================================================
// EXPLOIT PROBE: Generic ADT capability laundering
// ============================================================================

#[test]
fn generic_adt_with_cap_is_affine() {
    // Smuggler<FsCap> must be affine — copying it duplicates the capability.
    let err = check_err(
        r#"
        enum Smuggler<T> { Hold(T), Empty }
        extern fn use_fs(fs: FsCap) -> () & {Fs};

        fn launder(fs: FsCap) -> () & {Fs} {
            let wrapped = Smuggler::Hold(fs);
            let copy1 = wrapped;
            let copy2 = wrapped;

            match copy1 {
                Smuggler::Hold(x) => use_fs(x),
                Smuggler::Empty => (),
            };
            match copy2 {
                Smuggler::Hold(y) => use_fs(y),
                Smuggler::Empty => (),
            }
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use error on wrapped ADT containing cap, got: {err}"
    );
}

#[test]
fn generic_adt_without_cap_is_unrestricted() {
    // Smuggler<Int> is unrestricted — copying is fine.
    check_ok(
        r#"
        enum Smuggler<T> { Hold(T), Empty }
        fn ok() -> () & {} {
            let w = Smuggler::Hold(42);
            let a = w;
            let b = w;
            ()
        }
    "#,
    );
}

#[test]
fn nested_generic_adt_with_cap_is_affine() {
    // Outer<FsCap> contains Smuggler<FsCap> — both are affine.
    let err = check_err(
        r#"
        enum Smuggler<T> { Hold(T), Empty }
        enum Outer<T> { Inner(Smuggler<T>) }
        fn bad(fs: FsCap) -> () & {} {
            let nested = Outer::Inner(Smuggler::Hold(fs));
            let a = nested;
            let b = nested;
            ()
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use error on nested ADT containing cap, got: {err}"
    );
}

// ============================================================================
// CLOSURE CAP CAPTURE BAN — closures don't exist yet, so caps can't leak
// ============================================================================

#[test]
fn closure_syntax_not_supported() {
    // Closures/lambdas are not yet a language feature. This is the current
    // enforcement mechanism for the closure-cap-capture ban: you can't write
    // a closure that captures a capability because closures don't parse.
    //
    // When closures are added, Ty::kind() must propagate affinity through
    // Arrow types with captured environments (see comment in ty.rs).
    let src = r#"
        fn bad(fs: FsCap) -> () & {Fs} {
            let action = || { fs };
            action()
        }
    "#;
    // This must fail to parse — closure syntax doesn't exist
    assert!(
        strata_parse::parse_str("<test>", src).is_err(),
        "Closure syntax should not parse; if this passes, closures were added \
         and the move checker needs closure-cap-capture enforcement"
    );
}

// ============================================================================
// BORROW TESTS — borrowing does not consume, even in loops
// ============================================================================

#[test]
fn borrow_in_loop_ok() {
    // Borrowing inside a while loop is allowed — borrows are repeatable.
    // (Consuming a cap in a loop would be an error, but borrowing is fine.)
    check_ok(
        r#"
        extern fn check_status(net: &NetCap) -> Bool & {Net};
        fn poll(net: NetCap) -> () & {Net} {
            let mut done = false;
            while !done {
                done = check_status(&net);
            }
        }
    "#,
    );
}

// TEST: closure_capturing_cap_is_affine
// When closures are added, a closure that captures a cap must be affine.
// This means the closure can be defined once and called at most once.
//
// TODO: uncomment when closures are implemented
//
// #[test]
// fn closure_capturing_cap_is_affine() {
//     let err = check_err(r#"
//         extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
//         fn bad(fs: FsCap) -> () & {Fs} {
//             let action = |path: String| { read_file(fs, path) };
//             action("/a");  // OK — first call
//             action("/b");  // ERROR — closure is affine, already consumed
//         }
//     "#);
//     assert!(err.contains("already been used"),
//         "Expected affine closure double-call error, got: {err}");
// }
//
// #[test]
// fn closure_capturing_unrestricted_is_unrestricted() {
//     // A closure that only captures unrestricted values can be called freely
//     check_ok(r#"
//         fn ok() -> Int & {} {
//             let x = 42;
//             let action = || { x + 1 };
//             action() + action()
//         }
//     "#);
// }

// ============================================================================
// NEGATIVE TESTS — Invalid programs that the move checker should reject
// ============================================================================

#[test]
fn double_use_error() {
    // Using a capability twice is rejected
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn double(fs: FsCap) -> () & {Fs} {
            use_cap(fs);
            use_cap(fs)
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use error, got: {err}"
    );
}

#[test]
fn capability_in_loop_error() {
    // Using a capability inside a loop is rejected
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn looped(fs: FsCap) -> () & {Fs} {
            while true {
                use_cap(fs)
            }
        }
    "#,
    );
    assert!(
        err.contains("single-use capability") || err.contains("loop"),
        "Expected loop-use error, got: {err}"
    );
}

#[test]
fn use_after_branch_consumption_error() {
    // fs consumed in one branch, then used again after — error
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn branch_one(fs: FsCap, c: Bool) -> () & {Fs} {
            if c { use_cap(fs) } else { () };
            use_cap(fs)
        }
    "#,
    );
    assert!(
        err.contains("already"),
        "Expected post-branch error, got: {err}"
    );
}

#[test]
fn match_inconsistent_then_use_error() {
    // Only one match arm uses fs; post-match use is an error
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        enum Option<T> { Some(T), None }
        fn match_bad(opt: Option<Int>, fs: FsCap) -> () & {Fs} {
            match opt {
                Option::Some(x) => use_cap(fs),
                Option::None => (),
            };
            use_cap(fs)
        }
    "#,
    );
    assert!(
        err.contains("already"),
        "Expected post-match error, got: {err}"
    );
}

#[test]
fn left_to_right_arg_double_use_error() {
    // Same cap passed as two arguments — second arg sees it already consumed
    let err = check_err(
        r#"
        extern fn two_caps(a: FsCap, b: FsCap) -> () & {Fs};
        fn double_arg(fs: FsCap) -> () & {Fs} {
            two_caps(fs, fs)
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use in args error, got: {err}"
    );
}

#[test]
fn double_use_different_calls_error() {
    // Cap used in two sequential calls — error
    let err = check_err(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        fn double_call(fs: FsCap) -> () & {Fs} {
            log(fs, "first");
            log(fs, "second")
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use error, got: {err}"
    );
}

#[test]
fn recursive_double_use_error() {
    // Using cap AND passing to recursive call in same branch — error
    let err = check_err(
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
    assert!(
        err.contains("already been used"),
        "Expected double-use error, got: {err}"
    );
}

#[test]
fn cap_in_while_condition_error() {
    // Cap used even in the loop condition position is checked
    // (the condition is re-evaluated each iteration, but the overall
    // body is checked in loop mode)
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn loop_body(fs: FsCap) -> () & {Fs} {
            let mut i = 0;
            while i < 10 {
                use_cap(fs);
                i = i + 1;
            }
        }
    "#,
    );
    assert!(
        err.contains("single-use capability") || err.contains("loop"),
        "Expected loop-use error, got: {err}"
    );
}

#[test]
fn nested_if_double_use_error() {
    // Used in inner if, then used again in outer scope — error
    let err = check_err(
        r#"
        extern fn use_cap(fs: FsCap) -> () & {Fs};
        fn nested_bad(fs: FsCap, a: Bool, b: Bool) -> () & {Fs} {
            if a {
                use_cap(fs)
            } else {
                ()
            };
            use_cap(fs)
        }
    "#,
    );
    assert!(
        err.contains("already"),
        "Expected post-branch error, got: {err}"
    );
}
