# Strata: In Progress

**Last Updated:** February 2026
**Current Version:** v0.0.11
**Current Focus:** Post-011a — next issue TBD
**Progress:** Issues 001-010 + 011a complete

---

## Current Status

### Issue 011a: Traced Runtime COMPLETE (Feb 2026)

**What was built:**

**5 phases, 25 new tests (477 total):**

**Phase 1: Extern-Only Borrowing**
- `&CapType` syntax in extern fn params — borrow without consuming
- `Ty::Ref(Box<Ty>)` reference type, restricted to extern fn params
- Move checker treats borrows as non-consuming

**Phase 2: Host Function Dispatch**
- `HostRegistry` with 4 built-in host fns (read_file, write_file, now, random_int)
- Capability injection at main() entry point
- Position-aware dispatch via ExternFnMeta

**Phase 3: Effect Trace Emission**
- Streaming JSONL trace for every host fn call
- SHA-256 content hashing for outputs > 1KB
- Single dispatch path: always `dispatch_traced()`

**Phase 4: Deterministic Replay**
- `TraceReplayer` loads JSONL, validates operations + inputs
- Structured `ReplayError` enum for mismatch reporting
- `run_module_replay()` with `verify_complete()`

**Phase 5: CLI Integration**
- `strata run` / `strata replay` / `strata parse` subcommands
- `--trace` (audit, hashed) and `--trace-full` (replay-capable) flags
- Trace summary output, example program + documentation

**End-to-end workflow:**
```bash
strata run program.strata --trace-full trace.jsonl
strata replay trace.jsonl program.strata
# "Replay successful: 3 effects replayed."
```

---

### Previous Completions

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

### Development Philosophy
- **Soundness over speed** — A type system that lies is worse than no type system
- **Foundation integrity** — Fix bugs before they compound
- **Post-completion review** — Always audit before moving forward
- **Security enforcement must never be opt-in** — The zero-cap skip incident

---

## Project Statistics

**Codebase:**
- 4 crates (strata-ast, strata-parse, strata-types, strata-cli)
- ~12,000+ lines of Rust code
- 477 tests (all passing)
- 0 clippy warnings (enforced)

**Velocity:**
- Issues 001-010 + 011a: ~2.5 months
- On track for v0.1 timeline
