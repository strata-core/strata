# Strata: In Progress

**Last Updated:** February 2026
**Current Version:** v0.0.11
**Current Focus:** Issue 012 — Affine Integrity Sprint
**Progress:** Issues 001-010 + 011a complete

---

## Current Status

### Issue 012: Affine Integrity Sprint — PLANNING

**Goal:** Harden the interpreter to enforce affine semantics at runtime
(defense-in-depth). Even a Move Checker bug cannot lead to capability
duplication.

**What will be built:**
- `Value::Consumed` tombstone for moved values
- `Env::move_out()` — destructive read for affine variables
- Closure capture hollowing
- Match scrutinee consumption
- Scope-aware tombstoning (critical finding from adversarial review)

**Why now:** The interpreter is the v0.1 runtime and the Golden Specification.
It must be correct before we expand the stdlib (Issue 013).

**v0.1 critical path:** 012 → 013 → 014 → 015

---

### Issue Renumbering (Feb 2026)

After completing 011a, the roadmap was reorganized. Compilation (WASM/native)
deferred to v0.2+, changing the dependency chain:

| Old | Old Title | New | New Title | Notes |
|-----|-----------|-----|-----------|-------|
| (new) | — | 012 | Affine Integrity Sprint | Runtime hardening |
| 013 | Standard Library Primitives | 013 | (stays) | Removed cap bundles dep |
| 014 | CLI Tooling | 014 | `strata plan` + CLI Polish | Partly shipped in 011a |
| 015 | Error Reporting Polish | 015 | (stays) | No change |
| 012 | Hierarchical Cap Bundles | 016 | (demoted) | Deferred to v0.2 |

---

### Previous Completions

**Issue 011a: Traced Runtime** (Feb 2026, v0.0.11)
- 5 phases + hardening, 41 new tests (493 total)
- Borrowing, host dispatch, trace emission, replay, CLI
- 6 hardening fixes from external review

**Issue 010: Affine Types / Linear Capabilities** (Feb 2026)
- Kind system, move checker, single-use enforcement
- 442 tests at completion

**Issue 009: Capability Types** (Feb 2026)
- Mandatory capability validation, zero-cap skip fix
- Externally validated by 3 independent reviewers

**Issue 008: Effect System** (Feb 2026)
- Remy-style row polymorphism, two-phase solver

**Issues 001-007:** Parser, type inference, control flow, ADTs, pattern matching

---

## Development Lessons

### Key Learnings from Issue 011a
- **Single dispatch path**: Always use `dispatch_traced()` — never branch on tracer presence for dispatch logic. `TraceEmitter::disabled()` handles the no-output case.
- **Position-aware dispatch**: Walk the extern fn's type signature to classify params as Cap vs Data. Don't filter by runtime `Value::Cap(_)` type.
- **Replay before live**: Check replayer BEFORE live dispatch in the call path. Replayer takes priority.
- **Security enforcement unconditional**: Missing ExternFnMeta is a hard error, not a graceful degradation.
- **Tagged serialization**: Never serialize values without preserving type identity (Str vs Int).
- **Scope-aware operations**: ANY Env operation targeting a specific binding must resolve through the full scope chain, never just the current scope.

### Development Philosophy
- **Soundness over speed** — A type system that lies is worse than no type system
- **Foundation integrity** — Fix bugs before they compound
- **Post-completion review** — Always audit before moving forward
- **Security enforcement must never be opt-in** — The zero-cap skip incident
- **Defense in depth** — Static checks AND runtime enforcement

---

## Project Statistics

**Codebase:**
- 4 crates (strata-ast, strata-parse, strata-types, strata-cli)
- ~12,000+ lines of Rust code
- 493 tests (all passing)
- 0 clippy warnings (enforced)

**Velocity:**
- Issues 001-010 + 011a: ~2.5 months
- On track for v0.1 timeline
