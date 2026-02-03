# Strata: In Progress

**Last Updated:** February 3, 2026
**Current Version:** v0.0.7.0
**Current Focus:** Issue 007 Design Review
**Progress:** Issues 001-006 + Hardening complete ✅

---

## Current Status

### v0.0.7.0 Tagged ✅
**Released:** February 3, 2026

Clean CI baseline for Issue 007:
- Fixed clippy warnings (push_str → push, removed dead code, fixed approx_constant)
- All 163 tests passing
- Zero warnings on `cargo clippy --workspace --all-targets -- -D warnings`

---

## Recent Completions

### Issue 006-Hardening ✅
**Completed:** February 2-3, 2026

**Goal:** Security hardening and soundness fixes before Issue 007.

**What was built:**

**DoS Protection:**
| Limit | Value | Purpose |
|-------|-------|---------|
| Source size | 1 MB | Prevent memory exhaustion |
| Token count | 200,000 | Bound lexer work |
| Parser nesting | 512 | Prevent stack overflow |
| Inference depth | 512 | Bound type inference recursion |
| Eval call depth | 1,000 | Prevent runaway recursion |

**Soundness Fixes:**
- `Ty::Never` only unifies with itself (not a wildcard)
- Divergence handled correctly in if/else and block inference
- Removed panic!/expect() from type checker
- Token limit latching
- Scope guard for guaranteed pop_scope()

**Parser Improvements:**
- `::` qualified type paths
- Span-end fixes
- Universal lexer error surfacing
- Nesting guards for if/while/else-if

**Test stats:** 163 tests passing (30 new hardening tests)

---

### Issue 006: Blocks & Control Flow ✅
**Completed:** February 2, 2026

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

**What was fixed:**
1. Unknown identifiers now error properly
2. Real unification errors (no placeholders)
3. Correct generalization boundaries
4. Numeric constraints on arithmetic
5. Two-pass module checking
6. Minimal constraint provenance
7. Determinism audit - PASS

---

## Next Up

### Issue 007: ADTs & Pattern Matching
**Status:** Design review in progress
**Estimated time:** 10-14 days

**Goal:** Add structs, enums, generics, and pattern matching.

**Syntax:**
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

**Scope:**
- Struct definitions
- Enum definitions with variants
- Generic type parameters
- Match expressions
- Pattern matching (literals, vars, constructors)
- Exhaustiveness checking (Maranget algorithm)

**Design Decisions Under Review:**
1. Nested patterns — `Some(Some(x))`, struct patterns
2. Match guards — `Some(x) if x > 0 => ...`
3. Tuple types — `(Int, String)`
4. Variant syntax — Unit, Tuple, Struct variants
5. Struct update syntax — `Point { x: 1, ..other }`
6. Visibility modifiers — `pub struct`

**Critical Soundness Concerns (from Gemini):**
1. Constructor variance — constructors as polymorphic functions
2. Pattern scoping — mutability tracking in match arms
3. Exhaustiveness + Never — match type = join of non-Never arms
4. Redundant patterns — dead arms as security risk

**Current activity:** Multi-LLM design review (ChatGPT, Gemini, Grok)

---

## Development Lessons

### Key Learnings from Issues 005-006
- **Soundness review is essential:** Post-completion review caught 7 critical bugs
- **Cost asymmetry:** Hardening now prevents debugging nightmares later
- **Foundation first:** Don't build on broken abstractions
- **Trust is everything:** Type system must not lie
- **Security early:** DoS protection is foundational, not optional

### Development Philosophy
- **Soundness over speed** — A type system that lies is worse than no type system
- **Foundation integrity** — Fix bugs before they compound
- **Post-completion review** — Always audit before moving forward
- **Deterministic behavior** — Flaky tests waste more time than they save
- **Multi-LLM review** — Fresh eyes catch things we miss

---

## Project Statistics

**Codebase:**
- 4 crates (strata-ast, strata-parse, strata-types, strata-cli)
- ~5,500+ lines of Rust code
- 163 tests (all passing)
- 0 clippy warnings (enforced by CI)

**Velocity:**
- Issues 001-006 + hardening: ~2 months
- On track for v0.1 timeline (10-14 months total)
