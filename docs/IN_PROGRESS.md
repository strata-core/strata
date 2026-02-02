# Strata: In Progress

**Last Updated:** February 1, 2026  
**Current Version:** v0.0.5  
**Current Focus:** Ready to merge to main  
**Progress:** Issue 005 + 005-b complete ✅

---

## Recent Completions

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

### Issue 006: Blocks & Control Flow
**Estimated start:** Early-Mid February 2026

**Why now:** Foundation is sound and trustworthy. Ready to add control flow.

**Scope:**
- Block expressions: `{ stmt; stmt; expr }`
- If/else expressions (branches unify)
- While loops
- Return statements
- Mutable let bindings (arena-based)

**Example:**
```strata
fn max(x: Int, y: Int) -> Int {
    if x > y { x } else { y }
}

fn sum(n: Int) -> Int {
    let mut total = 0;
    let mut i = 0;
    while i < n {
        total = total + i;
        i = i + 1;
    }
    total
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
- ~3,500+ lines of Rust code
- 49 tests (all passing)
- 0 clippy warnings (enforced by pre-commit)