# Strata Roadmap

**Last Updated:** February 7, 2026
**Current Version:** v0.0.9
**Target v0.1:** November 2026 - February 2027 (10-14 months from project start)

This document describes the planned roadmap for Strata, organized by development phases. It reflects strategic decisions made in January 2026 to focus on a **minimal, shippable v0.1** that proves core value propositions.

---

## Quick Navigation

- [Strategic Focus](#strategic-focus-for-v01)
- [Phase 2: Type System](#phase-2-type-system-foundations-complete-) (Complete ✅)
- [Phase 3: Effect System](#phase-3-effect-system-your-differentiator) (Current - Critical)
- [Phase 4: Runtime](#phase-4-minimal-viable-runtime)
- [Phase 5: Hardening & Launch](#phase-5-hardening--launch)
- [Deferred Features](#explicitly-deferred-to-v02)
- [Timeline Summary](#timeline-summary)
- [Important Notes](#important-notes)

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

---

## Important Notes

### Critical Path to v0.1: The Next 3 Months

**External validation from PL researcher (Feb 2026):**

> "Strata will live or die by its rigor. If you start adding 'ergonomic shortcuts' that create soundness holes, Strata becomes just another scripting language with a confusing type system."

**The Three Milestones:**

**1. Issue 009 (Capability Types):** Kill the escape hatch ✅ DONE
- Total capability coverage check — mandatory, no conditional bypass
- Effect {X} requires XCap in scope — enforced for all functions and externs
- No ambient authority enforced — zero-cap skip eliminated
- **Status:** COMPLETE (Feb 6-7, 2026) — externally validated by 3 independent sources

**2. Issue 010 (Affine Types):** "The Final Boss"
- Prevent capability duplication and leaking
- Use-at-most-once semantics without Rust-level complexity
- **Critical quote:** "If you can implement Affine Types without making the language feel like Rust, you will have achieved something truly unique."
- **Status:** After Issue 009 (2-3 weeks)

**3. Issue 011 (WASM Runtime + Traces):** Prove the value
- Formalize main() definition (caps injected by runtime)
- Effect traces for debugging
- Deterministic replay working
- Killer demos functioning
- **Status:** After Issue 010 (3-4 weeks)

**If these 3 issues succeed: Technical confidence → 90%+**

### Design Philosophy: Rigor Over Ergonomics

**Core principle (external validation):**
> "If an automation engineer has to type `using fs: FsCap` to delete a file, don't apologize for it. That explicitness is exactly why the world needs Strata."

**The lesson from "Zero-Cap Skip" incident:**
Attempted "ergonomic shortcut" (bypass for functions without capabilities) fundamentally broke the security proof. It was caught, scrutinized, and fixed. This is the right approach - **never compromise safety for convenience**.

**Positioning:**
Strata is a **security primitive**, not just a better scripting language. The secure way must be the easy way (zero-tax safety), but "easy" means "obvious and explicit," not "hidden and implicit."

### Timeline Reconciliation

External feedback suggests Phase 2-3 (type system + effects) is achievable in 3-6 months. Our 10-14 month timeline includes Phase 4-5 (runtime, stdlib, tooling, docs, killer demos) which represents the additional effort to ship a production-ready v0.1.

### Standard Library Phasing
- **Issues 005-007:** Pure functions only (no I/O)
- **Issue 008:** Effect system complete
- **Phase 4:** Add I/O stdlib (file, http, time, AI) with effects

This sequencing ensures the effect system is complete before we add effectful operations to the standard library.

### Current Evaluator Status

The evaluator (`eval.rs`) is a temporary smoke-test tool for validating type checker output. It has known limitations (no closures yet, no real I/O, limited error handling). This is intentional - it will be replaced by WASM compilation in Phase 4.

### Performance Philosophy

Performance optimization is explicitly **deferred to v0.2+**. v0.1 prioritizes correctness, safety, and usability over speed.

**Why performance is not a v0.1 concern:**
- Target users (automation engineers) care about safety and audit trails, not microsecond optimizations
- Automation scripts are I/O-bound (network, disk, AI APIs), not CPU-bound
- Variable lookup overhead is noise compared to HTTP request latency
- Premature optimization would delay shipping and add complexity

**Exception:** Effect tracing is designed to be toggleable (see Phase 4.2) to avoid overhead when not needed, but tracing is **enabled by default** since audit trails are Strata's core value proposition.

**When to optimize:** After v0.1 ships, after profiling real-world usage, if users report performance issues.

### Balancing Five Concerns

Strata balances five equally important concerns: type soundness, operational reality, human factors, security, and maintainability. This "anti-single-axis" approach means features are judged by their worst-case behavior, not best-case elegance.

**Key principles:**
- Power must be visible (explicit capabilities and effects)
- Failure story before success story (all features define error behavior)
- Inference reduces syntax, never meaning (humans can read code)
- Governed, not forbidden (dangerous operations are tracked, not eliminated)

See [`DESIGN_DECISIONS.md`](DESIGN_DECISIONS.md) for complete design philosophy and technical decisions.

---

## Phase 1: Foundation (Complete ✅)

**Timeline:** Months 0-1 (Nov 2025 - Dec 2025)  
**Status:** COMPLETE

### Issue 001: Parser + AST ✅
- Full lexer with all token types
- Expression parser with correct precedence
- Declaration parser (let bindings)
- Clean AST representation
- 13+ parser tests

### Issue 002: Effects & Profiles ✅
- Effect enum: Net, FS, Time, Rand, Fail
- EffectRow as canonical set
- Profile enum: Kernel, Realtime, General
- Set operations for effects

### Issue 003: Type Inference Scaffolding ✅
- Inference types (Ty) with type variables
- Full unification algorithm with occurs check
- Substitution with composition
- Type error reporting
- 11 unification tests

### Issue 004: Basic Type Checking ✅
- Expression type inference
- Type environment for let bindings
- Variable lookup and scoping
- Type annotations with verification
- 34 type checker tests

**Phase 1 Result:** Solid foundation with parser, AST, basic type system, and effect types.

---

## Phase 2: Type System Foundations (Complete ✅)

**Timeline:** Months 1-4 (Jan 2026 - Apr 2026)
**Status:** COMPLETE (Issues 005-007)

### Issue 005: Functions & Inference ✅
**Status:** COMPLETE (Feb 1, 2026)

**What was built:**
- Constraint-based type inference
- Function definitions with multi-parameter arrows
- Function calls with type checking
- Generalization/instantiation (let-polymorphism)
- Higher-order functions
- Two-pass module checking (forward references, mutual recursion)

**Example working code:**
```strata
fn add(x: Int, y: Int) -> Int { x + y }
fn double(n: Int) -> Int { add(n, n) }

// Polymorphic identity
fn identity(x) { x }
let a = identity(42);    // Int
let b = identity(true);  // Bool
```

### Issue 005-b: Soundness Hardening ✅
**Status:** COMPLETE (Feb 1, 2026)

**What was fixed:**
1. Unknown identifiers now error properly
2. Real unification errors (no placeholders)
3. Correct generalization boundaries
4. Numeric constraints on arithmetic
5. Two-pass module checking
6. Minimal constraint provenance
7. Determinism audit

**Test coverage:** 49 tests after soundness review

### Issue 006: Blocks & Control Flow ✅
**Status:** COMPLETE (Feb 2, 2026)

**What was built:**
- Block expressions with tail/semicolon semantics
- If/else expressions (branches must unify)
- While loops with proper control flow
- Return statements with propagation
- Mutable let bindings (`let mut x = ...`)
- Assignment statements respecting mutability
- Scope stack evaluator with lexical scoping
- Ty::Never (bottom type) for diverging expressions

**Example working code:**
```strata
fn max(a: Int, b: Int) -> Int {
    if a > b { a } else { b }
}

fn factorial(n: Int) -> Int {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

fn sum_to(n: Int) -> Int {
    let mut total = 0;
    let mut i = 1;
    while i <= n {
        total = total + i;
        i = i + 1;
    };
    total
}
```

### Issue 006-Hardening: Security & Soundness ✅
**Status:** COMPLETE (Feb 2-3, 2026)

**DoS Protection:**
| Limit | Value | Purpose |
|-------|-------|---------|
| Source size | 1 MB | Prevent memory exhaustion |
| Token count | 200,000 | Bound lexer work |
| Parser nesting | 512 | Prevent stack overflow |
| Inference depth | 512 | Bound type inference recursion |
| Eval call depth | 1,000 | Prevent runaway recursion |

**Soundness Fixes:**
- `Ty::Never` only unifies with itself (not a wildcard)
- Divergence handled correctly in if/else and block inference
- Removed panic!/expect() from type checker
- Token limit latching
- Scope guard for guaranteed pop_scope()

**Parser Improvements:**
- `::` qualified type paths
- Span-end fixes
- Universal lexer error surfacing
- Nesting guards for if/while/else-if

**Test stats:** 163 tests passing (30 new hardening tests)

### Issue 007: ADTs & Pattern Matching ✅
**Status:** COMPLETE (Feb 4, 2026)

**What was built:**
- Struct definitions with named fields
- Enum definitions with unit + tuple variants
- Generic type parameters on ADTs (`Option<T>`, `Result<T, E>`)
- Match expressions with exhaustiveness checking (Maranget algorithm)
- Nested patterns, wildcards, literals, variable binders
- Unreachable arm detection (warnings)
- Irrefutable destructuring let (`let (a, b) = pair;`)
- Capability-in-ADT ban (safety infrastructure)

**Example working code:**
```strata
struct Point { x: Int, y: Int }

enum Option<T> {
    Some(T),
    None,
}

fn unwrap_or(opt: Option<Int>, default: Int) -> Int {
    match opt {
        Option::Some(x) => x,
        Option::None => default,
    }
}
```

**Test stats:** 292 tests after Issue 007

**Phase 2 Result:** Complete type system with HM inference, polymorphism, ADTs, and pattern matching.

---

## Phase 3: Effect System (Your Differentiator)

**Timeline:** Months 4-7 (Feb 2026 - May 2026)
**Focus:** Make effects first-class, checked, and enforceable
**Status:** Issues 008-009 COMPLETE, Issue 010 is THE CRITICAL PATH

### Issue 008: Effect System ✅
**Status:** COMPLETE (Feb 5, 2026)

**What was built:**
- Bitmask-based EffectRow with 5 effects: Fs, Net, Time, Rand, Ai
- Effect annotations on functions: `fn f() -> T & {Fs, Net}`
- Extern function effect declarations (required, not optional)
- Two-phase constraint solver (type equality then effect subset)
- Effect variable generalization and instantiation in schemes
- Cycle detection and canonical variable resolution
- Comprehensive error messages for effect mismatches

**Example working code:**
```strata
// Pure function
fn add(x: Int, y: Int) -> Int & {} {
    x + y
}

// Effectful functions
extern fn read_file(path: String) -> String & {Fs};
extern fn fetch(url: String) -> String & {Net};

fn download_and_save(url: String, path: String) -> () & {Fs, Net} {
    let data = fetch(url);
    read_file(path)
}
```

**Test stats:** 354 tests (all passing)

**External validation:**
> "You are solving the 'Laundering' problem correctly. 90% of effect systems fail because they allow effects to be laundered through higher-order functions or ADTs. Your test `adversarial_generic_adt_preserves_effects_through_type_param` proves your unifier is sophisticated. Many junior language designers miss this. You didn't."

### Issue 009: Capability Types ✅
**Status:** COMPLETE (Feb 6-7, 2026)
**Phase:** 3 (Effect System)
**Duration:** 1 week
**Tests:** 398 (44 new, 11% increase)

**What was built:**
- Five capability types: `FsCap`, `NetCap`, `TimeCap`, `RandCap`, `AiCap`
- First-class `Ty::Cap(CapKind)` in the type system (leaf type, not ADT)
- Mandatory `validate_caps_against_effects()` for all functions and extern functions
- Name shadowing protection (reserved capability type names)
- Ergonomic error hints (FsCap/Fs confusion detection)

**Critical fix: Zero-cap skip eliminated.**
The initial implementation had a conditional guard that silently skipped capability validation for functions with zero capability parameters. Three independent external reviewers flagged this as critical. The fix made validation mandatory — no conditional bypass, no exceptions.

**External validation (3 independent sources):**
- Security Review: 7/10 (up from 5/10) — "Core bypass patched. Zero-cap functions/externs with concrete effects now error as MissingCapability/ExternMissingCapability, enforcing 'no ambient' soundly."
- PL Researcher: "Green Light — Strata now possesses a mathematically sound capability-security model."
- Technical Assessment: 0.87-0.90 (up from 0.80-0.85) — "You removed the single biggest security/DX footgun (skip-on-zero-caps). That wasn't a minor bug; it was a 'silent bypass' class."

**Known gaps:**
- Open effect tail polymorphism at instantiation sites may need additional checking
- No warnings for unused capabilities
- Missing tests for: polymorphic HOF + partial concrete effects, mutual recursion SCC, deep recursion chains
- Capability provisioning (how `main` gets capabilities) deferred to Issue 011

**Example working code:**
```strata
extern fn read_file(path: String, fs: FsCap) -> String & {Fs};
extern fn fetch(url: String, net: NetCap) -> String & {Net};

fn load_config(fs: FsCap, path: String) -> String & {Fs} {
    read_file(path, fs)
}

fn download_and_save(fs: FsCap, net: NetCap, url: String, path: String) -> () & {Fs, Net} {
    let data = fetch(url, net);
    write_file(path, data, fs)
}
```

### Issue 010: Affine Types (Linear Capabilities) — "The Final Boss"
**Status:** Ready after Issue 009 (2-3 weeks)
**Phase:** 3 (Effect System)
**Depends on:** Issue 009
**Blocks:** Issue 011

**External validation:**
> "This is the 'Final Boss' of your type system. If you can implement Affine Types without making the language feel like Rust, you will have achieved something truly unique."

**Goal:**
Prevent capability duplication and leaking. Capabilities are *affine* — use at most once.

**Core Concept:**
Types have kinds:
- `*` (Unrestricted): Int, String, Bool — can be used any number of times
- `!` (Affine): FsCap, NetCap, etc. — can be used at most once

**Semantics:**
```strata
fn example(fs: FsCap) -> () & {Fs} {
    use_cap(fs);   // fs moved here
    use_cap(fs);   // ERROR: use of moved value `fs`
}

// Dropping is allowed (affine, not linear)
fn drop_ok(fs: FsCap) -> () & {} {
    ()  // fs unused, that's fine
}

// Return transfers ownership
fn passthrough(fs: FsCap) -> FsCap & {} {
    fs
}
```

**Restrictions (v0.1):**
- Affine values cannot be captured by closures
- Affine values cannot be used in loops
- All branches must agree on moved state

**Why this is critical:**
This enables Demo 2 (Meta-Agent Orchestration with Affine Types), which external validation calls "the Holy Grail of 2026 AI Safety."

**Phase 3 Result:** Complete capability security model with affine types. This is the foundation for everything else.

---

## Phase 4: Minimal Viable Runtime

**Timeline:** Months 7-10 (May 2026 - Aug 2026)  
**Focus:** Make Strata actually run programs with real I/O

### 4.1: Standard Library (I/O Basics)
**Estimated time:** 2-3 weeks  
**Scope:**
- **File I/O:** `read_file`, `write_file`, `exists`, `list_dir`
- **HTTP client:** `http_get`, `http_post` (basic)
- **Time operations:** `now()`, `sleep()`, `log()`
- **AI model APIs:** `ai_generate()`, `ai_analyze()` (wrappers for OpenAI/Anthropic)
- **Utilities:** `parse_json()`, `hash()`, `len()`, `format()`

**Critical note from external validation:**

**Library Gravity Problem:**
> "People use Python for `boto3` and `pandas`. To succeed, Strata needs:
> 1. **Seamless FFI** via WASM Component Model (call existing WASM components)
> 2. **DevOps Standard Library** (HTTP, JSON, YAML, Cloud APIs - all capability-gated)"

**Priority elevated:** DevOps stdlib and FFI are now **v0.1 critical**, not v0.2 nice-to-have.

**Specific stdlib requirements:**
- HTTP client (comprehensive, not just basic)
- JSON parsing/serialization
- YAML parsing
- Cloud provider API wrappers (AWS, GCP, Azure) - at least HTTP wrappers
- All operations capability-gated

**FFI Strategy:**
- WASM Component Model integration for calling existing WASM components
- Don't force users to rewrite everything
- Leverage existing ecosystem
- Timeline: Build during stdlib phase, not after

### 4.2: Effect Tracing (Runtime Hook)
**Estimated time:** 1-2 weeks  
**Scope:**
- Hook into effectful operations
- Capture inputs/outputs as JSON
- Emit trace to file or stdout
- CLI: `--trace` flag

**Design:**
```bash
# Default behavior: tracing enabled
strata run deploy.strata --trace trace.json

# Optional: Disable tracing for performance
strata run deploy.strata --no-trace

# Replay always works on captured traces
strata replay trace.json
```

**Implementation approach:**
- Tracing enabled by default (audit trails are Strata's differentiator)
- `--no-trace` flag disables trace generation for performance
- Conditional execution: trace code only runs when enabled
- Future optimization: compile-time elimination when provably not needed

**Example trace output:**
```json
{
  "program": "deploy.strata",
  "start_time": "2026-08-15T10:00:00Z",
  "effects": [
    {"timestamp": "...", "effect": "Fs", "operation": "read", "path": "/model.pkl", "bytes": 1048576},
    {"timestamp": "...", "effect": "Net", "operation": "post", "url": "https://...", "status": 200},
    ...
  ],
  "result": {"ok": "deploy-123"},
  "duration_ms": 1234
}
```

### 4.3: Replay Runner
**Estimated time:** 1 week  
**Scope:**
- Read effect trace JSON
- Mock effectful operations with trace values
- Re-execute program deterministically
- CLI: `strata replay trace.json`

**Example usage:**
```bash
# Original run (fails)
strata run deploy.strata --trace failed.json
# Error: Connection timeout at line 45

# Replay exact failure
strata replay failed.json
# Reproduces same failure deterministically
```

**Critical for Demo 3:** Time-Travel Bug Hunting

### 4.4: WASM Compilation (Basic)
**Estimated time:** 2-3 weeks  
**Scope:**
- Lower AST → WASM instructions (emit WAT, compile via wat2wasm)
- Basic function calls
- Arithmetic and logic operations
- Memory management (arena-per-function)
- FFI stubs for effects

**Critical note from external validation:**

**Main Definition:**
> "You need to formalize how a Strata program starts. Confidence-building design: `fn main(fs: FsCap, net: NetCap) { ... }` where the runtime (WASM host) injects these capabilities based on a manifest. This closes the loop on 'No Ambient Authority.'"

**Implementation:**
- Define `fn main()` signature with explicit capability parameters
- Runtime injects capabilities based on manifest file
- Manifest specifies which capabilities the program gets
- No ambient authority — all caps come from main

**Deferred to v0.2:**
- Optimization passes
- Advanced WASM features
- Custom VM with bytecode

**Phase 4 Result:** Working killer demos with effect traces and deterministic replay. FFI via WASM Component Model enables calling existing ecosystem.

---

## Phase 5: Hardening & Launch

**Timeline:** Months 10-14 (Aug 2026 - Dec 2026)  
**Focus:** Make v0.1 production-ready and ship it

### 5.1: Developer Experience
**Estimated time:** 2-3 weeks  
**Scope:**
- Improved error messages with suggestions
- Warnings for common mistakes
- Help text and documentation strings
- Stack traces with source spans

**Critical for affine types:** Error messages must be exceptionally clear for move semantics, since this is unfamiliar to target audience (ops engineers, not Rust developers).

### 5.2: Tooling Basics
**Estimated time:** 2-3 weeks  
**Scope:**
- LSP server (basic completion, go-to-definition)
- Formatter (canonical style)
- Test runner (`strata test`)

**Deferred to v0.2:**
- Debugger
- REPL
- Package manager
- Documentation generator

### 5.3: Documentation
**Estimated time:** 2-3 weeks  
**Scope:**
- Tutorial (from first program to killer demo)
- Effect system guide
- Capability security explained (for non-PL people)
- Affine types explained (without referencing Rust)
- Standard library reference
- Examples repository

**Critical:** Ops engineers are the audience, not PL researchers. Avoid academic jargon.

### 5.4: Examples & Demos
**Estimated time:** 2-3 weeks  
**Scope:**
- Polish the three killer demos (deployments, agents, replay)
- Add 10-15 example programs
- Blog posts explaining key features
- Video walkthrough of demos

### 5.5: Testing & CI
**Estimated time:** 1-2 weeks  
**Scope:**
- Test coverage >90%
- CI pipeline (GitHub Actions)
- Benchmark suite
- Integration tests

### 5.6: Release
**Estimated time:** 1 week  
**Scope:**
- Binary releases (Linux, macOS, Windows)
- Installation script
- Release notes
- Launch blog post
- HN/Reddit announcements

**Phase 5 Result:** Production-ready v0.1 with great DX, docs, and tooling.

---

## Explicitly Deferred to v0.2+

These features are important but not required for v0.1:

### Deferred Language Features
- **Profile Enforcement** (`@profile(Kernel)` annotations)
- **Effect Handlers** (algebraic effects with continuations)
- **Capability Attenuation** (Issue 010.5 - for Demo 5: Blast Radius Controller)
  - Type operations: `Cap<A+B>` → `Cap<A>`
  - Estimated: 5-7 days
  - Enables dynamic authority slicing
- **DSU Solver Rewrite** (Union-Find + worklist for scale)
- **Row polymorphism** (effect variables: `fn f<E>() -> T & E`)
- **Actors & supervision** (concurrency model)
- **Async/await syntax** (concurrency primitives)
- **Ownership/borrow checking** (memory safety beyond arenas)
- **Advanced traits** (associated types, defaults, etc.)
- **Custom VM with bytecode** (performance optimization)
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

## Timeline Summary

**Phase 1 (Complete):** Foundation - Parser, AST, Effects, Basic Types  
**Months 0-1** ✅

**Phase 2 (Complete):** Type System - Functions, Inference, ADTs, Patterns
**Months 1-4** ✅

**Phase 3 (In Progress - CRITICAL):** Effect System - Effect checking, Capabilities, Affine Types
**Months 4-7** (Issues 008-009 ✅, Issue 010 next — THE CRITICAL PATH)

**Phase 4:** Runtime - Stdlib + FFI, Tracing, Replay, WASM  
**Months 7-10**

**Phase 5:** Hardening & Launch - DX, Tooling, Docs, Release  
**Months 10-14**

**Total: 10-14 months from project start to v0.1 launch**

**Critical 3-Month Window (Feb-May 2026):**
- Month 5 (Feb): Issue 009 (Capabilities - Kill the Escape Hatch) ✅ DONE
- Month 6 (Mar): Issue 010 (Affine Types - The Final Boss) ← NEXT
- Month 7 (Apr-May): Issue 011 (Runtime + Demos - Prove the Value)

**If Issues 009-011 succeed: Technical confidence → 90%+**

---

## Current Status (v0.0.9)

**Completed:**
- ✅ Parser & AST (Issue 001)
- ✅ Effects & Profiles (Issue 002)
- ✅ Type Inference Scaffolding (Issue 003)
- ✅ Basic Type Checking (Issue 004)
- ✅ Functions & Inference (Issue 005)
- ✅ Soundness Hardening (Issue 005-b)
- ✅ Blocks & Control Flow (Issue 006)
- ✅ Security Hardening (Issue 006-Hardening)
- ✅ ADTs & Pattern Matching (Issue 007)
- ✅ Effect System (Issue 008)
- ✅ Capability Types (Issue 009) ← NEW

**Next Up (The Critical Path):**
- Issue 010: Affine Types — "The Final Boss" (2-3 weeks)
- Issue 011: WASM Runtime + Effect Traces (3-4 weeks)

**Project Stats:**
- 398 tests (all passing)
- 4 crates (ast, parse, types, cli)
- ~9,000+ lines of Rust
- 0 clippy warnings (enforced)

**External validation (Feb 2026, 3 independent sources):**
- Security Review: 7/10 — "Core bypass patched, no ambient authority enforced"
- PL Researcher: "Green Light — mathematically sound capability-security model"
- Technical Assessment: 0.87-0.90 — "Single biggest security/DX footgun removed"
- Warning: "Will live or die by rigor" — no compromises

---

## Success Metrics

### Technical Milestones
- ✅ Parser working
- ✅ Basic type checking
- ✅ HM inference with polymorphism
- ✅ Control flow (if/while/return)
- ✅ ADTs and pattern matching
- ✅ Effect system complete
- ✅ Capability types (Issue 009) — externally validated
- ⏳ Affine types / linearity (Issue 010 - critical, NEXT)
- ⏳ Working killer demos (Issue 011)

### Quality Metrics
- Test coverage >90% (currently ~85%)
- Zero clippy warnings ✅
- Deterministic behavior ✅
- Clear error messages (improving)

### Launch Metrics (v0.1)

**Good success:**
- 10+ GitHub stars/week after launch
- Blog posts/discussions in Rust/Haskell/ops communities
- 1-2 conference talk submissions accepted
- At least one "I rewrote my deploy scripts in Strata" testimonial

**Great success:**
- 50+ GitHub stars/week
- Trending on HN front page
- Multiple blog posts from users
- Early adopter companies identified

**Knock-it-out success:**
- 100+ stars/week
- Multiple conference acceptances
- Companies asking "when can we use this in prod?"
- Partnership discussions with workflow platforms

---

## Related Documents

- [`IMPLEMENTED.md`](IMPLEMENTED.md) - Detailed feature status
- [`IN_PROGRESS.md`](IN_PROGRESS.md) - Current work and next steps
- [`DESIGN_DECISIONS.md`](DESIGN_DECISIONS.md) - Technical and architectural choices
- [`KILLER_DEMOS.md`](KILLER_DEMOS.md) - v0.1 demo specifications (3 core + 3 expansion)
- [`issues/`](../issues/) - Detailed issue specifications

---

**Last Updated:** February 7, 2026
**Current Version:** v0.0.9
**Next Review:** After Issue 010 completion
