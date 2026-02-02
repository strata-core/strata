# Strata: In Progress

**Last Updated:** February 2, 2026
**Current Version:** v0.0.6
**Current Focus:** Ready to merge to main
**Progress:** Issue 006 complete ✅

---

## Recent Completions

### Issue 006: Blocks & Control Flow ✅
**Completed:** February 2, 2026

**Goal:** Add blocks, if/else, while, return statements, and mutable bindings.

**What was built:**
- Block expressions with tail/semicolon semantics
- If/else expressions with branch type unification
- While loops with proper control flow
- Return statements with propagation through nested blocks
- Mutable let bindings (`let mut x = ...`)
- Assignment statements respecting mutability
- Scope stack evaluator with proper lexical scoping
- Closures with captured environments
- Self-recursion and mutual recursion support

**Test stats:** 133 tests passing (84 new tests added)

**Key features verified:**
- Nested block return propagation
- 1000-iteration while loops (no stack overflow)
- Mutual recursion (`is_even`/`is_odd` pattern)
- Variable shadowing in nested scopes

**Example programs:**
```strata
fn max(a: Int, b: Int) -> Int {
    if a > b { a } else { b }
}

fn factorial(n: Int) -> Int {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

fn sum_to(n: Int) -> Int {
    let mut total = 0;
    let mut i = 1;
    while i <= n {
        total = total + i;
        i = i + 1;
    };
    total
}
```

---

### Issue 005-b: Soundness & Trustworthiness Hardening ✅
**Completed:** February 1, 2026

**Goal:** Make the typechecker/inference **sound enough to build on**, **deterministic enough to debug**, and **honest enough to trust** before adding blocks/control-flow in Issue 006.

**Status:** ✅ **COMPLETE - All 7 fixes shipped**

**What was fixed:**

1. ✅ **Unknown identifiers now error properly**
   - Fixed: `InferCtx::infer_expr` returns `Result<Ty, InferError>`
   - Added: `InferError::UnknownVariable` variant
   - Test: `type_error_unknown_variable`
   - Impact: Type system no longer lies about unknown identifiers

2. ✅ **Real unification errors (no placeholders)**
   - Fixed: Thread `unifier::TypeError` through to `checker::TypeError`
   - Added: `unifier_error_to_type_error()` conversion helper
   - Impact: Errors show actual types ("Int vs Bool" not "Unit vs Unit")

3. ✅ **Correct generalization boundaries**
   - Fixed: Compute actual environment free vars before generalizing
   - Added: `free_vars_scheme()` and `free_vars_env()` functions
   - Impact: Sound polymorphism, ready for nested scopes

4. ✅ **Numeric constraints on arithmetic**
   - Fixed: Arithmetic operators (+, -, *, /) constrain to Int
   - Fixed: Unary negation (-) constrains to Int
   - Tests: `type_error_bool_arithmetic`, `type_error_neg_bool`
   - Impact: `true + true` now correctly errors

5. ✅ **Two-pass module checking**
   - Fixed: Predeclare all functions in pass 1, check bodies in pass 2
   - Added: `extract_fn_signature()` helper
   - Tests: `forward_reference_works`, `mutual_recursion_predeclared`
   - Impact: Forward references work, recursion enabled

6. ✅ **Minimal constraint provenance**
   - Fixed: Added `Span` field to `Constraint::Equal`
   - Updated: All constraint generation sites include span
   - Impact: Foundation for better error messages in Issue 006

7. ✅ **Determinism audit - PASS**
   - Audited: All HashMap iteration for non-deterministic behavior
   - Documented: `docs/DETERMINISM_AUDIT.md`
   - Impact: Stable, reproducible behavior - no flaky CI

**Test stats:** 49 tests passing (44 existing + 5 new)

**Key learnings:**
- Post-completion soundness review is essential
- Cost asymmetry: Hardening now prevents debugging nightmares later
- Type system must not lie - trust is everything
- Foundation first: don't build on broken abstractions

---

### Issue 005: Functions & Type Inference ✅
**Completed:** February 1, 2026

**What was built:**
- Constraint-based type inference system
- InferCtx for fresh variable generation and constraint collection
- Solver for constraint solving via unification
- Generalization (∀ introduction) and instantiation (∀ elimination)
- Function declarations with multi-param arrows
- Function call type checking
- Polymorphic type schemes (let-polymorphism)
- Higher-order function support
- 10 comprehensive function tests
- Working CLI examples (add.strata, type_error.strata)

---

## Next Up

### Issue 007: ADTs & Pattern Matching
**Estimated start:** Mid-February 2026

**Why now:** Control flow complete. Ready to add algebraic data types.

**Scope:**
- Struct definitions: `struct Point { x: Int, y: Int }`
- Enum definitions: `enum Option<T> { Some(T), None }`
- Generic type parameters
- Match expressions with pattern matching
- Exhaustiveness checking
- Basic traits

**Example:**
```strata
struct Point { x: Int, y: Int }

enum Option<T> {
    Some(T),
    None,
}

fn unwrap_or<T>(opt: Option<T>, default: T) -> T {
    match opt {
        Some(x) => x,
        None => default,
    }
}
```

---

## Development Lessons

### Learnings
- **Soundness review is essential:** Initial "completion" missed critical bugs
- **Cost asymmetry:** Hardening now prevents debugging nightmares later
- **Foundation first:** Don't build on broken abstractions
- **Trust is everything:** Type system must not lie
- **Velocity with correctness:** Ship fast, but ship it sound

### Development Philosophy
- **Soundness over speed** — A type system that lies is worse than no type system
- **Foundation integrity** — Fix bugs before they compound
- **Post-completion review** — Always audit before moving forward
- **Deterministic behavior** — Flaky tests waste more time than they save

---

## Project Statistics

**Codebase:**
- 4 crates (strata-ast, strata-parse, strata-types, strata-cli)
- ~5,000+ lines of Rust code
- 133 tests (all passing)
- 0 clippy warnings (enforced by pre-commit)