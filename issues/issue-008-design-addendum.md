# Issue 008 Design Review — Codebase Verification Addendum

> Added after reading the actual v0.0.7.0+ source code  
> Attach this alongside issue-008-design-review.md for reviewers

## Corrections & Observations

### 1. Effect Enum Mismatch

**Design doc proposes:** Net, Fs, Time, Rand, Fail, Ai  
**Actual `effects.rs` has:** Io, Fs, Net, Time, Random, Ffi, Panic, Unwind

**Reconciliation needed:**

| Existing | Keep? | Rename? | Notes |
|----------|-------|---------|-------|
| Io | Drop | — | Too generic. Fs and Net cover the specific I/O |
| Fs | Keep | — | Filesystem |
| Net | Keep | — | Network |
| Time | Keep | — | Time operations |
| Random | Keep | Rand | Shorter, matches design |
| Ffi | Drop (v0.1) | — | No FFI in v0.1 |
| Panic | Rename | → Fail | Recoverable failure, unparameterized |
| Unwind | Drop (v0.1) | — | Stack unwinding is runtime, not v0.1 |
| — | Add | Ai | New: LLM/model calls |

**Proposed v0.1 Effect enum:**
```rust
pub enum Effect {
    Fs = 0,
    Net = 1,
    Time = 2,
    Rand = 3,
    Fail = 4,
    Ai = 5,
}
```

**Derek's call needed:** This is a breaking change to the Effect enum. Fine since it's unused in the checker, but the bitmask positions shift.

### 2. EffectRow Representation — Bitmask vs. Algebraic

**Current:** `EffectRow { mask: u64 }` — fast, simple, but **cannot represent row variables**.

**Options:**

**(A) Replace entirely:**
```rust
pub enum EffectRow {
    Closed(BTreeSet<Effect>),                    // Known, complete
    Open { known: BTreeSet<Effect>, tail: EffectVarId }, // Partially known
}
```
Pro: Clean, single type. Con: Loses bitmask performance (irrelevant at compile time).

**(B) Two-tier:**
Keep bitmask `EffectSet` for concrete operations, add `InferEffectRow` for inference:
```rust
// For concrete effect algebra (profiles, capability checking)
pub struct EffectSet { mask: u64 }

// For inference (new)
pub struct InferEffectRow {
    pub concrete: BTreeSet<Effect>,
    pub tail: Option<EffectVarId>,
}
```
Pro: Existing code keeps working. Con: Two representations to maintain.

**(C) Extend bitmask:**
```rust
pub struct EffectRow {
    mask: u64,
    tail: Option<EffectVarId>,
}
```
Pro: Minimal change. Con: BTreeSet is cleaner for display/iteration, bitmask has fixed effect limit.

**My recommendation:** Option (A). The bitmask was premature optimization for a compiler that processes files in milliseconds. BTreeSet gives us canonical ordering for Display, easy iteration, and natural extension to user-defined effects (v0.2+). The bitmask's 64-effect limit is fine but the ergonomic cost isn't worth it.

### 3. Ty::Arrow Has No Effect Row

**Current:** `Arrow(Vec<Ty>, Box<Ty>)`  
**Needed:** `Arrow(Vec<Ty>, Box<Ty>, EffectRow)`

**Impact radius:** Every match on `Ty::Arrow(params, ret)` across:
- `ty.rs` (Display, free_vars, constructors)
- `unifier.rs` (unification)
- `subst.rs` (substitution application)
- `checker.rs` (~1017 lines, multiple matches)
- `constraint.rs` (~49K — the big one)
- `infer/tests.rs`

**Strategy:** Add the field, let the compiler tell you everywhere it breaks. This is mechanical but tedious — probably 1-2 hours of find-and-fix. The compiler is your friend here.

### 4. Scheme Needs Effect Variable Quantification

**Current:** `Scheme { vars: Vec<TypeVarId>, ty: Ty }`  
**Needed:** `Scheme { type_vars: Vec<TypeVarId>, effect_vars: Vec<EffectVarId>, ty: Ty }`

This affects generalization (which effect variables to quantify) and instantiation (creating fresh effect variables alongside fresh type variables).

### 5. Subst Needs Effect Variable Mapping

**Current:** `Subst { map: HashMap<TypeVarId, Ty> }`  
**Needed:** Also `effect_map: HashMap<EffectVarId, EffectRow>`

The `apply` method needs to walk effect rows inside `Ty::Arrow` and substitute effect variables.

### 6. Constraint Needs Effect Variants

**Current:** Only `Constraint::Equal(Ty, Ty, Span)`  
**Needed:** Add effect-specific constraints:
```rust
pub enum Constraint {
    Equal(Ty, Ty, Span),
    EffectSubset(EffectRow, EffectRow, Span),  // body ⊆ declared
}
```

### 7. free_vars Needs Effect Variable Tracking

**Current:** `free_vars(ty: &Ty) -> HashSet<TypeVarId>` ignores effect rows.  
**Needed:** Also collect effect variables from Arrow's EffectRow for correct generalization.

Possibly split into `free_type_vars` and `free_effect_vars`, or return a combined struct.

### 8. FnDecl Has No Effect Annotation

**Current AST:** `FnDecl { name, params, ret_ty, body, span }`  
**Needed:** Add `effects: Option<Vec<Ident>>` (parsed effect names, resolved in checker)

### 9. TypeExpr::Arrow Has No Effect Row

**Current:** `Arrow { params, ret, span }`  
**Needed:** `Arrow { params, ret, effects: Option<Vec<Ident>>, span }` for function type annotations like `(String) -> String & {Fs}`

### 10. Crate Count

**Design doc says 4 crates.** Actual repo has 8 crate directories:
strata-ast, strata-cli, strata-codegen, strata-ir, strata-logic, strata-parse, strata-rt, strata-types, strata-vm

Most of the extra crates (codegen, ir, logic, rt, vm) appear to be stubs for future work. Issue 008 only touches strata-ast, strata-parse, strata-types, and strata-cli.
