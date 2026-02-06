# Issue 010: Affine Types (Linear Capabilities)

**Status:** Ready (Issue 009 complete, milestone review done)
**Estimated time:** 7-10 days
**Phase:** 3 (Effect System — final issue)
**Depends on:** Issue 009 (Capability Types)
**Blocks:** Issue 011 (WASM Runtime)
**Branch:** `issue-010`

## Goal

Prevent capability duplication and leaking by making capabilities *affine* — they can be used at most once. This ensures capabilities cannot be copied, stored globally, or otherwise escaped from their intended scope.

After Issue 010, Strata's security model is complete: capabilities cannot be forged (Issue 009), cloned (Issue 010), or leaked (Issue 010). Combined with effect typing (Issue 008), this gives mathematical guarantees about what any Strata program can do.

## The Problem We're Solving

After Issue 009, capabilities gate effects. But without linearity:
```strata
fn leak(fs: FsCap) -> () & {} {
    let copy1 = fs;
    let copy2 = fs;  // Duplicated!
    store_in_global(copy1);  // Leaked!
    ()  // Returns pure, but fs is now ambient
}
```

This defeats capability security. With affine types:
```strata
fn leak(fs: FsCap) -> () & {} {
    let copy1 = fs;      // fs transferred to copy1
    let copy2 = fs;      // ERROR: capability 'fs' has already been used
}
```

## Core Concept: Kinds

Types have *kinds* that describe their usage constraints:

| Kind | Name | Meaning |
|------|------|---------|
| `*` | Unrestricted | Can be used any number of times (Int, String, Bool, etc.) |
| `!` | Affine | Can be used *at most once* (capabilities) |

All capability types (`FsCap`, `NetCap`, etc.) have kind `!`. All other types have kind `*`.

## Syntax

No new surface syntax required. Affinity is intrinsic to capability types — users don't annotate it, the compiler knows.

```strata
// FsCap is inherently single-use — no annotation needed
fn use_twice(fs: FsCap) -> () & {Fs} {
    do_thing_1(fs);  // fs used here
    do_thing_2(fs);  // ERROR: capability 'fs' has already been used
}
```

## Architecture: Separate Post-Inference Pass

**Critical decision:** Move checking is a **separate validation pass** that runs after type inference and constraint solving are complete. It operates on a fully-typed, fully-solved AST.

**Why not embed in the checker/inference?**
- During inference, types may still be variables — you can't determine affinity until types are resolved
- Coupling move analysis with constraint solving creates order-dependent bugs that are nearly impossible to debug
- Separate pass is independently testable with clear inputs (typed AST) and outputs (move errors)
- Preserves the "Python-like" feel of inference — users get standard type errors first, then security errors

**Integration point:** After `check_module()` succeeds, run `move_check_module()` on the result.

```
Source → Parse → Type Check (HM inference, effects, caps) → Move Check → Done
                                                              ↑
                                                    Queries Ty::kind() on
                                                    resolved types to decide
                                                    what needs tracking
```

**Important:** The type inference engine (unifier, solver, constraint generator) is **unchanged**. Kinds do not participate in unification. A type variable `T` unifies with `FsCap` normally — the move checker later sees the resolved type is `FsCap`, determines it's affine, and tracks its usage.

## Semantics

### Vocabulary

Strata uses **permission/authority language**, not Rust's memory-management language:

| Concept | Strata term | NOT this (Rust term) |
|---------|-------------|---------------------|
| Using a capability | "used" / "transferred" | "moved" |
| After use | "no longer available" | "moved out" |
| Type property | "single-use capability" | "affine type" |
| Error | "has already been used" | "use of moved value" |

This vocabulary matters — Strata's target audience is ops/SRE engineers, not Rust developers. The language should feel like it's talking about **permissions**, not **memory**.

### Transfer Semantics

Using a single-use capability *transfers* it. After transfer, the binding is no longer valid.

```strata
fn example(fs: FsCap) -> () & {Fs} {
    let a = fs;       // fs transferred to a (fs no longer available)
    let b = fs;       // ERROR: capability 'fs' has already been used

    use_cap(a);       // a transferred into function call
    use_cap(a);       // ERROR: capability 'a' has already been used
}
```

### Single-Use Values Can Be Dropped

Unlike linear types (use-exactly-once), single-use values can go out of scope unused:

```strata
fn drop_ok(fs: FsCap) -> () & {} {
    // fs is never used — that's fine (single-use means "at most once")
    ()
}
```

### Return Transfers Ownership

Returning a single-use value transfers ownership to the caller:

```strata
fn passthrough(fs: FsCap) -> FsCap & {} {
    fs  // ownership transferred to caller
}

fn caller(fs: FsCap) -> () & {Fs} {
    let fs2 = passthrough(fs);  // fs transferred in, fs2 received back
    use_cap(fs2);  // valid
}
```

### Branching (If/Else)

The **pessimistic join rule**: if a capability is used in *any* branch, it is considered used for the remainder of the function, regardless of which branch executes at runtime.

```strata
// PASS: Both branches use fs — consistent
fn branch_ok(fs: FsCap, cond: Bool) -> () & {Fs} {
    if cond {
        use_cap(fs);
    } else {
        use_cap(fs);
    }
    // fs is used regardless of which branch — consistent
}

// FAIL: Only one branch uses fs — inconsistent
fn branch_bad(fs: FsCap, cond: Bool) -> () & {Fs} {
    if cond {
        use_cap(fs);  // fs used here
    }
    // ERROR: capability 'fs' may or may not be available here
    //        (used in one branch but not the other)
    use_cap(fs);
}

// PASS: Neither branch uses fs — consistent
fn branch_neither(fs: FsCap, cond: Bool) -> () & {Fs} {
    if cond {
        ()
    } else {
        ()
    }
    use_cap(fs);  // valid — fs was not used in either branch
}
```

### Pattern Matching

Pattern matching follows the same pessimistic join rule as if/else, with additional rules for destructuring.

**Rule 1: Matching on a single-use value consumes it.**
If `opt` has type `Option<FsCap>`, then `match opt { ... }` uses `opt`. It is no longer available after the match block.

**Rule 2: Pattern bindings inherit kind from their resolved type.**
In `Option::Some(fs) => ...`, the binding `fs` has type `FsCap`, so it's single-use within that arm.

**Rule 3: Pessimistic join across arms.**
If an external capability is used in any arm, it's considered used after the match. All arms must agree on what's used.

```strata
// PASS: Extracted capability used once per arm
fn match_use(opt: Option<FsCap>) -> () & {Fs} {
    match opt {
        Option::Some(fs) => use_cap(fs),
        Option::None => ()
    }
}

// FAIL: Extracted capability used twice in one arm
fn match_double(opt: Option<FsCap>) -> () & {Fs} {
    match opt {
        Option::Some(fs) => {
            use_cap(fs);
            use_cap(fs);  // ERROR: capability 'fs' has already been used
        },
        Option::None => ()
    }
}

// FAIL: External cap used inconsistently across arms
fn match_inconsistent(opt: Option<Int>, fs: FsCap) -> () & {Fs} {
    match opt {
        Option::Some(x) => use_cap(fs),
        Option::None => ()  // fs NOT used here
    }
    use_cap(fs);  // ERROR: capability 'fs' may or may not be available
}
```

**Note:** Since ADT fields containing capabilities remain banned in v0.1 (see Changes to Issue 009 below), the pattern matching move checker only needs to handle:
- Capabilities as match scrutinees (rare but valid)
- Capabilities in scope being used within match arms (common)
- Tuple destructuring where elements may be capabilities

Full ADT destructuring (e.g., `Struct { cap: fs, ... }`) is a v0.2 concern when the ADT cap ban is lifted.

### Loops

Single-use capabilities cannot be used inside loops — the body executes multiple times, violating "at most once":

```strata
fn loop_bad(fs: FsCap) -> () & {Fs} {
    while true {
        use_cap(fs);  // ERROR: capability 'fs' would be used multiple times in loop
    }
}
```

### Closures (v0.1: Banned)

For v0.1, single-use capabilities **cannot be captured by closures**:

```strata
fn closure_bad(fs: FsCap) -> () & {} {
    let f = fn() { use_cap(fs) };  // ERROR: cannot capture single-use capability in closure
    f();
}
```

This is the simplest safe design. v0.2 may allow single-use closures (closures that capture single-use values become single-use themselves).

### Shadowing

Rebinding a name via `let` is a transfer, not a copy. The old binding is consumed:

```strata
// PASS: Shadowing is a transfer
fn shadow_ok(fs: FsCap) -> () & {Fs} {
    let fs = fs;   // old fs transferred to new fs
    use_cap(fs);   // valid — refers to the new binding
}

// FAIL: Old binding consumed by shadow
fn shadow_fail(fs: FsCap) -> () & {Fs} {
    let fs2 = fs;  // fs transferred to fs2
    let fs3 = fs;  // ERROR: capability 'fs' has already been used
}
```

## Changes to Issue 009 Restrictions

Issue 009 introduced two restrictions that Issue 010 modifies:

### 1. `CapabilityInBinding` — Now Allowed as Transfer

**Issue 009 behavior:** `let x = fs;` produced `CapabilityInBinding` error.
**Issue 010 behavior:** `let x = fs;` is allowed — it's a transfer of ownership. The old binding (`fs`) is consumed.

**Rationale:** The 009 ban existed because without move tracking, `let` would allow duplication. Now that the move checker enforces single-use, `let` binding is safe and natural. Keeping the ban would confuse users ("why can I pass it to a function but not bind it to a variable?").

**Implementation:** Remove the `CapabilityInBinding` error variant (or gate it behind "pre-010" mode). The move checker replaces its safety guarantee.

### 2. ADT Fields Containing Capabilities — Still Banned

**Issue 009 behavior:** `struct Foo { cap: FsCap }` produced error.
**Issue 010 behavior:** Same — still banned.

**Rationale:** Allowing capabilities in ADT fields requires *kind inference for ADTs* — if any field is affine, the ADT itself must be affine. This is significant complexity (structural kind analysis, propagation through generics, interaction with pattern matching) that would expand 010's scope dramatically. The ban forces capabilities to exist only as local variables and function parameters, making authority flow trivially traceable.

**v0.2:** Introduce affine ADTs where `Kind(ADT) = supremum(Kind(fields))`. This enables `Option<FsCap>`, capability bundles, etc.

## Type System Changes

### Kind Tracking

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Unrestricted,  // * — can be used any number of times
    Affine,        // ! — can be used at most once
}

impl Ty {
    pub fn kind(&self) -> Kind {
        match self {
            Ty::Cap(_) => Kind::Affine,
            _ => Kind::Unrestricted,
        }
    }
}
```

**Important:** `Kind` does NOT participate in unification. The unifier is unchanged. `Kind` is only queried by the post-inference move checker.

### Move Checker Pass (New Module)

The move checker is a new module (e.g., `move_check.rs`) that validates single-use semantics on a fully-typed AST.

**Input:** Typed AST with all types resolved (after `check_module()` + constraint solving)
**Output:** List of move errors, or success

**Scoping rules:**
- Each function body starts with a fresh tracking state. Parameters are "alive" at function entry.
- `let x = expr` introduces a new tracked binding if `expr`'s resolved type is affine.
- Block scopes: a use inside a nested block propagates outward (the enclosing scope sees the binding as consumed).
- Shadowing: `let fs = fs;` consumes the old `fs` and introduces a new `fs` that is alive.
- Function calls: passing an affine value to a function consumes it.
- Return: returning an affine value consumes it (transfers to caller).

**Control flow rules:**
- **If/else:** Pessimistic join. If consumed in any branch, considered consumed after the if/else.
- **Match arms:** Same pessimistic join. Each arm is a separate path.
- **While loops:** Any affine binding used in a loop body is an error (would be consumed multiple times).
- **Closures:** Any affine binding captured by a closure is an error (v0.1).

**What is NOT specified:** The internal data structure. The implementation should use whatever representation makes the scoping and join rules clean (likely binding IDs with scope stacks, not string names).

## Error Messages

All error messages use permission/authority vocabulary, not Rust memory-management terms.

### Capability Already Used
```
Error: capability 'fs' has already been used

  --> deploy.strata:5:12
   |
 4 |     upload_logs(fs, data);
   |                 -- permission was transferred here
 5 |     close_file(fs);
   |                ^^ 'fs' is no longer available
   |
   = note: FsCap is a single-use capability — it can only be used once for security
   = help: if you need to use it again, have 'upload_logs' return it back to you
```

### Cannot Capture in Closure
```
Error: cannot capture single-use capability in closure

  --> deploy.strata:3:14
   |
 3 |     let f = fn() { use_cap(fs) };
   |                    ^^^^^^^^^^^ closure captures 'fs'
   |
   = note: FsCap is a single-use capability and cannot be captured by closures
   = help: pass the capability as a function parameter instead
```

### Inconsistent Branches
```
Error: capability 'fs' may or may not be available after branch

  --> deploy.strata:7:12
   |
 4 |     if cond {
 5 |         use_cap(fs);
   |                 -- used in this branch
 6 |     }
 7 |     use_cap(fs);
   |             ^^ 'fs' might not be available here
   |
   = help: ensure all branches either use or don't use 'fs'
```

### Cannot Use in Loop
```
Error: cannot use single-use capability inside loop

  --> deploy.strata:3:16
   |
 3 |     while cond { use_cap(fs); }
   |                          ^^ 'fs' would be used on every iteration
   |
   = note: FsCap can only be used once, but loops execute multiple times
   = help: restructure to use the capability before or after the loop
```

## Implementation Plan

### Phase 1: Kind System (no behavioral changes)

- Add `Kind` enum to type system
- Add `Ty::kind()` method
- Add conditional serde derives to `Kind` (lesson from Issues 008/009: any type inside `Ty` needs this)
- All existing tests still pass — no behavioral changes

### Phase 2: Move Checker Pass

- New module: `move_check.rs` (or `crates/strata-types/src/move_check.rs`)
- New error types for move violations (use vocabulary above)
- Integration: call after `check_module()` succeeds
- Implement in order:
  1. Simple let-binding transfers
  2. Function call consumption
  3. Return transfers
  4. Use-after-transfer errors
  5. If/else branch consistency (pessimistic join)
  6. While loop rejection
  7. Closure capture rejection
  8. Match arm consistency
  9. Shadowing
- Tests at each step

### Phase 3: Relax Issue 009 Restrictions

- Remove `CapabilityInBinding` error — replace with move tracking
- Update all tests that expected `CapabilityInBinding` rejection to expect move-tracking behavior
- Keep ADT cap field ban (unchanged from 009)
- Update documentation (DESIGN_DECISIONS.md, IMPLEMENTED.md)

## Definition of Done

- [ ] Kind enum: `Kind::Unrestricted` and `Kind::Affine`
- [ ] `Ty::kind()` method returning appropriate kind
- [ ] Conditional serde derives on `Kind`
- [ ] Move checker: new module, post-inference pass
- [ ] Move checker: error on use-after-transfer for single-use values
- [ ] Move checker: error on capturing single-use values in closures
- [ ] Move checker: pessimistic join for if/else branches
- [ ] Move checker: pessimistic join for match arms
- [ ] Move checker: error on single-use values in loops
- [ ] Move checker: shadowing handled correctly
- [ ] Move checker: nested block propagation
- [ ] Move checker: polymorphic functions with single-use values
- [ ] CapabilityInBinding replaced with move semantics
- [ ] ADT cap field ban unchanged (documented as v0.2)
- [ ] Error messages use permission/authority vocabulary
- [ ] Tests: basic transfer semantics
- [ ] Tests: use-after-transfer errors
- [ ] Tests: branch consistency (if/else)
- [ ] Tests: match arm consistency
- [ ] Tests: loop errors
- [ ] Tests: closure capture errors
- [ ] Tests: return transfers ownership
- [ ] Tests: drop is allowed (single-use, not must-use)
- [ ] Tests: shadowing transfers
- [ ] Tests: polymorphic + single-use interaction
- [ ] Tests: unrestricted types unaffected
- [ ] Documentation: DESIGN_DECISIONS.md updated
- [ ] Documentation: IMPLEMENTED.md updated
- [ ] ADR if any design forks emerge during implementation

## Test Cases

```strata
// === PASS: Basic transfer ===

fn single_use(fs: FsCap) -> () & {Fs} {
    use_cap(fs);
}

fn unused(fs: FsCap) -> () & {} {
    ()  // drop is ok — single-use, not must-use
}

fn passthrough(fs: FsCap) -> FsCap & {} {
    fs  // ownership transferred to caller
}

fn let_transfer(fs: FsCap) -> () & {Fs} {
    let a = fs;    // fs transferred to a
    use_cap(a);    // valid
}

fn shadow_transfer(fs: FsCap) -> () & {Fs} {
    let fs = fs;   // old fs transferred, new fs alive
    use_cap(fs);
}

fn branch_both_use(fs: FsCap, c: Bool) -> () & {Fs} {
    if c { use_cap(fs) } else { use_cap(fs) }
}

fn branch_neither_use(fs: FsCap, c: Bool) -> () & {Fs} {
    if c { () } else { () };
    use_cap(fs)  // valid — neither branch used fs
}

fn branch_return(fs: FsCap, c: Bool) -> FsCap & {} {
    if c { fs } else { fs }  // both branches return fs — consistent
}

fn unrestricted_normal(x: Int) -> Int & {} {
    let a = x;
    let b = x;  // fine — Int is unrestricted
    a + b
}

fn poly_affine(fs: FsCap) -> () & {Fs} {
    let fs2 = identity(fs);  // fs transferred into identity, fs2 received
    use_cap(fs2);
}

fn match_extract(opt: Option<Int>, fs: FsCap) -> () & {Fs} {
    match opt {
        Option::Some(x) => use_cap(fs),
        Option::None => use_cap(fs),   // consistent — both arms use fs
    }
}

// === FAIL: Use after transfer ===

fn double_use(fs: FsCap) -> () & {Fs} {
    use_cap(fs);
    use_cap(fs);  // ERROR: capability 'fs' has already been used
}

fn rebind_then_use(fs: FsCap) -> () & {Fs} {
    let a = fs;
    use_cap(fs);  // ERROR: capability 'fs' has already been used
}

fn poly_double(fs: FsCap) -> () & {Fs} {
    let fs2 = identity(fs);  // fs transferred
    use_cap(fs);              // ERROR: already used
}

// === FAIL: Inconsistent branches ===

fn branch_one_side(fs: FsCap, c: Bool) -> () & {Fs} {
    if c { use_cap(fs) };
    use_cap(fs);  // ERROR: may or may not be available
}

fn match_inconsistent(opt: Option<Int>, fs: FsCap) -> () & {Fs} {
    match opt {
        Option::Some(x) => use_cap(fs),
        Option::None => ()  // fs NOT used
    };
    use_cap(fs);  // ERROR: may or may not be available
}

// === FAIL: Loop ===

fn loop_use(fs: FsCap) -> () & {Fs} {
    while true { use_cap(fs) }  // ERROR: would be used multiple times
}

// === FAIL: Closure capture ===

fn closure_capture(fs: FsCap) -> () & {} {
    let f = fn() { use_cap(fs) };  // ERROR: cannot capture single-use capability
}

// === PASS: Unrestricted in all contexts ===

fn unrestricted_branch(x: Int, c: Bool) -> Int & {} {
    if c { x } else { x }  // fine — Int is unrestricted
}

fn unrestricted_loop(x: Int) -> Int & {} {
    let mut sum = 0;
    while sum < 10 { sum = sum + x };  // fine
    sum
}
```

## v0.2 Considerations (Documented Now, Deferred)

1. **Affine ADTs:** If an ADT contains a single-use field, the ADT itself becomes single-use. Requires kind inference for ADTs (`Kind(ADT) = sup(Kind(fields))`).

2. **Affine closures:** If a closure captures single-use values, the closure itself becomes single-use. Requires propagating kind through closure types.

3. **Borrowing:** `&FsCap` as a way to "reference" a capability without consuming it. Requires lifetime tracking. Major complexity — evaluate carefully.

4. **Effect handlers with continuations:** If continuations capture single-use values, continuations must be single-use (at-most-once). Multi-shot continuations are incompatible with single-use captures.

## Security Rationale

Affine types complete Strata's capability security model:

| Property | Enforced by | Issue |
|----------|-------------|-------|
| No forging | Capabilities are built-in types, reserved names | 009 |
| No ambient authority | All effects require matching capabilities | 009 |
| No duplication | Single-use enforcement (at most once) | 010 |
| No leaking | Cannot store in ADTs or closures | 009 + 010 |
| Explicit flow | Capabilities must be threaded through calls | 009 + 010 |
| Auditability | Capability usage visible in function signatures | 009 |

After Issue 010, the security promise is: **capabilities cannot be forged, cloned, or leaked.** Authority flow is explicit, auditable, and compiler-enforced.

## Design Principles Check

Verified against `docs/design-principles.md`:

- **FM1 (Correctness ≠ Safety):** ✅ Single-use enforcement is a safety property, not just correctness
- **FM2 (Smarter than human):** ✅ No hidden inference of affinity — it's intrinsic to the type
- **FM3 (Elegance before failure):** ✅ Error messages defined, scoping rules specified, edge cases covered
- **FM4 (Priesthood):** ✅ Permission/transfer vocabulary, not Rust jargon. SRE-friendly errors.
- **FM5 (Restriction vs governance):** ✅ Capabilities are governed (tracked, typed), not forbidden
- **FM6 (Composition erases responsibility):** ✅ Single-use threading preserves authority provenance
- **FM7 (Adoption over integrity):** ✅ No shortcuts — full enforcement from day one
- **FM8 (Outlive creators):** ✅ Enforcement is in the compiler, not documentation
