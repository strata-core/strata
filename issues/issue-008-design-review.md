# Issue 008: Effect System — Design Review Document

> **Version:** Draft v0.1 — Pending codebase verification  
> **Author:** Chief Architect  
> **Date:** February 5, 2026  
> **Status:** Design review (pre-implementation)  
> **Prerequisite:** Issue 007 (ADTs & Pattern Matching) merged, 292 tests, v0.0.7.0+

---

## 1. Executive Summary

Issue 008 introduces Strata's **effect system** — the language's core differentiator. Every side effect (network, filesystem, time, randomness, failure, AI calls) must appear in function type signatures. The type checker enforces this: if a function performs effects not declared in its signature, it's a compile error.

This issue covers **effect annotations, inference, checking, and propagation**. It does NOT include capability tokens (Issue 009), effect handlers (Issue 010+), or profile enforcement (Issue 010+).

**Why this matters:** Without this, Strata is "just another ML-family language." With it, Strata delivers its promise: you can look at a function signature and know *exactly* what it can do to the outside world.

---

## 2. Feature Scope

### In Scope (Must Ship)

| Feature | Description |
|---------|-------------|
| Effect annotations | `fn f() -> T & {Net, Fs}` syntax on function signatures |
| Effect row in Ty | Function types carry effect information internally |
| Effect inference | Infer effects from function bodies automatically |
| Effect checking | Verify declared effects ⊇ actual effects |
| Effect propagation | Calling effectful function propagates effects to caller |
| Row variables | Effect polymorphism for higher-order functions |
| Built-in effects | Net, Fs, Time, Rand, Fail, Ai |
| Pure functions | No annotation = pure (empty effect row) |
| Error messages | Clear diagnostics for effect mismatches |
| DoS limits | Effect variable depth, row size limits |

### Out of Scope (Deferred)

| Feature | Deferred To | Reason |
|---------|-------------|--------|
| Capability tokens (`using cap: Type`) | Issue 009 | Separate concern, needs effect system first |
| Effect handlers | Issue 010+ | Algebraic effects are post-v0.1 stretch |
| Profile enforcement (`profile Kernel;`) | Issue 010+ | Needs effect system + capabilities first |
| Parameterized effects (`Fail<E>`) | v0.2+ | Adds type-level complexity, MVP doesn't need it |
| Sub-effects (`Fs.Read`, `Fs.Write`) | v0.2+ | Coarse-grained is sufficient for MVP |
| Effect aliases | v0.2+ | Convenience, not foundational |
| User-defined effects | v0.2+ | Needs handlers first |

---

## 3. Design Decisions

### D1: Effect Annotation Syntax — `& {Effect1, Effect2}`

**Decision:** Effects appear after the return type, separated by `&`.

```strata
// Effectful function
fn read_file(path: String) -> String & {Fs} { ... }

// Multiple effects
fn fetch_and_log(url: String) -> String & {Net, Fs, Time} { ... }

// Pure function — no annotation needed
fn add(x: Int, y: Int) -> Int { ... }

// Explicit pure (optional, for documentation)
fn add(x: Int, y: Int) -> Int & {} { ... }
```

**Rationale:**
- Already decided in Issue 002 and DESIGN_DECISIONS.md
- Visually separates "what" (return type) from "how" (side effects)
- Reads as refinement: "returns String, and also does Fs"
- Keeps Rust-adjacent readability

**Parsing note:** The `&` is unambiguous here because it follows a return type position. No conflict with binary `&` (bitwise AND) since that only appears in expression context.

**What about functions with no return type annotation?**

```strata
// Effect annotation without explicit return type:
// The return type is inferred, but effects are declared
fn log_it(msg: String) & {Fs} { ... }

// Equivalent to:
fn log_it(msg: String) -> Unit & {Fs} { ... }
```

**Decision:** Allow `& {Effects}` without explicit `-> ReturnType`. Parser interprets this as `-> Unit & {Effects}`.

---

### D2: Effect Representation in the Type System

**Decision:** Extend `Ty` with effect rows. Function types carry an `EffectRow`.

```
Current:  Ty::Fun(Vec<Ty>, Box<Ty>)           // (params) -> return
Proposed: Ty::Fun(Vec<Ty>, Box<Ty>, EffectRow) // (params) -> return & effects
```

**EffectRow representation:**

```rust
/// An effect row: a set of concrete effects plus an optional tail variable.
/// 
/// - Closed row: `{Net, Fs}` — tail is None, effects are exactly known
/// - Open row: `{Net} ∪ ε` — tail is Some(var), may have more effects
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectRow {
    /// Concrete effects known to be in this row
    pub effects: BTreeSet<Effect>,
    /// Row tail variable for polymorphism (None = closed/fully known)
    pub tail: Option<EffectVarId>,
}
```

**Why this representation:**
- **Closed rows** (`tail: None`) represent fully-known effect sets — used for annotated functions
- **Open rows** (`tail: Some(ε)`) represent partially-known effects — used during inference
- **Unification** of open rows is straightforward: concrete effects union, tail variables unify
- **Set semantics** (not list): `{Net, Fs}` == `{Fs, Net}`, no duplicates — already decided in Issue 002
- **BTreeSet** gives canonical ordering for deterministic output

**EffectVarId:** New ID type, separate from TypeVarId. Effects and types are different domains — keeping their variable IDs separate prevents confusion and bugs.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectVarId(pub u32);
```

---

### D3: Row Variables — Required for Higher-Order Functions

**Decision:** Implement row variables (effect polymorphism) in Issue 008. This is NOT deferrable.

**Why it's necessary:**

```strata
fn apply<A, B>(f: (A) -> B, x: A) -> B {
    f(x)  // What effects does apply have?
}
```

Without row variables, `apply` cannot express "whatever effects `f` has." The options are:
1. ❌ Make `apply` pure → forbids passing effectful functions (unusable)
2. ❌ Give `apply` all effects → loses all precision (defeats the purpose)
3. ✅ Row variable: `apply` has effects `ε` where `f: (A) -> B & ε`

**Typed signature with row variable:**

```strata
// Inferred (user doesn't write this):
// apply : ∀A B ε. ((A) -> B & ε, A) -> B & ε
fn apply<A, B>(f: (A) -> B, x: A) -> B {
    f(x)
}
```

**Implementation approach:** Row variables participate in constraint generation and unification just like type variables. The constraint solver already handles variables + unification; we extend it with effect variable constraints.

**Constraint types to add:**

```rust
enum Constraint {
    // Existing
    TypeEqual(Ty, Ty, Span),
    
    // New for effects
    EffectSubset(EffectRow, EffectRow, Span),  // row1 ⊆ row2
    EffectEqual(EffectRow, EffectRow, Span),    // row1 = row2
}
```

**Key insight:** In the constraint-based approach, adding effect constraints is natural. We're not threading substitutions — we're collecting constraints and solving them. Effect constraints are just more constraints.

---

### D4: Built-in Effects for v0.1

**Decision:** Six built-in effects, matching the existing `Effect` enum plus `Ai`.

| Effect | Meaning | Examples |
|--------|---------|----------|
| `Net` | Network I/O | HTTP requests, TCP connections, DNS |
| `Fs` | Filesystem I/O | Read/write files, directory ops |
| `Time` | Time observation/manipulation | Clock reads, sleep, timeouts |
| `Rand` | Non-determinism | Random number generation |
| `Fail` | Recoverable failure | Error raising (unparam. for now) |
| `Ai` | AI/LLM calls | Model inference, embeddings |

**`Ai` rationale:** Strata's target includes AI/ML ops. An explicit `Ai` effect lets teams audit which functions call external models — critical for cost control, latency budgeting, and compliance. This also supports the "killer demo" (multi-LLM orchestration).

**`Fail` note:** For v0.1, `Fail` is unparameterized. It means "this function can fail." Parameterized `Fail<E>` (where `E` is the error type) is deferred to v0.2+ — it interacts with ADTs in ways that need careful design, and unparameterized Fail is sufficient for the MVP effect tracking story.

**Pure:** A function with no effects (empty row) is "pure" in Strata's sense. Pure functions are safe to retry, cache, replay, and parallelize — this is foundational for deterministic replay.

---

### D5: Effect Inference Algorithm

**Decision:** Infer effects bottom-up from function bodies, using the constraint solver.

**How it works:**

1. **Fresh effect variable:** When inferring a function body, create a fresh effect variable `ε_body` to accumulate the body's effects.

2. **Effect propagation rules:**

| Expression | Effect contribution |
|------------|-------------------|
| Literal (`1`, `"hello"`, `true`) | ∅ (pure) |
| Variable reference | ∅ (pure) |
| Binary/unary op | ∅ (pure — operators are primitive) |
| Function call `f(args)` | effects(f) ∪ effects(args) |
| Let binding `let x = e;` | effects(e) |
| Block `{ s1; s2; expr }` | effects(s1) ∪ effects(s2) ∪ effects(expr) |
| If/else `if c { a } else { b }` | effects(c) ∪ effects(a) ∪ effects(b) |
| While `while c { body }` | effects(c) ∪ effects(body) |
| Match `match s { arms }` | effects(s) ∪ effects(arm₁) ∪ ... ∪ effects(armₙ) |
| Return `return e` | effects(e) |
| Struct construction `S { f: e }` | effects(e₁) ∪ ... ∪ effects(eₙ) |
| Enum construction `E::V(e)` | effects(e) |
| Field access `e.field` | effects(e) |

3. **Constraint generation for calls:** When we see `f(x)` and `f` has type `(A) -> B & ε_f`:
   - Generate type constraints as usual
   - Generate effect constraint: `ε_f ⊆ ε_body` (f's effects flow into body's effects)

4. **Checking against annotation:** If the function has a declared effect annotation `& {Net, Fs}`:
   - Generate constraint: `ε_body ⊆ {Net, Fs}` (body effects must be subset of declared)
   - If solving fails (body has effects not in annotation), report error

5. **Unannotated functions:** If no effect annotation:
   - The function's effect row is `ε_body` (inferred, open)
   - Effects propagate to callers via the row variable

**Example walkthrough:**

```strata
fn process(path: String) -> String & {Fs, Net} {
    let data = read_file(path);   // read_file : (String) -> String & {Fs}
    let resp = http_post(data);   // http_post : (String) -> String & {Net}
    resp
}
```

Inference:
- `ε_body` = fresh effect variable
- `read_file(path)`: constraint `{Fs} ⊆ ε_body`
- `http_post(data)`: constraint `{Net} ⊆ ε_body`
- Declared: `& {Fs, Net}`, so constraint `ε_body ⊆ {Fs, Net}`
- Solver: `ε_body = {Fs, Net}` — satisfies all constraints ✅

---

### D6: Effect Checking Semantics

**Decision:** Declared effects must be a **superset** of actual effects.

```strata
// OK: declares more effects than used (conservative)
fn foo() -> Int & {Net, Fs} {
    read_file("x")  // only uses Fs
}

// ERROR: body uses Fs but not declared
fn bar() -> Int & {Net} {
    read_file("x")  // Error: undeclared effect Fs
}

// OK: pure function, no effects
fn baz(x: Int) -> Int {
    x + 1
}
```

**Superset, not exact match.** This is deliberate:
- Allows forward-compatible signatures (declare effects you'll use later)
- Matches how Rust handles trait bounds (you can be more general)
- Prevents breakage when refactoring implementations
- The important property is **soundness**: no undeclared effects sneak through

**Warning for unused declared effects?** Not for v0.1. This is a lint concern, not a soundness concern. Can add later as a clippy-style warning.

---

### D7: Effect Propagation Through Higher-Order Functions

**Decision:** Effects flow through function arguments via row variables.

```strata
fn map<A, B>(f: (A) -> B, xs: List<A>) -> List<B> {
    // f is called inside map, so map inherits f's effects
    // Inferred: map : ∀A B ε. ((A) -> B & ε, List<A>) -> List<B> & ε
    ...
}

// Usage:
let files = map(read_file, paths);
// read_file : (String) -> String & {Fs}
// So: map(read_file, paths) : List<String> & {Fs}
// Effect Fs propagates correctly!
```

**This is why row variables are essential.** Without them, `map` would have to be either pure (can't take effectful functions) or maximally effectful (useless).

**Closures:** A closure's effect row includes the effects of any effectful calls it makes.

```strata
let f = |x| read_file(x);  // f : (String) -> String & {Fs}
let g = |x| x + 1;          // g : (Int) -> Int       (pure)
```

---

### D8: Function Types in Surface Syntax

**Decision:** Function types in annotations include effect rows.

```strata
// Function type without effects (pure)
let f: (Int) -> Int = |x| x + 1;

// Function type with effects
let g: (String) -> String & {Fs} = read_file;

// Higher-order function type
let h: ((Int) -> Int, Int) -> Int = apply;
```

**Row variables are NOT exposed in surface syntax for v0.1.** Users cannot write `& ε` or `& {Net} ∪ ε`. Row variables are internal to the inference engine. Users either:
- Annotate with concrete effects: `& {Net, Fs}`
- Omit effects (inferred with row variable)

**Rationale:** Exposing row variables to users adds complexity with minimal benefit for v0.1. The inference engine handles polymorphism automatically. If users need to constrain effects precisely, they annotate with concrete sets. This can be revisited post-v0.1 if demand exists.

---

### D9: Error Messages

**Decision:** Effect errors must be specific, actionable, and show what was expected vs. found.

**Error categories:**

1. **Undeclared effect:**
```
error[E080]: undeclared effect in function body
  --> src/main.stra:5:12
   |
 5 |     read_file("config.txt")
   |     ^^^^^^^^^ this call has effect `Fs`
   |
   = note: function `process` declares effects: {Net}
   = note: but body requires effects: {Net, Fs}
   = help: add `Fs` to the effect annotation: `& {Net, Fs}`
```

2. **Effect mismatch in assignment:**
```
error[E081]: effect mismatch in function type
  --> src/main.stra:3:18
   |
 3 |     let f: (String) -> String = read_file;
   |            ^^^^^^^^^^^^^^^^^^   ^^^^^^^^^ has effects {Fs}
   |            |
   |            expected pure function (no effects)
   |
   = help: add effects to the type: `(String) -> String & {Fs}`
```

3. **Propagation from higher-order:**
```
error[E082]: effectful function passed where pure function expected
  --> src/main.stra:7:14
   |
 7 |     pure_map(read_file, paths)
   |              ^^^^^^^^^ has effects {Fs}
   |
   = note: `pure_map` expects a pure function argument
   = note: parameter type: `(String) -> String` (no effects)
```

---

### D10: DoS Protection

**Decision:** Extend existing limits to cover effect system.

| Limit | Value | Rationale |
|-------|-------|-----------|
| Effects per row | 32 | Prevents effect explosion from composition |
| Effect variables per function | 16 | Matches generic param limit |
| Effect constraint count | 4096 | Prevents solver runaway |

These are generous limits. No legitimate program should hit them. They exist to prevent adversarial inputs from causing unbounded computation in the solver.

---

## 4. Implementation Phases

### Phase 1: AST & Parser (3-4 days)

**Goal:** Parse effect annotations in function signatures.

1. Add `EffectAnnotation` to AST
   - `FnDef` gets optional `effects: Option<Vec<Effect>>`
   - Function type expressions get optional effect annotation
2. Extend parser
   - After `-> ReturnType`, check for `& { ... }`
   - Parse comma-separated effect names inside braces
   - Support `& {}` for explicit pure
3. Add effect keywords to lexer
   - `Net`, `Fs`, `Time`, `Rand`, `Fail`, `Ai` as effect identifiers
   - Or: treat them as regular identifiers and resolve in type checking
4. Tests
   - Parse effectful function signatures
   - Parse pure functions (no annotation)
   - Parse explicit pure (`& {}`)
   - Parse function types with effects in let annotations
   - Error cases: unknown effects, malformed syntax

**Decision point — effects as keywords or identifiers?**

Recommendation: **Identifiers resolved during type checking**, not keywords. This means:
- `Net`, `Fs`, etc. are normal identifiers in the parser
- The type checker recognizes them as built-in effects
- Users can't shadow them (type checker rejects this)
- Leaves room for user-defined effects later

This keeps the parser simpler and avoids keyword explosion.

### Phase 2: Type System Extension (4-5 days)

**Goal:** Extend internal types with effect rows and add effect variable support.

1. Add `EffectRow` type (as designed in D2)
   - `effects: BTreeSet<Effect>`, `tail: Option<EffectVarId>`
   - Constructors: `EffectRow::pure()`, `EffectRow::concrete(...)`, `EffectRow::open(...)`
2. Extend `Ty::Fun` to carry `EffectRow`
   - Update all code that constructs/destructs function types
   - **This will touch a LOT of existing code** — plan for it
3. Add `EffectVarId` and fresh variable generation
   - Separate counter from type variables
   - Add to the checker's state
4. Add effect-related constraints
   - `Constraint::EffectSubset(EffectRow, EffectRow, Span)`
   - `Constraint::EffectEqual(EffectRow, EffectRow, Span)`
5. Extend substitution to handle effect variables
   - `Subst` maps both `TypeVarId -> Ty` and `EffectVarId -> EffectRow`
6. Tests
   - EffectRow construction and operations
   - Effect variable substitution
   - Function type construction with effects

**Risk:** Extending `Ty::Fun` is a wide-reaching change. Every place that matches on `Ty::Fun(params, ret)` needs updating to `Ty::Fun(params, ret, effects)`. Do this carefully with the compiler guiding you — add the field, then fix every compile error.

### Phase 3: Effect Inference (5-6 days)

**Goal:** Infer effects from function bodies and generate effect constraints.

1. Extend `infer_expr` / `infer_stmt` for effect tracking
   - Each expression inference now also tracks accumulated effects
   - Function calls: look up callee's effect row, propagate
   - Control flow: union effects from branches
2. Effect constraint generation
   - At function boundaries: body effects ⊆ declared effects (if annotated)
   - At call sites: callee effects ⊆ caller's accumulator
   - For closures: capture the inferred effect row
3. Extend solver for effect constraints
   - Effect variable unification
   - Effect row subset checking
   - Occurs check for effect variables (prevent infinite effect rows)
4. Bridge from surface annotation to internal EffectRow
   - Convert parsed `Vec<Effect>` to `EffectRow` (closed, no tail)
5. Tests
   - Pure function inference
   - Single-effect function inference
   - Multi-effect function inference
   - Effect propagation through calls
   - Effect propagation through control flow (if, while, match)
   - Higher-order effect propagation (row variables)
   - Closure effect inference
   - Nested function calls

### Phase 4: Effect Checking & Errors (3-4 days)

**Goal:** Enforce effect declarations and produce clear error messages.

1. Implement effect checking
   - Annotated function: body effects ⊆ declared effects
   - Unannotated function: effects inferred (no check needed)
   - Function type annotations in `let`: effect compatibility check
2. Implement effect error types
   - `TypeError::UndeclaredEffect { effect, function, declared, span }`
   - `TypeError::EffectMismatch { expected, found, span }`
   - `TypeError::EffectInPureContext { effect, span }`
3. Error message formatting
   - Show expected vs found effect sets
   - Point to the offending call site
   - Suggest fix (add effect to annotation)
4. Tests
   - Undeclared effect → error
   - Extra declared effect → OK (superset allowed)
   - Pure context violation → error
   - Higher-order effect mismatch → error
   - All error messages readable and helpful

### Phase 5: Hardening & Integration (3-4 days)

**Goal:** Harden, test edge cases, integrate with existing features.

1. Edge case testing
   - Recursive functions with effects
   - Mutually recursive effectful functions
   - Effects through pattern matching arms
   - Effects with generic ADTs
   - Empty effect row interactions
   - Never type + effects
2. DoS limit enforcement
   - Effect row size limits
   - Effect constraint count limits
3. Evaluator updates
   - Evaluator doesn't need to *enforce* effects at runtime (that's the type checker's job)
   - But: should we tag runtime values with their effect context? (Recommendation: no, for v0.1)
4. Documentation
   - Update IMPLEMENTED.md
   - Write ADR-0005 for effect system decisions
   - Update code comments
   - Add effect examples to examples/
5. Bug hunt
   - Systematic testing of interactions between effects and all existing features
   - Focus on: generics + effects, closures + effects, ADTs + effects, Never + effects

**Total estimated time: 18-23 days**

---

## 5. Interaction with Existing Features

### Effects + Generics

```strata
fn unwrap_or<T>(opt: Option<T>, default: T) -> T {
    match opt {
        Option::Some(x) => x,
        Option::None => default,
    }
}
// Inferred: pure (no effects) — correct!

fn unwrap_or_else<T>(opt: Option<T>, f: () -> T) -> T {
    match opt {
        Option::Some(x) => x,
        Option::None => f(),
    }
}
// Inferred: ∀T ε. (Option<T>, () -> T & ε) -> T & ε — row variable from f!
```

### Effects + Pattern Matching

Effects propagate through match arms via union:
```strata
fn handle(opt: Option<String>) -> String & {Fs, Net} {
    match opt {
        Option::Some(path) => read_file(path),    // {Fs}
        Option::None => http_get("default.txt"),   // {Net}
    }
}
// Body effects: {Fs} ∪ {Net} = {Fs, Net} — matches annotation ✅
```

### Effects + Closures

Closures inherit effects from their body:
```strata
let reader = |path| read_file(path);
// reader : (String) -> String & {Fs}

let pure_fn = |x| x + 1;
// pure_fn : (Int) -> Int  (pure)
```

### Effects + Never

A diverging expression (Never type) contributes no effects:
```strata
fn diverge() -> Never & {} {
    while true { }  // while is pure
}

fn maybe_diverge(x: Bool) -> Int & {Fs} {
    if x {
        read_file("f")  // {Fs}
    } else {
        diverge()  // Never — no effect contribution to result
    }
}
// But diverge() is called, so its effects still propagate!
// Actually: diverge() declares & {} so no effects propagate. Correct.
```

**Subtlety:** The *call* to a function propagates its declared effects, regardless of whether the return type is Never. A diverging function with effects `& {Net}` still propagates Net to the caller.

### Effects + Mutable Bindings

Mutable bindings themselves have no effect — they're local state. Assignment is pure.

```strata
fn counter() -> Int {
    let mut x = 0;
    while x < 10 {
        x = x + 1;  // Pure — no effect
    }
    x
}
// Inferred: pure ✅
```

---

## 6. Open Questions for Multi-LLM Review

### Q1: Effect Variable Syntax in Annotations (Future)

For v0.1, row variables are internal only. But eventually, will users write:

```strata
// Option A: Explicit effect variable
fn map<A, B, effect E>(f: (A) -> B & E, xs: List<A>) -> List<B> & E

// Option B: Implicit (inferred, never written)
fn map<A, B>(f: (A) -> B, xs: List<A>) -> List<B>

// Option C: Wildcard syntax
fn map<A, B>(f: (A) -> B & .., xs: List<A>) -> List<B> & ..
```

**Current decision:** Option B for v0.1 (inferred). Revisit for v0.2.

**Question for reviewers:** Does deferring this create technical debt that's hard to unwind?

### Q2: Effect Inference vs. Mandatory Annotation

Should **all** functions at module boundaries require effect annotations?

```strata
// Option A: Mandatory at pub boundaries (like types)
pub fn read(path: String) -> String & {Fs} { ... }  // Required

// Option B: Always inferred (like Go error handling — implicit)
pub fn read(path: String) -> String { ... }  // Fs inferred

// Option C: Mandatory everywhere
fn helper(x: Int) -> Int & {} { ... }  // Must write & {} for pure
```

**Current decision:** Option A (mandatory at `pub` boundaries, inferred locally). Matches the existing design principle: "explicit at boundaries, inferred locally."

**But we don't have `pub` yet.** For v0.1 008:
- Top-level functions: annotation optional, effects inferred if omitted
- The "mandatory at boundaries" rule activates when we add modules/visibility

**Question for reviewers:** Is this the right default? Or should we require annotations on all top-level functions from day one?

### Q3: How Do We Bootstrap Effectful Builtins?

Functions like `read_file`, `http_get`, etc. don't exist yet. For testing effect inference, we need some way to introduce effectful operations.

**Options:**
- **A: Builtin declarations** — hardcode a few effectful functions the type checker knows about
- **B: `extern` declarations** — `extern fn read_file(path: String) -> String & {Fs};`
- **C: Effect annotation is the source of truth** — if you annotate a function with effects, the type checker trusts it, even if the body is empty/stub

**Recommendation:** Option C for v0.1 testing, Option B for v0.1 release. The annotation is authoritative — the checker verifies bodies don't exceed declared effects, but declared effects are trusted as the function's contract. This means we can write test stubs.

### Q4: Interaction with `Ty::Never` Edge Cases

```strata
// What are the effects of this?
fn impossible() -> Never & {Net} {
    http_get("x")  // Never returns, but does Net
}

// And this?
fn also_impossible() -> Never {
    while true { }  // No effects — pure divergence
}
```

**Current thinking:** Effects of a Never-returning function are still tracked. `impossible()` has effect `{Net}` even though it never returns. The effect annotation describes what the function *does*, not what it *produces*.

**Question for reviewers:** Any edge cases we're missing with Never + effects?

### Q5: Recursive Effects

```strata
fn retry(f: () -> String, n: Int) -> String & {???} {
    if n <= 0 {
        f()
    } else {
        retry(f, n - 1)
    }
}
```

The effects of `retry` depend on the effects of `f` (row variable) plus its own recursive call. The recursive call's effects are the same as `retry`'s effects — this is fine, it's just a fixed point.

**Approach:** Same as recursive type inference. The function's effect row starts as a fresh variable, infers from the body, and the recursive reference uses the same variable. The solver finds the fixed point.

**Question for reviewers:** Are there pathological cases in recursive effect inference that could diverge or produce incorrect results?

---

## 7. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Ty::Fun refactor is wide-reaching | High | Medium | Compiler-guided: add field, fix all errors |
| Row variable unification bugs | Medium | High | Extensive test suite, multi-LLM review |
| Effect inference interacts badly with existing features | Medium | High | Phase 5 integration testing |
| Performance regression from effect tracking | Low | Low | Effects are compile-time only |
| Scope creep into capability/handler territory | Medium | Medium | Hard scope boundary at the top of this doc |

---

## 8. Success Criteria

After Issue 008, these must all be true:

1. **Functions can declare effects:** `fn f() -> T & {Net, Fs}` parses and type-checks
2. **Effects are inferred:** Unannotated functions get correct effect rows
3. **Effect checking works:** Undeclared effects produce clear compile errors
4. **Propagation works:** Calling an effectful function propagates effects to the caller
5. **Row variables work:** Higher-order functions correctly propagate effects from function arguments
6. **All existing tests still pass** (effects default to pure/inferred for existing code)
7. **292+ existing tests pass** + comprehensive new effect tests
8. **Clean CI:** `cargo test --workspace` + `cargo clippy --workspace` clean
9. **Error messages are clear and actionable**
10. **No effect can sneak through without appearing in the type**

---

## 9. Appendix: Effect Row Unification Algorithm

For the solver, effect row unification works as follows:

**Case 1: Two closed rows**
```
unify({A, B}, {A, B}) = OK
unify({A, B}, {A, C}) = Error (B ≠ C)
```

**Case 2: Closed row with open row**
```
unify({A, B}, {A} ∪ ε) = [ε → {B}]
unify({A}, {A, B} ∪ ε) = [ε → {}]  // ε becomes empty
```

**Case 3: Two open rows**
```
unify({A} ∪ ε1, {B} ∪ ε2) =
    let ε3 = fresh()
    [ε1 → {B} ∪ ε3, ε2 → {A} ∪ ε3]
// Both rows share unknown remainder ε3
```

**Case 4: Subset checking (for declared ⊇ body)**
```
subset({A, B} ∪ ε, {A, B, C}) = [ε → {} or subset of {C}]
subset({A, B, D} ∪ ε, {A, B, C}) = Error (D not in declared)
```

**Occurs check:** If `ε` appears in the row being substituted for `ε`, that's an infinite effect row — error.

---

*End of design review document. Ready for multi-LLM review.*
