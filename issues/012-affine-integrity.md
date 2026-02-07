# Issue 012: Affine Integrity Sprint (Runtime Hardening)

## Status: Planning

## Goal

Harden the tree-walking interpreter to enforce affine semantics at runtime,
making it a faithful "Golden Specification" for any future compiled backend
(WASM or native via Cranelift).

Currently, the interpreter uses Rust's Clone for Value, meaning Value::Cap
is trivially clonable at the Rust level. The static Move Checker prevents
double-use in well-typed programs, but a Move Checker bug would silently
allow capability duplication at runtime. The interpreter must independently
enforce affine semantics as defense-in-depth.

**Origin:** Type theory review identified this as a semantic drift risk —
if the interpreter (the Golden Spec) allows clones that a compiled backend
wouldn't, programs tested against the interpreter will break under compilation.
Adversarial security review found a concrete exploit in the proposed fix.

## What This Delivers

### 1. Value::Consumed Tombstone
New Value variant that replaces affine values after they're moved:
```rust
Value::Consumed { var_name: String, moved_at: Span }
```
Accessing a consumed value produces a clear runtime error:
"Capability 'X' was already moved at [Span]. This is a runtime safety violation."

### 2. Env::move_out() — Destructive Read
New method on Env. When evaluating Expr::Var for a Kind::Affine variable,
the interpreter takes the value out and leaves a tombstone. Non-affine
variables continue using get().clone().

### 3. Closure Capture Hollowing
When a closure captures an affine variable, the variable is moved from the
parent scope into the closure's env. The parent scope gets tombstoned. This
prevents both the closure and the parent from accessing the same capability.

Closure instantiation is a consumption event:
```strata
let fs = get_fs_cap();
let f = || { read_file(&fs, "test"); };
// After this line, `fs` in parent scope is Consumed
// The capability lives only inside f's captured environment
```

### 4. Match Scrutinee Consumption
Match on an affine variable moves it into the pattern binding. The original
variable is tombstoned.

### 5. Borrow Exception
Expr::Borrow does NOT trigger destructive read. Borrows are second-class
(type-level enforcement from 011a Fix 1) and can safely clone because
the reference cannot escape extern fn parameters.

## Prerequisites
- Issue 011a (traced runtime — COMPLETE, v0.0.11)

## Security Model
- **Static:** Move Checker prevents double-use at compile time (existing)
- **Dynamic:** Interpreter enforces destructive reads at runtime (this issue)
- **Combined:** Even a Move Checker bug cannot lead to capability duplication

## Key Implementation Notes
- Value::Consumed must include var_name and moved_at Span for clear errors
- eval_expr for Expr::Var must check Kind::Affine to decide get() vs move_out()
- Closure creation must use the move_check capture list to know which vars to hollow
- Env cloning for closures must happen AFTER hollowing the parent
- move_out must key off type metadata (Kind::Affine), not pattern-match on Value::Cap —
  this ensures correctness when affine types expand beyond caps (channels, futures, etc.)

## Critical Finding: Scope-Aware Tombstoning (Adversarial Review)

**EXPLOITABLE BUG (if implemented naively):**

Env uses Vec<HashMap> for scopes. A naive move_out that inserts Consumed into
the current scope would SHADOW the outer binding instead of tombstoning it:

```strata
fn main(cap: FsCap) {
    {
        read_file(cap, "inner");  // naive move_out shadows 'cap' with Consumed in inner scope
    }
    read_file(cap, "outer");  // outer scope cap still alive — capability bypass!
}
```

**Required fix:** move_out MUST traverse scopes in reverse order (like get/lookup
does) and tombstone at the ACTUAL binding depth where the variable lives. Do NOT
insert a shadow in the current scope.

Use generation-aware BindingId (same pattern as the Issue 010 fix where binding_types
was re-keyed from String to BindingId). This is the same class of bug recurring.
See lessons-learned.md.

## Full Adversarial Review Summary

| Attack Vector | Rating | Action |
|---|---|---|
| 1. Tombstone bypass (nested closures) | Theoretical | Ensure capture analysis is recursive |
| 2. Closure capture timing (loops, branches) | Mitigated | Static move checker + runtime align |
| 3. Env scoping/shadowing | **EXPLOITABLE** | move_out must traverse scopes like lookup |
| 4. Borrow exception loophole | Theoretical | Hosts trusted for v0.1; Ty::Ref second-class |
| 5. Error path exploitation | Mitigated | Affine consumption before call is consistent |

**Additional concern:** Runtime cap detection should key off type metadata
(Kind::Affine), not Value::Cap pattern matching. When affine types expand beyond
capabilities (channels, futures), tombstoning must still work.

## Design Review: Complete — ready for implementation planning
