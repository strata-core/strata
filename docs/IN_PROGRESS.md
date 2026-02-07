# Strata: In Progress

**Last Updated:** February 2026
**Current Version:** v0.0.12
**Current Focus:** Issue 013 — Standard Library Primitives
**Progress:** Issues 001-012 complete

---

## Current Status

### Issue 012: Affine Integrity Sprint — COMPLETE ✅

**Completed:** February 2026

Runtime defense-in-depth for affine semantics. The interpreter now enforces
single-use independently of the static Move Checker. Even a Move Checker bug
cannot lead to capability duplication at runtime.

**What was built:**
- `Value::Consumed` tombstone for moved values
- `Env::move_out()` — destructive read with scope-aware traversal
- Recursive `is_affine()` for compound types (tuples, structs, variants)
- Borrow exemption (no tombstoning through `&x`)
- CAP-MOVE-RUNTIME error code with dual-span reporting

**Post-012 cohort findings (deferred to v0.2+):**
- CapToken with shared consumed state (`Rc<Cell>`) — eliminates alias escape class
- Ownership-based pattern matching — `match_pattern` takes Value by ownership
- Closure capture hollowing — when user-defined closures/lambdas are added

---

## Next Up

### Issue 013: Standard Library Primitives
**Status:** Planning
**Depends on:** Issue 012 (COMPLETE)

Expand beyond the 4 built-in host functions:
- HTTP client (`http_get`, `http_post`)
- File operations (`file_exists`, `list_dir`)
- String utilities (`len`, `concat`, `to_string`)
- JSON extraction (`parse_json`, `json_get`)
- Time operations (`sleep`)

All effectful operations automatically participate in tracing.

### Issue 014: `strata plan` + CLI Polish
**Depends on:** Issue 013

### Issue 015: Error Reporting Polish
**Can run in parallel with:** 013, 014

**v0.1 critical path:** 013 → 014 → 015

---

### Previous Completions

**Issue 012: Affine Integrity Sprint** (Feb 2026, v0.0.12)
- Runtime tombstones, scope-aware destructive reads, recursive is_affine()
- 507 tests at completion (14 new)
- Cohort review: 3 fixes (recursive affinity, unwrap removal, dual-span errors)

**Issue 011a: Traced Runtime** (Feb 2026, v0.0.11)
- 5 phases + hardening, 41 new tests (493 total)
- Borrowing, host dispatch, trace emission, replay, CLI
- 6 hardening fixes from external review

**Issue 010: Affine Types / Linear Capabilities** (Feb 2026)
- Kind system, move checker, single-use enforcement

**Issue 009: Capability Types** (Feb 2026)
- Mandatory capability validation, zero-cap skip fix
- Externally validated by 3 independent reviewers

**Issue 008: Effect System** (Feb 2026)
- Remy-style row polymorphism, two-phase solver

**Issues 001-007:** Parser, type inference, control flow, ADTs, pattern matching

---

## Development Lessons

### Key Learnings from Issue 012
- **Defense-in-depth must mirror ALL cases from the primary enforcement layer.** If `Ty::kind()` propagates through N type constructors, `Value::is_affine()` must propagate through the corresponding N value constructors.
- **Defense-in-depth code must never panic.** Replace `.unwrap()` with `.ok_or_else()` for actionable error messages instead of crashes.
- **Scope-aware operations are an architectural invariant.** Any Env operation targeting a binding must traverse the full scope chain, never just the current scope.

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
- 507 tests (all passing)
- 0 clippy warnings (enforced)

**Velocity:**
- Issues 001-012: ~2.5 months
- On track for v0.1 timeline
