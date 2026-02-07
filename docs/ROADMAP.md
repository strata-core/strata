# Strata Roadmap

**Last Updated:** February 2026
**Current Version:** v0.0.11
**Target v0.1:** November 2026 - February 2027 (10-14 months from project start)

This document describes the planned roadmap for Strata, organized by development phases. It reflects strategic decisions made in January 2026 to focus on a **minimal, shippable v0.1** that proves core value propositions.

---

## Quick Navigation

- [Strategic Focus](#strategic-focus-for-v01)
- [Phase 2: Type System](#phase-2-type-system-foundations-complete-) (Complete)
- [Phase 3: Effect System](#phase-3-effect-system-complete-) (Complete)
- [Phase 3.5: Traced Runtime](#phase-35-traced-runtime-complete-) (Complete)
- [Phase 4: v0.1 Critical Path](#phase-4-v01-critical-path)
- [Phase 5: Hardening & Launch](#phase-5-hardening--launch)
- [Deferred Features](#explicitly-deferred-to-v02)
- [Timeline Summary](#timeline-summary)

---

## Strategic Focus for v0.1

**Target User:** Automation Engineer (AI/ML Ops, SRE, DevOps, MLOps, Security)

**Core Value Propositions:**
1. Effect-typed safety (explicit side effects in types)
2. Capability security (no ambient authority)
3. Deterministic replay (reproduce failures from traces)
4. Explainability (full audit trail of all effects)

**Killer Demos:** See [`KILLER_DEMOS.md`](KILLER_DEMOS.md)
1. Model Deployment with Deterministic Replay
2. Meta-Agent Orchestration with Affine Types (AI safety)
3. Time-Travel Bug Hunting (tooling demo)

**v0.1 Success Metric:** An automation engineer can write a production deployment script in Strata that is safer, more auditable, and easier to debug than Python/Bash equivalents.

**v0.1 Runtime:** The traced tree-walking interpreter is the v0.1 runtime. Compilation (WASM/native) is deferred to v0.2+.

---

## Important Notes

### Design Philosophy: Rigor Over Ergonomics

**Core principle (external validation):**
> "If an automation engineer has to type `using fs: FsCap` to delete a file, don't apologize for it. That explicitness is exactly why the world needs Strata."

**The lesson from "Zero-Cap Skip" incident:**
Attempted "ergonomic shortcut" (bypass for functions without capabilities) fundamentally broke the security proof. It was caught, scrutinized, and fixed. This is the right approach - **never compromise safety for convenience**.

### Compilation Deferral (Feb 2026 Decision)

WASM and native compilation are deferred to v0.2+. Rationale:
- The interpreter is sufficient for v0.1 target workloads (I/O-bound automation)
- Compilation adds complexity without changing the safety story
- The interpreter serves as the "Golden Specification" — if it's correct, compiled
  backends can be validated against it
- **WASM** targets sandboxed/untrusted AI agent execution (v0.3)
- **Native compilation** (Cranelift) targets SRE deployment story (v0.3+)
- Both are enabled by the interpreter's execution-engine-agnostic host ABI

### Performance Philosophy

Performance optimization is explicitly **deferred to v0.2+**. v0.1 prioritizes correctness, safety, and usability over speed.

**Why performance is not a v0.1 concern:**
- Target users (automation engineers) care about safety and audit trails, not microsecond optimizations
- Automation scripts are I/O-bound (network, disk, AI APIs), not CPU-bound
- Variable lookup overhead is noise compared to HTTP request latency

**When to optimize:** After v0.1 ships, after profiling real-world usage, if users report performance issues.

### Balancing Five Concerns

Strata balances five equally important concerns: type soundness, operational reality, human factors, security, and maintainability. This "anti-single-axis" approach means features are judged by their worst-case behavior, not best-case elegance.

See [`DESIGN_DECISIONS.md`](DESIGN_DECISIONS.md) for complete design philosophy and technical decisions.

---

## Phase 1: Foundation (Complete)

**Timeline:** Months 0-1 (Nov 2025 - Dec 2025)
**Status:** COMPLETE

### Issue 001: Parser + AST
### Issue 002: Effects & Profiles
### Issue 003: Type Inference Scaffolding
### Issue 004: Basic Type Checking

**Phase 1 Result:** Solid foundation with parser, AST, basic type system, and effect types.

---

## Phase 2: Type System Foundations (Complete)

**Timeline:** Months 1-4 (Jan 2026 - Apr 2026)
**Status:** COMPLETE (Issues 005-007)

### Issue 005: Functions & Inference + 005-b Soundness Hardening
### Issue 006: Blocks & Control Flow + 006-Hardening
### Issue 007: ADTs & Pattern Matching

**Phase 2 Result:** Complete type system with HM inference, polymorphism, ADTs, and pattern matching.

---

## Phase 3: Effect System (Complete)

**Timeline:** Months 4-7 (Feb 2026)
**Status:** COMPLETE (Issues 008-010)

### Issue 008: Effect System
- Bitmask-based EffectRow with 5 effects
- Two-phase constraint solver
- Effect variable generalization

### Issue 009: Capability Types
- Mandatory capability validation
- Zero-cap skip fix (critical security finding)
- Externally validated by 3 independent reviewers

### Issue 010: Affine Types (Linear Capabilities)
- Move checker as separate post-inference pass
- Use-at-most-once enforcement
- Pessimistic branch analysis

**Phase 3 Result:** Complete capability security model with affine types.

---

## Phase 3.5: Traced Runtime (Complete)

**Timeline:** Feb 2026
**Status:** COMPLETE (Issue 011a, v0.0.11, 493 tests)

### Issue 011a: Traced Runtime (5 phases + hardening)
- Phase 1: Extern-only borrowing (`&CapType`)
- Phase 2: Host function dispatch with capability injection
- Phase 3: Effect trace emission (streaming JSONL)
- Phase 4: Deterministic replay from effect traces
- Phase 5: CLI integration (`strata run`, `strata replay`, `strata parse`)
- Hardening: 6 fixes from external review (tagged serialization, schema versioning, etc.)

**Phase 3.5 Result:** End-to-end traced runtime. Programs execute with real effects and produce auditable JSONL traces. Deterministic replay verified.

---

## Phase 4: v0.1 Critical Path

**Timeline:** Months 7-10 (Mar 2026 - Jun 2026)
**Focus:** Harden runtime, expand stdlib, ship killer demos

**v0.1 critical path: 012 → 013 → 014 → 015**

### Issue 012: Affine Integrity Sprint (Runtime Hardening)
**Estimated time:** 1-2 weeks
**Status:** Planning

Harden the interpreter to enforce affine semantics at runtime (defense-in-depth):
- `Value::Consumed` tombstone for moved values
- `Env::move_out()` — destructive read for affine variables
- Closure capture hollowing
- Match scrutinee consumption
- Scope-aware tombstoning (critical finding from adversarial review)

**Why first:** The interpreter is the v0.1 runtime and the Golden Specification for
future backends. It must be correct before we expand the stdlib.

### Issue 013: Standard Library Primitives
**Estimated time:** 2-3 weeks
**Status:** Planning
**Depends on:** Issue 012 (runtime must be sound)

Expand beyond the 4 built-in host functions:
- HTTP client (`http_get`, `http_post`)
- File operations (`file_exists`, `list_dir`)
- String utilities (`len`, `concat`, `to_string`)
- JSON extraction (`parse_json`, `json_get`)
- Time operations (`sleep`)

All effectful operations automatically participate in tracing.

### Issue 014: `strata plan` + CLI Polish
**Estimated time:** 2-3 weeks
**Status:** Planning
**Depends on:** Issue 013 (meaningful plans need real host functions)

`strata plan` is the killer adoption demo: static effect/capability analysis
without executing. "Terraform plan for automation." Shows security teams exactly
what a script will do before it runs.

Also: `strata check`, `--deny <cap>` flag, JSON output format.

### Issue 015: Error Reporting Polish
**Estimated time:** 1-2 weeks
**Status:** Planning
**Can run in parallel with:** 013, 014

Production-quality diagnostics with source snippets, underline markers,
"Help:" suggestions, and color output. Consider `ariadne` or `miette`.

**Phase 4 Result:** Sound runtime, expanded stdlib, killer `strata plan` demo, polished errors.

---

## Phase 5: Hardening & Launch

**Timeline:** Months 10-14 (Jul 2026 - Nov 2026)
**Focus:** Make v0.1 production-ready and ship it

### 5.1: Developer Experience
- Improved error messages with suggestions
- Stack traces with source spans
- Help text and documentation strings

### 5.2: Tooling Basics
- LSP server (basic completion, go-to-definition)
- Formatter (canonical style)
- Test runner (`strata test`)

### 5.3: Documentation
- Tutorial (from first program to killer demo)
- Effect system guide
- Capability security explained (for non-PL people)
- Standard library reference

### 5.4: Examples & Demos
- Polish the three killer demos
- 10-15 example programs
- Blog posts explaining key features

### 5.5: Testing & CI
- CI pipeline (GitHub Actions)
- Benchmark suite
- Integration tests

### 5.6: Release
- Binary releases (Linux, macOS, Windows)
- Installation script
- Release notes
- Launch announcements

**Phase 5 Result:** Production-ready v0.1 with great DX, docs, and tooling.

---

## Post-v0.1 Vision

### v0.2: The Sandbox Release
- Effect virtualization — shallow handlers (testing, sandboxing, dry-run)
- Capability attenuation — coarse-grained least-privilege delegation
- Hierarchical capability bundles (Issue 016)

### v0.3: The Compilation Release
- **WASM compilation** — sandboxed/untrusted AI agent execution
- **Native compilation** (Cranelift) — SRE deployment story
- Both validated against the interpreter (Golden Specification)
- Full algebraic effect handlers (with resume/continuations)
- Structured reason tracing (OTel-compatible audit trails)

### v0.3+: The Orchestration Release
- Durable execution (runtime product, not language feature)
- Capability delegation (WASM Component Model FFI)
- Temporal/leased capabilities (time-bounded authority)

See `docs/feature-candidates.md` for detailed assessments and security analysis.

---

## Explicitly Deferred to v0.2+

These features are important but not required for v0.1:

### Deferred Language Features
- **Compilation** (WASM + Cranelift native) — interpreter is v0.1 runtime
- **Hierarchical Capability Bundles** (Issue 016 — granular Read/Write)
- **Profile Enforcement** (`@profile(Kernel)` annotations)
- **Effect Handlers** (algebraic effects with continuations)
- **Capability Attenuation** (Type operations: `Cap<A+B>` → `Cap<A>`)
- **User-facing effect type variables** (explicit `fn f<E>() -> T & E` syntax)
- **Actors & supervision** (concurrency model)
- **Async/await syntax** (concurrency primitives)
- **Advanced traits** (associated types, defaults, etc.)
- **Logic programming (Datalog)** (explainability engine)

### Deferred Tooling
- Debugger
- REPL
- Package manager
- Documentation generator
- IDE plugins beyond LSP

### Deferred Stdlib
- Advanced collections (trees, graphs, ropes)
- Database drivers (beyond HTTP wrappers)
- Advanced HTTP (streaming, WebSockets)
- Crypto beyond hashing
- Compression

**Rationale for deferrals:** Each deferred feature saves 1-6 months. Focus on proving core value (effects + capabilities + replay) before expanding scope.

---

## Issue Renumbering (Feb 2026)

After completing Issue 011a, the roadmap was reorganized. Compilation (WASM/native)
was deferred, changing the dependency chain:

| Old | Old Title | New | New Title | Notes |
|-----|-----------|-----|-----------|-------|
| (new) | — | 012 | Affine Integrity Sprint | Runtime hardening |
| 013 | Standard Library Primitives | 013 | (stays) | Removed cap bundles dep |
| 014 | CLI Tooling | 014 | `strata plan` + CLI Polish | Partly shipped in 011a |
| 015 | Error Reporting Polish | 015 | (stays) | No change |
| 012 | Hierarchical Cap Bundles | 016 | (demoted) | Deferred to v0.2 |

---

## Timeline Summary

**Phase 1 (Complete):** Foundation - Parser, AST, Effects, Basic Types
**Months 0-1**

**Phase 2 (Complete):** Type System - Functions, Inference, ADTs, Patterns
**Months 1-4**

**Phase 3 (Complete):** Effect System - Effect checking, Capabilities, Affine Types
**Months 4-7** (Issues 008-010)

**Phase 3.5 (Complete):** Traced Runtime - Host dispatch, Tracing, Replay, CLI
**Feb 2026** (Issue 011a, v0.0.11)

**Phase 4:** v0.1 Critical Path - Affine integrity, Stdlib, strata plan, Error polish
**Months 7-10** (Issues 012-015)

**Phase 5:** Hardening & Launch - DX, Tooling, Docs, Release
**Months 10-14**

**Total: 10-14 months from project start to v0.1 launch**

---

## Current Status (v0.0.11)

**Completed:**
- Issues 001-010 + 011a (all phases + hardening)
- 493 tests (all passing)
- 4 crates (ast, parse, types, cli)
- 0 clippy warnings (enforced)

**Next Up:**
- Issue 012: Affine Integrity Sprint (runtime hardening)
- Issue 013: Standard Library Primitives
- Issue 014: `strata plan` + CLI Polish
- Issue 015: Error Reporting Polish
- Issue 016: Hierarchical Cap Bundles (deferred to v0.2)

**External validation (Feb 2026, 3 independent sources):**
- Security Review: 7/10 — "Core bypass patched, no ambient authority enforced"
- PL Researcher: "Green Light — mathematically sound capability-security model"
- Technical Assessment: 0.87-0.90 — "Single biggest security/DX footgun removed"

---

## Success Metrics

### Technical Milestones
- Parser working
- Basic type checking
- HM inference with polymorphism
- Control flow (if/while/return)
- ADTs and pattern matching
- Effect system complete
- Capability types — externally validated
- Affine types / linearity — externally validated
- Traced runtime with replay — working
- Runtime affine enforcement (Issue 012)
- Working killer demos (Issue 014)

### Quality Metrics
- Test coverage >90% (currently ~85%)
- Zero clippy warnings
- Deterministic behavior
- Clear error messages (improving)

---

## Related Documents

- [`IMPLEMENTED.md`](IMPLEMENTED.md) - Detailed feature status
- [`IN_PROGRESS.md`](IN_PROGRESS.md) - Current work and next steps
- [`DESIGN_DECISIONS.md`](DESIGN_DECISIONS.md) - Technical and architectural choices
- [`KILLER_DEMOS.md`](KILLER_DEMOS.md) - v0.1 demo specifications
- [`feature-candidates.md`](feature-candidates.md) - Post-v0.1 feature assessments
- [`issues/`](../issues/) - Issue specifications

---

**Last Updated:** February 2026
**Current Version:** v0.0.11
**Next Review:** After Issue 012 completion
