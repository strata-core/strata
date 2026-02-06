# Strata: In Progress

**Last Updated:** February 5, 2026
**Current Version:** Post-Issue 008 (pre-tag)
**Current Focus:** Issue 009 next
**Progress:** Issues 001-008 complete + code review fixes

---

## Current Status

### Issue 008: Effect System ✅
**Completed:** February 5, 2026

**What was built:**

**Effect Annotations:**
- Function effect declarations: `fn f() -> Int & {Fs} { ... }`
- Multiple effects: `& {Fs, Net, Time, Rand, Ai}`
- Explicit pure: `& {}`, implicit pure (no annotation)
- Extern functions: `extern fn read(path: String) -> String & {Fs};`

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

**Example programs:**
```strata
extern fn read_file(path: String) -> String & {Fs};
extern fn fetch(url: String) -> String & {Net};

fn download_and_save(url: String, path: String) -> () & {Fs, Net} {
    let data = fetch(url);
    read_file(path)
}

fn apply(f, x) { f(x) }
fn use_it() -> String & {Fs} { apply(read_file, "test.txt") }
```

---

## Recent Completions

### Issue 007: ADTs & Pattern Matching ✅
**Completed:** February 4, 2026

- Struct and enum definitions with generics
- Pattern matching with exhaustiveness checking
- Tuple types and destructuring
- Evaluator support for all ADT operations

---

### Issue 006-Hardening ✅
**Completed:** February 2-3, 2026

- DoS protection limits across all crates
- Soundness fixes for Never type and divergence
- Removed panic!/expect() from type checker

---

## Next Up

### Issue 009: Capability Types
**Status:** Not started

**Goal:** Capability-based security model with linear capability tracking.

---

## Development Lessons

### Key Learnings from Issue 008
- **Two-phase solver is correct:** Separating type equality (Phase A) from effect subset (Phase B) avoids complex interactions
- **Fixpoint accumulation:** Multiple call sites need effect accumulation, not immediate binding
- **Structural refactor first:** Adding EffectRow to Arrow in Phase 1 (zero semantic change) let us verify 292 tests still passed before adding semantics
- **Open rows for inference:** Fresh effect vars with open rows let unification resolve callee effects naturally

### Development Philosophy
- **Soundness over speed** — A type system that lies is worse than no type system
- **Foundation integrity** — Fix bugs before they compound
- **Post-completion review** — Always audit before moving forward

---

## Project Statistics

**Codebase:**
- 4 crates (strata-ast, strata-parse, strata-types, strata-cli)
- ~8,500+ lines of Rust code
- 344 tests (all passing)
- 0 clippy warnings (enforced)

**Velocity:**
- Issues 001-008: ~2.5 months
- On track for v0.1 timeline
