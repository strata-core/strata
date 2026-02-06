# Strata: In Progress

**Last Updated:** February 7, 2026
**Current Version:** Post-Issue 009 (pre-tag)
**Current Focus:** Issue 010 (Affine Types / Linear Capabilities - "The Final Boss")
**Progress:** Issues 001-009 complete

---

## Current Status

### Issue 009: Capability Types COMPLETE (Feb 6-7, 2026, 1 week)

**What was built:**

**Capability Types:**
- Five capability types: `FsCap`, `NetCap`, `TimeCap`, `RandCap`, `AiCap`
- First-class `Ty::Cap(CapKind)` variant in the type system (not ADTs)
- `CapKind` enum with 1:1 mapping to effects

**Mandatory Capability Validation:**
- `validate_caps_against_effects()` shared helper — mandatory for ALL functions and externs
- Functions with concrete effects MUST have matching capability parameters (no exceptions)
- Extern functions with declared effects MUST have matching capability parameters
- Open tail effect variables (from HOF callbacks) are not checked (correct — parametric effects)
- Unused capabilities are allowed (pass-through pattern)

**Name Shadowing Protection:**
- ADT definitions cannot shadow capability type names (FsCap, NetCap, etc.)
- `ReservedCapabilityName` error for structs/enums using capability names

**Ergonomic Error Messages:**
- Clear "missing capability" errors with help text suggesting the missing parameter
- Hints for FsCap/Fs confusion (effect name used as type or vice versa)
- Extern error includes suggestion to remove effect annotation if actually pure

**Critical Fix: Zero-Cap Skip Eliminated:**
- Removed `if !param_caps.is_empty()` guard that allowed zero-cap functions to bypass enforcement
- Removed `has_any_cap` guard that allowed zero-cap externs to bypass enforcement
- Three independent external reviewers flagged this as critical (see External Validation below)
- Adversarial test `adversarial_zero_caps_entire_call_chain` verifies the fix

**External Validation (3 Independent Sources):**
- Security Review: 7/10 confidence (up from 5/10) — "Core bypass patched"
- PL Researcher: "Green Light — mathematically sound capability-security model"
- Technical Assessment: 0.87-0.90 (up from 0.80-0.85) — "Single biggest security/DX footgun removed"

**Known Limitations:**
- Open effect tail polymorphism at instantiation sites may need additional checking
- No warnings for unused capabilities (intentional for now — pass-through pattern)
- Diagnostics could be more polished (span formatting)
- Capability provisioning (how `main` gets capabilities from runtime) deferred to Issue 011

**Test stats:** 402 tests passing (44 new, 11% increase)

**Example:**
```strata
// Extern functions must have cap params matching their effects
extern fn read_file(path: String, fs: FsCap) -> String & {Fs};

// Functions with effects must have matching capabilities
fn load_config(fs: FsCap, path: String) -> String & {Fs} {
    read_file(path, fs)
}

// This would ERROR — no FsCap for {Fs} effect:
// fn bad() -> () & {Fs} {}
```

---

### Issue 008: Effect System COMPLETE (Feb 5, 2026)

**What was built:**

**Effect Annotations:**
- Function effect declarations: `fn f() -> Int & {Fs} { ... }`
- Multiple effects: `& {Fs, Net, Time, Rand, Ai}`
- Explicit pure: `& {}`, implicit pure (no annotation)
- Extern functions: `extern fn read(path: String, fs: FsCap) -> String & {Fs};`

**Effect Inference:**
- Unannotated functions infer effects from their body
- Call site effects propagate to enclosing function
- Multiple effects accumulate across call sites
- Transitive propagation through call chains
- Higher-order function effect propagation

**Effect Checking:**
- Body effects checked against declared annotation
- Pure functions cannot call effectful functions
- Unknown effect names produce compile-time errors
- ADT constructors are always pure

**Implementation:**
- Bitmask-based EffectRow with open/closed rows
- Remy-style row unification
- Two-phase constraint solver (type equality then effect subset)
- Fixpoint accumulation for multi-call-site propagation
- Effect variable generalization in polymorphic schemes

**Test stats:** 344 tests passing (47 new tests for effects)

---

## Recent Completions

### Issue 007: ADTs & Pattern Matching COMPLETE (Feb 4, 2026)

- Struct and enum definitions with generics
- Pattern matching with exhaustiveness checking
- Tuple types and destructuring
- Evaluator support for all ADT operations

---

### Issue 006-Hardening COMPLETE (Feb 2-3, 2026)

- DoS protection limits across all crates
- Soundness fixes for Never type and divergence
- Removed panic!/expect() from type checker

---

## Starting Issue 010: Affine Types - "The Final Boss"

### Issue 010: Affine Types / Linear Capabilities
**Status:** Not started
**Phase:** 3 (Effect System)
**Depends on:** Issue 009 (complete)
**Blocks:** Issue 011

**Goal:** Prevent capability duplication with affine/linear type system. Capabilities are use-at-most-once.

**External validation:**
> "This is the 'Final Boss' of your type system. If you can implement Affine Types without making the language feel like Rust, you will have achieved something truly unique."

**What this enables:**
- Capabilities can't be duplicated (no `let fs2 = fs; use_both(fs, fs2)`)
- Capabilities can't be leaked (dropped is fine — affine, not linear)
- Foundation for Demo 2: Meta-Agent Orchestration with Affine Types

---

## Development Lessons

### Key Learnings from Issue 009
- **Security enforcement must never be opt-in:** The zero-cap skip guard was a critical bypass. Three independent reviewers caught it. Rule: if removing a guard breaks tests, update the tests.
- **Multi-source review is now proven:** Three independent reviewers converged on the same critical finding. This process works and must continue for Issue 010.
- **Shared validation helpers prevent drift:** `validate_caps_against_effects()` ensures fn and extern paths stay consistent.
- **Name shadowing is a real attack vector:** Users could define `struct FsCap {}` to confuse the type system. Reserved name check prevents this.

### Development Philosophy
- **Soundness over speed** — A type system that lies is worse than no type system
- **Foundation integrity** — Fix bugs before they compound
- **Post-completion review** — Always audit before moving forward
- **Rigor over ergonomics** — Never compromise safety for convenience

---

## Project Statistics

**Codebase:**
- 4 crates (strata-ast, strata-parse, strata-types, strata-cli)
- ~9,000+ lines of Rust code
- 402 tests (all passing)
- 0 clippy warnings (enforced)

**Velocity:**
- Issues 001-009: ~2.5 months
- On track for v0.1 timeline
