//! Integration tests for capability types (Issue 009)
//!
//! Capability types gate effect usage: a function performing `{Fs}` effects
//! must have `FsCap` in its parameter list. This enforces "no ambient authority."
//!
//! Post-review: The capability check is MANDATORY for all functions and externs
//! with concrete effects. There is no opt-in guard.

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

/// Helper: parse and type-check, expect failure, return error message
fn check_err(src: &str) -> String {
    let module = parse_str("<test>", src).expect("parse failed");
    let mut checker = TypeChecker::new();
    let err = checker
        .check_module(&module)
        .expect_err("expected type error but got OK");
    format!("{err}")
}

// ============================================================================
// POSITIVE TESTS - Valid programs with capabilities
// ============================================================================

#[test]
fn pure_function_needs_no_capabilities() {
    // Pure function with no effects needs no capability params
    check_ok("fn add(x: Int, y: Int) -> Int { x + y }");
}

#[test]
fn function_with_matching_capability() {
    // Function declares {Fs} effect and has FsCap param → OK
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn load(fs: FsCap, p: String) -> String & {Fs} { read_file(fs, p) }
    "#,
    );
}

#[test]
fn function_with_multiple_capabilities() {
    // Function with multiple effects has all matching capabilities
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
fn capability_passthrough_no_effect_used() {
    // A function with FsCap param but no {Fs} effect is valid (pass-through pattern).
    // Unused capabilities are not errors.
    check_ok(
        r#"
        fn passthrough(fs: FsCap, x: Int) -> Int { x + 1 }
    "#,
    );
}

#[test]
fn transitive_capability_flow() {
    // Caller passes capability to callee that uses it
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn inner(fs: FsCap, p: String) -> String & {Fs} { read_file(fs, p) }
        fn outer(fs: FsCap, p: String) -> String & {Fs} { inner(fs, p) }
    "#,
    );
}

#[test]
fn extern_fn_with_cap_param_matching_effect() {
    // Extern fn with FsCap param and {Fs} effect → OK
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
    "#,
    );
}

#[test]
fn extern_fn_pure_no_caps_needed() {
    // Pure extern function needs no capabilities
    check_ok(
        r#"
        extern fn pure_calc(x: Int) -> Int & {};
    "#,
    );
}

#[test]
fn all_five_capability_types() {
    // All five capability types are recognized as valid types
    check_ok(
        r#"
        fn use_all(
            fs: FsCap,
            net: NetCap,
            time: TimeCap,
            rand: RandCap,
            ai: AiCap,
            x: Int
        ) -> Int {
            x
        }
    "#,
    );
}

#[test]
fn capability_type_display() {
    // Capability types should display correctly in error messages
    check_ok(
        r#"
        fn identity(fs: FsCap) -> FsCap { fs }
    "#,
    );
}

// ============================================================================
// NEGATIVE TESTS - Missing capabilities
// ============================================================================

#[test]
fn missing_fs_capability() {
    // Function declares {Fs} effect and has NetCap (wrong cap) but no FsCap
    let err = check_err(
        r#"
        fn bad(net: NetCap) -> String & {Fs} { "hello" }
    "#,
    );
    assert!(
        err.contains("FsCap"),
        "Expected error mentioning FsCap, got: {err}"
    );
    assert!(
        err.contains("bad"),
        "Expected error mentioning function name 'bad', got: {err}"
    );
}

#[test]
fn missing_one_of_two_capabilities() {
    // Function needs {Fs, Net} but only has FsCap → error for missing NetCap
    let err = check_err(
        r#"
        fn bad(fs: FsCap) -> String & {Fs, Net} { "hello" }
    "#,
    );
    assert!(
        err.contains("NetCap"),
        "Expected error mentioning NetCap, got: {err}"
    );
}

#[test]
fn error_message_contains_help_text() {
    // Error messages should contain help text about adding capability param
    let err = check_err(
        r#"
        fn bad(net: NetCap) -> String & {Fs} { "hello" }
    "#,
    );
    assert!(
        err.contains("FsCap"),
        "Expected help text mentioning FsCap, got: {err}"
    );
}

#[test]
fn extern_fn_with_effects_but_no_cap_error() {
    // Extern fn declares {Fs} but has no FsCap param → error
    let err = check_err(
        r#"
        extern fn bad(path: String) -> String & {Fs};
    "#,
    );
    assert!(
        err.contains("FsCap"),
        "Expected error mentioning FsCap, got: {err}"
    );
}

#[test]
fn extern_fn_with_cap_but_missing_other_effect_cap() {
    // Extern fn has NetCap but declares {Fs, Net} effects → missing FsCap
    let err = check_err(
        r#"
        extern fn bad(net: NetCap, path: String) -> String & {Fs, Net};
    "#,
    );
    assert!(
        err.contains("FsCap"),
        "Expected error mentioning FsCap, got: {err}"
    );
}

#[test]
fn fn_with_zero_caps_and_concrete_effects_error() {
    // Critical regression: A function with zero cap params but concrete effects
    // MUST be rejected. This is the core "no ambient authority" rule.
    let err = check_err(
        r#"
        fn bad() -> () & {Fs} { () }
    "#,
    );
    assert!(
        err.contains("FsCap") || err.contains("capability"),
        "Expected missing capability error for zero-cap function, got: {err}"
    );
}

#[test]
fn adversarial_zero_caps_entire_call_chain() {
    // The adversarial test from the post-review spec:
    // Zero FsCap anywhere in the entire call chain. Must be rejected.
    let err = check_err(
        r#"
        extern fn raw_delete_disk(fs: FsCap) -> () & {Fs};
        fn middle_man() -> () & {Fs} { () }
        fn main_fn() -> () & {Fs} { middle_man() }
    "#,
    );
    assert!(
        err.contains("FsCap") || err.contains("capability"),
        "Expected missing capability error in call chain with no caps, got: {err}"
    );
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn function_with_inferred_effects_and_cap() {
    // Function has FsCap param and inferred {Fs} effects (no annotation) → OK
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn load(fs: FsCap, p: String) -> String { read_file(fs, p) }
    "#,
    );
}

#[test]
fn function_with_inferred_effects_missing_cap() {
    // Function has no caps and inferred {Fs} effects → error
    // (Uses the cap check on functions with zero cap params)
    let err = check_err(
        r#"
        fn bad() -> Int & {Fs} { 42 }
    "#,
    );
    assert!(
        err.contains("FsCap"),
        "Expected error mentioning FsCap for inferred effects, got: {err}"
    );
}

#[test]
fn hof_with_effectful_callback_needs_no_cap() {
    // A HOF that takes an effectful callback should NOT need capabilities
    // for the callback's effects, only for its own concrete effects.
    // The HOF's effects from the callback are parametric (open tail variable).
    // Using single-arg extern for HOF compatibility.
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};
        fn apply(f, x) { f(x) }
        fn use_it(fs: FsCap) -> String & {Fs} { apply(do_fs, fs) }
    "#,
    );
}

#[test]
fn hof_with_cap_and_own_effect() {
    // HOF that has its own concrete effect AND takes a callback.
    // The HOF needs FsCap for its own {Fs} effect, but NOT for the callback's effects.
    check_ok(
        r#"
        extern fn log(fs: FsCap, msg: String) -> () & {Fs};
        fn apply_with_log(fs: FsCap, f, x) & {Fs} {
            log(fs, "calling");
            f(x)
        }
    "#,
    );
}

#[test]
fn capability_in_adt_field_rejected() {
    // Capabilities cannot be stored in ADT fields (until linear types)
    let err = check_err(
        r#"
        struct HasCap { cap: FsCap }
    "#,
    );
    assert!(
        err.contains("FsCap") || err.contains("capability"),
        "Expected error about capability in ADT, got: {err}"
    );
}

#[test]
fn capability_in_let_binding_is_transfer() {
    // Let-binding a capability is now a transfer (move), not an error.
    // The move checker enforces single-use: fs is transferred to cap,
    // and cap is dropped (unused is OK for affine types).
    check_ok(
        r#"
        fn transfer(fs: FsCap) -> () & {} {
            let cap = fs;
            ()
        }
    "#,
    );
}

#[test]
fn pure_extern_with_cap_param_ok() {
    // An extern fn with a cap param but no effects is fine (weird but valid)
    check_ok(
        r#"
        extern fn weird(fs: FsCap) -> Int & {};
    "#,
    );
}

#[test]
fn cap_type_unification_mismatch() {
    // FsCap and NetCap should not unify
    let err = check_err(
        r#"
        fn bad(fs: FsCap) -> NetCap { fs }
    "#,
    );
    assert!(
        err.contains("mismatch") || err.contains("Mismatch"),
        "Expected type mismatch error, got: {err}"
    );
}

#[test]
fn cap_type_used_as_return_type() {
    // Capability type can be used as return type (for pass-through)
    check_ok(
        r#"
        fn passthrough(fs: FsCap) -> FsCap { fs }
    "#,
    );
}

#[test]
fn multiple_cap_params_same_kind() {
    // Multiple cap params of the same kind is valid (redundant but not wrong)
    check_ok(
        r#"
        extern fn read_file(fs: FsCap, path: String) -> String & {Fs};
        fn double_fs(fs1: FsCap, fs2: FsCap, p: String) -> String & {Fs} {
            read_file(fs1, p)
        }
    "#,
    );
}

// ============================================================================
// CAPS-IN-ADT BAN REGRESSION TESTS (Defense-in-Depth for Generic ADTs)
//
// These tests verify the caps-in-ADT ban covers generic ADT instantiation
// with capability types. When the ban is eventually lifted (post-linear types),
// the move checker's generic field resolution must correctly track affinity.
// ============================================================================

#[test]
fn caps_in_direct_adt_field_banned() {
    // Direct capability type in an ADT field is rejected at registration time.
    // This is the existing enforcement mechanism.
    let err = check_err(
        r#"
        enum Wrapper { Hold(FsCap) }
    "#,
    );
    assert!(
        err.contains("capability") || err.contains("FsCap"),
        "Expected error about capability in ADT field, got: {err}"
    );
}

#[test]
fn generic_adt_with_cap_single_use_ok() {
    // Generic ADTs instantiated with caps are NOT caught by the caps-in-ADT
    // registration ban (ban checks field types at definition time where fields
    // are type variables). However, Ty::kind() propagates affinity through ADT
    // type args, so MyOption<FsCap> is Kind::Affine. This means:
    // - Single use (constructing and returning) is fine
    // - Duplication (let a = x; let b = x;) is caught by the move checker
    check_ok(
        r#"
        enum MyOption<T> { Some(T), None }
        fn wraps_cap(fs: FsCap) -> MyOption<FsCap> & {} {
            MyOption::Some(fs)
        }
    "#,
    );
}

#[test]
fn generic_adt_cap_field_double_use_after_extract() {
    // When a cap is extracted from a generic ADT via pattern match,
    // the extracted binding must be affine. Double use is rejected.
    let err = check_err(
        r#"
        enum Box<T> { Val(T) }
        extern fn use_fs(fs: FsCap) -> () & {Fs};
        fn bad(fs: FsCap) -> () & {Fs} {
            let b = Box::Val(fs);
            match b {
                Box::Val(inner) => { use_fs(inner); use_fs(inner) },
            }
        }
    "#,
    );
    assert!(
        err.contains("already been used"),
        "Expected double-use error on cap extracted from generic ADT, got: {err}"
    );
}

// ============================================================================
// ADT NAME SHADOWING TESTS
// ============================================================================

#[test]
fn struct_shadowing_capability_name_rejected() {
    let err = check_err(
        r#"
        struct FsCap { x: Int }
    "#,
    );
    assert!(
        err.contains("reserved") || err.contains("FsCap"),
        "Expected reserved name error, got: {err}"
    );
}

#[test]
fn enum_shadowing_capability_name_rejected() {
    let err = check_err(
        r#"
        enum NetCap { A, B }
    "#,
    );
    assert!(
        err.contains("reserved") || err.contains("NetCap"),
        "Expected reserved name error, got: {err}"
    );
}

// ============================================================================
// ERGONOMIC HINT TESTS
// ============================================================================

#[test]
fn hint_fscap_in_effect_annotation() {
    // Writing FsCap in an effect annotation should hint toward Fs
    let err = check_err(
        r#"
        fn bad() -> Int & {FsCap} { 0 }
    "#,
    );
    assert!(
        err.contains("Did you mean"),
        "Expected ergonomic hint for FsCap in effect position, got: {err}"
    );
}

#[test]
fn hint_fs_as_parameter_type() {
    // Writing Fs as a parameter type should hint toward FsCap
    let err = check_err(
        r#"
        fn bad(fs: Fs) -> Int { 0 }
    "#,
    );
    assert!(
        err.contains("Did you mean") || err.contains("FsCap"),
        "Expected ergonomic hint for Fs in type position, got: {err}"
    );
}

// ============================================================================
// ADVERSARIAL SECURITY TESTS
//
// These tests verify that the type/effect system cannot be tricked into
// erasing or laundering effects through various language features.
// If ANY of these tests that are expected to fail instead succeed,
// that indicates a critical soundness bug.
// ============================================================================

// ---------------------------------------------------------------------------
// Attack Vector 1: Higher-Order Function Effect Laundering
// ---------------------------------------------------------------------------

#[test]
fn adversarial_hof_typed_pure_callback_rejects_effectful_fn() {
    // Attack: Declare a HOF that takes a pure callback via explicit type annotation,
    // then pass an effectful function to launder its effects.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        fn apply_pure(f: fn(FsCap) -> String, x: FsCap) -> String { f(x) }

        fn evil(fs: FsCap) -> String {
            apply_pure(do_fs, fs)
        }
    "#,
    );
    assert!(
        err.contains("effect")
            || err.contains("Effect")
            || err.contains("mismatch")
            || err.contains("Mismatch"),
        "Expected effect mismatch error when passing effectful fn as pure callback, got: {err}"
    );
}

#[test]
fn adversarial_hof_untyped_callback_propagates_effects() {
    // An untyped HOF correctly propagates effects from its callback.
    // A pure function calling this should fail.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        fn apply(f, x) { f(x) }

        fn evil(fs: FsCap) -> String & {} {
            apply(do_fs, fs)
        }
    "#,
    );
    assert!(
        err.contains("Fs") || err.contains("effect") || err.contains("Effect"),
        "Expected effect error: pure fn cannot transitively use Fs via HOF, got: {err}"
    );
}

#[test]
fn adversarial_hof_untyped_callback_infers_effects_correctly() {
    // Positive control: untyped HOF with effectful callback infers effects.
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        fn apply(f, x) { f(x) }

        fn caller(fs: FsCap) -> String & {Fs} {
            apply(do_fs, fs)
        }
    "#,
    );
}

// ---------------------------------------------------------------------------
// Attack Vector 2: Effectful Closures in ADT Variants
// ---------------------------------------------------------------------------

#[test]
fn adversarial_adt_stores_effectful_fn_rejected() {
    // Attack: Store an effectful function in an enum variant.
    // An ADT field with fn(FsCap) -> String is rejected because it contains
    // a capability type in the function parameter. This is correct: ADTs
    // cannot store capability-accepting functions (prevents capability laundering).
    let err = check_err(
        r#"
        enum Sneaky { Act(fn(FsCap) -> String) }
    "#,
    );
    assert!(
        err.contains("FsCap") || err.contains("capability") || err.contains("Capability"),
        "Expected error about capability in ADT, got: {err}"
    );
}

#[test]
fn adversarial_adt_effect_annotation_silently_dropped() {
    // Even with effects on arrow type in ADT, rejected due to capability in type.
    let err = check_err(
        r#"
        enum Sneaky { Act(fn(FsCap) -> String & {Fs}) }
    "#,
    );
    assert!(
        err.contains("FsCap") || err.contains("capability") || err.contains("Capability"),
        "Expected error about capability in ADT, got: {err}"
    );
}

#[test]
fn adversarial_adt_pure_fn_field_stores_pure_fn_ok() {
    // Control test: Storing a pure function in an ADT field typed as pure works.
    check_ok(
        r#"
        fn double(x: Int) -> Int { x + x }

        enum Wrapper { Func(fn(Int) -> Int) }

        fn use_it() -> Int {
            let w = Wrapper::Func(double);
            match w {
                Wrapper::Func(f) => f(21),
            }
        }
    "#,
    );
}

#[test]
fn adversarial_generic_adt_preserves_effects_through_type_param() {
    // Generic ADT preserves effectful fn type through type parameter.
    // Calling in a pure context should fail.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        enum Box<T> { Val(T) }

        fn evil(fs: FsCap) -> String & {} {
            let b = Box::Val(do_fs);
            match b {
                Box::Val(f) => f(fs),
            }
        }
    "#,
    );
    assert!(
        err.contains("Fs") || err.contains("effect") || err.contains("Effect"),
        "Expected effect error: generic ADT should preserve effectful fn type, got: {err}"
    );
}

#[test]
fn adversarial_generic_adt_effectful_fn_infers_correctly() {
    // Control: Generic ADT with effectful function, caller correctly annotates effects.
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        enum Box<T> { Val(T) }

        fn caller(fs: FsCap) -> String & {Fs} {
            let b = Box::Val(do_fs);
            match b {
                Box::Val(f) => f(fs),
            }
        }
    "#,
    );
}

// ---------------------------------------------------------------------------
// Attack Vector 3: Let-Generalization Effect Erasure
// ---------------------------------------------------------------------------

#[test]
fn adversarial_let_binding_preserves_effects() {
    // Let-bound effectful function must preserve effects.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        fn evil(fs: FsCap) -> String & {} {
            let f = do_fs;
            f(fs)
        }
    "#,
    );
    assert!(
        err.contains("Fs") || err.contains("effect") || err.contains("Effect"),
        "Expected effect error: let-bound effectful fn must preserve effects, got: {err}"
    );
}

#[test]
fn adversarial_let_binding_effectful_fn_infers_correctly() {
    // Let-binding the result of an effectful call with correct annotation.
    // The move checker tracks fs: it's consumed by the call to do_fs,
    // and the result (String) is unrestricted.
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        fn caller(fs: FsCap) -> String & {Fs} {
            let result = do_fs(fs);
            result
        }
    "#,
    );
}

#[test]
fn adversarial_let_generalization_at_module_level() {
    // Module-level generalization does NOT erase effects.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        let f = do_fs;

        fn evil(fs: FsCap) -> String & {} {
            f(fs)
        }
    "#,
    );
    assert!(
        err.contains("Fs") || err.contains("effect") || err.contains("Effect"),
        "Expected effect error: module-level let generalization must not erase effects, got: {err}"
    );
}

#[test]
fn adversarial_let_generalization_module_level_infers_correctly() {
    // Control: Module-level let binding of a pure value with correct effect propagation.
    // (Direct `let f = do_fs` at module level is rejected because arrow type contains
    //  FsCap. Instead, test that effects propagate correctly through function calls.)
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};
        fn apply(f, x) { f(x) }
        fn caller(fs: FsCap) -> String & {Fs} {
            apply(do_fs, fs)
        }
    "#,
    );
}

// ============================================================================
// HARDENING TESTS — External Review Exploits
//
// These tests target specific attack vectors identified by independent
// security and type-theory reviewers. If tests 1, 2, or 4 compile
// without error, that indicates a critical effect inference bug.
// ============================================================================

#[test]
fn hardening_exploit_1_polymorphic_hof_open_tail_smuggling() {
    // Exploit 1: A polymorphic HOF has an open effect tail that gets
    // instantiated to concrete effects at the call site. The zero-cap caller
    // should fail because the concrete effects ({Fs}) bubble up through
    // the open tail and require FsCap at the caller.
    //
    // apply: ∀E. (t -> r & E, t) -> r & E
    // When called with do_fs, E is instantiated to {Fs}.
    // smuggle() declares & {} but transitively uses {Fs} → must error.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        fn apply(f, x) { f(x) }

        fn smuggle(fs: FsCap) -> String & {} {
            apply(do_fs, fs)
        }
    "#,
    );
    assert!(
        err.contains("Fs") || err.contains("effect") || err.contains("Effect"),
        "CRITICAL: Polymorphic HOF open tail smuggled {{Fs}} past zero-effect caller, got: {err}"
    );
}

#[test]
fn hardening_exploit_2_recursive_open_tail_callback() {
    // Exploit 2: A recursive function with an open-tail callback parameter.
    // A zero-cap caller passes an effectful extern. The effects from the callback
    // must propagate to the caller; without matching effects the caller must fail.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        fn repeat(f, x, n: Int) -> String {
            if n <= 0 {
                x
            } else {
                repeat(f, f(x), n - 1)
            }
        }

        fn smuggle(fs: FsCap) -> String & {} {
            repeat(do_fs, fs, 3)
        }
    "#,
    );
    assert!(
        err.contains("Fs") || err.contains("effect") || err.contains("Effect"),
        "CRITICAL: Recursive open-tail callback smuggled {{Fs}} past zero-effect caller, got: {err}"
    );
}

#[test]
fn hardening_exploit_3_hof_own_effect_plus_open_callback_tail() {
    // Exploit 3: A HOF that has its OWN concrete effect ({Fs}) plus an open callback
    // tail from calling its parameter. The function needs FsCap for its own {Fs}
    // effect, but should NOT need caps for the callback's parametric effects.
    // This should compile successfully.
    check_ok(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        fn pure_fn(n: Int) -> String { "ok" }

        fn apply_with_log(fs: FsCap, f, x) -> String & {Fs} {
            let _log = do_fs(fs);
            f(x)
        }

        fn caller(fs: FsCap, x: Int) -> String & {Fs} {
            apply_with_log(fs, pure_fn, x)
        }
    "#,
    );
}

#[test]
fn hardening_exploit_4_mutual_recursion_scc_introduces_effect() {
    // Exploit 4: Mutual recursion SCC where one function introduces {Fs}.
    // pong has FsCap (so it can call ping) but declares & {} (pure).
    // Since pong transitively calls ping which does {Fs}, pong's declared
    // effects must include {Fs}. Declaring & {} should fail.
    let err = check_err(
        r#"
        extern fn do_fs(fs: FsCap) -> String & {Fs};

        fn ping(fs: FsCap, n: Int) -> String & {Fs} {
            if n <= 0 { do_fs(fs) } else { pong(fs, n - 1) }
        }

        fn pong(fs: FsCap, n: Int) -> String & {} {
            if n <= 0 { "done" } else { ping(fs, n - 1) }
        }
    "#,
    );
    assert!(
        err.contains("Fs")
            || err.contains("effect")
            || err.contains("Effect")
            || err.contains("FsCap")
            || err.contains("capability")
            || err.contains("Capability"),
        "CRITICAL: Mutual recursion SCC allowed {{Fs}} effect without matching annotation on pong, got: {err}"
    );
}
