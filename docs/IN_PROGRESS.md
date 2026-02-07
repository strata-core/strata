# Strata: In Progress

**Last Updated:** February 2026
**Current Version:** v0.0.10.1
**Current Focus:** Issue 011a (Traced Runtime — Interpreter-based)
**Progress:** Issues 001-010 complete + pre-011 security hardening

---

## Current Status

### v0.0.10.1: Pre-011 Security Hardening (Feb 2026)

**Security fix: Kind propagation for compound types**

`Ty::kind()` now propagates affinity through ADTs, tuples, and lists. Previously, `Smuggler<FsCap>` was classified as `Kind::Unrestricted`, allowing capability duplication through generic wrappers — a full capability bypass.

**What was fixed:**
- `Ty::kind()` recurses into `Ty::Adt` type args, `Ty::Tuple` elements, and `Ty::List` inner type
- Generic ADTs containing capabilities (e.g., `Smuggler<FsCap>`) are correctly `Kind::Affine`
- Defense-in-depth: move checker resolves generic field types through ADT registry substitution
- `Ty::kind()` documented for future closure affinity propagation

**New tests:** 7 tests (4 exploit/negative, 2 ban regression, 1 closure ban)
**Total tests:** 442

---

### Issue 010: Affine Types / Linear Capabilities COMPLETE (Feb 2026)

**What was built:**

**Kind System:**
- `Kind` enum: `Unrestricted` (free use) and `Affine` (at-most-once)
- `Ty::kind()` method: `Ty::Cap(_)` → `Affine`; compound types propagate affinity from contents
- Kind is intrinsic to the type — no user annotation needed

**Move Checker (post-inference pass):**
- Separate validation pass after type inference — does NOT modify unification or solver
- Tracks binding consumption: each affine binding used at most once
- Generation-based binding IDs for correct shadowing
- Let-binding transfers ownership: `let a = fs;` consumes `fs`, makes `a` alive
- Function call arguments evaluated left-to-right with cumulative state
- Pessimistic branch join: if consumed in ANY branch, consumed after if/else/match
- Loop rejection: capability use inside while loops is an error
- Polymorphic return type resolution via manual scheme instantiation

**CapabilityInBinding Replaced:**
- `let cap = fs;` is now a valid ownership transfer (was error in Issue 009)
- Move checker enforces single-use instead of blanket rejection
- ADT field capability ban remains (caps cannot be struct/enum fields)

**Error Messages (permission/authority vocabulary):**
- `capability 'fs' has already been used; permission was transferred at ...; 'fs' is no longer available`
- `cannot use single-use capability 'fs' inside loop; 'fs' would be used on every iteration`
- Designed for SRE/DevOps audience, not PL researchers

**Security Milestone: Capability Model Complete**

| Property | Enforced by | Issue |
|----------|-------------|-------|
| No forging | Built-in types, reserved names | 009 |
| No ambient authority | Effects require matching capabilities | 009 |
| No duplication | Affine single-use enforcement | 010 |
| No leaking | Cannot store in ADTs or closures | 009 + 010 |
| Explicit flow | Capabilities threaded through calls | 009 + 010 |
| Auditability | Capability usage visible in signatures | 009 |

**Test stats:** 442 tests passing (33 move checker + 7 hardening)

---

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

**Critical Fix: Zero-Cap Skip Eliminated:**
- Removed `if !param_caps.is_empty()` guard that allowed zero-cap functions to bypass enforcement
- Three independent external reviewers flagged this as critical

**External Validation (3 Independent Sources):**
- Security Review: 7/10 confidence (up from 5/10) — "Core bypass patched"
- PL Researcher: "Green Light — mathematically sound capability-security model"
- Technical Assessment: 0.87-0.90 (up from 0.80-0.85) — "Single biggest security/DX footgun removed"

---

### Issue 008: Effect System COMPLETE (Feb 5, 2026)

- Remy-style row polymorphism for effects
- Two-phase constraint solver (type equality then effect subset)
- Bitmask-based EffectRow with open/closed rows
- Effect inference, checking, and propagation

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

## Development Lessons

### Key Learnings from Issue 010
- **Move checker is a separate pass:** Keeping it out of the unifier/solver preserves inference simplicity
- **Pessimistic branch join is correct for safety:** If consumed in ANY branch, treat as consumed
- **Permission vocabulary matters:** "Capability has already been used" is clearer than "moved value" for our target audience

### Key Learnings from Issue 009
- **Security enforcement must never be opt-in:** The zero-cap skip guard was a critical bypass. Three independent reviewers caught it. Rule: if removing a guard breaks tests, update the tests.
- **Multi-source review is now proven:** Three independent reviewers converged on the same critical finding.
- **Shared validation helpers prevent drift:** `validate_caps_against_effects()` ensures fn and extern paths stay consistent.

### Development Philosophy
- **Soundness over speed** — A type system that lies is worse than no type system
- **Foundation integrity** — Fix bugs before they compound
- **Post-completion review** — Always audit before moving forward
- **Rigor over ergonomics** — Never compromise safety for convenience

---

## Project Statistics

**Codebase:**
- 4 crates (strata-ast, strata-parse, strata-types, strata-cli)
- ~10,000+ lines of Rust code
- 442 tests (all passing)
- 0 clippy warnings (enforced)

**Velocity:**
- Issues 001-010: ~2.5 months
- On track for v0.1 timeline
