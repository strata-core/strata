# Strata Roadmap

**Last Updated:** February 5, 2026
**Current Version:** v0.0.8
**Target v0.1:** November 2026 - February 2027 (10-14 months from project start)

This document describes the planned roadmap for Strata, organized by development phases. It reflects strategic decisions made in January 2026 to focus on a **minimal, shippable v0.1** that proves core value propositions.

---

## Quick Navigation

- [Strategic Focus](#strategic-focus-for-v01)
- [Phase 2: Type System](#phase-2-type-system-foundations-mostly-complete) (Mostly Complete ✅)
- [Phase 3: Effect System](#phase-3-effect-system-your-differentiator) (Next)
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
1. Safe model deployment script
2. AI-powered incident response workflow

**v0.1 Success Metric:** An automation engineer can write a production deployment script in Strata that is safer, more auditable, and easier to debug than Python/Bash equivalents.

---

## Important Notes

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

### Design Philosophy
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

**Timeline:** Months 4-7 (Apr 2026 - Jul 2026)
**Focus:** Make effects first-class, checked, and enforceable
**Status:** Issue 008 COMPLETE, Issues 009-010 next

### Issue 008: Effect System ✅
**Status:** COMPLETE (Feb 5, 2026)

**What was built:**
- Bitmask-based EffectRow with 5 effects: Fs, Net, Time, Rand, Ai
- Effect annotations on functions: `fn f() -> T & {Fs, Net}`
- Extern function effect declarations (required, not optional)
- Two-phase constraint solver (type equality then effect subset)
- Rémy-style row unification for effect rows
- Effect variable generalization and instantiation in schemes
- Cycle detection and canonical variable resolution in solver
- Higher-order effect propagation

**Example working code:**
```strata
extern fn read_file(path: String) -> String & {Fs};
extern fn fetch(url: String) -> String & {Net};

fn download_and_save(url: String, path: String) -> () & {Fs, Net} {
    let data = fetch(url);
    read_file(path)
}

// Pure functions — no annotation needed
fn add(x: Int, y: Int) -> Int { x + y }
```

**Test stats:** 354 tests after Issue 008

### Issue 009: Capability Types
**Status:** Ready to start
**Estimated time:** 5-7 days

**Scope:**
- Five capability types: FsCap, NetCap, TimeCap, RandCap, AiCap
- Capabilities as function parameters
- Compiler enforces: effect {X} requires XCap in scope
- Extern functions must have capability params for declared effects
- Capabilities cannot be stored in ADTs

**Example:**
```strata
fn read_file(path: String, fs: FsCap) -> String & {Fs} { ... }

fn main(fs: FsCap, net: NetCap) -> () & {Fs, Net} { ... }
```

### Issue 010: Affine Types (Linear Capabilities)
**Status:** Ready after Issue 009
**Estimated time:** 7-10 days

**Scope:**
- Kind system: Unrestricted (`*`) and Affine (`!`)
- Move tracking in checker
- Use-after-move detection
- Affine values cannot be captured by closures or used in loops
- Branch consistency validation

**Example:**
```strata
fn example(fs: FsCap) -> () & {Fs} {
    use_cap(fs);   // fs moved here
    use_cap(fs);   // ERROR: use of moved value `fs`
}
```

**Phase 3 Result:** Effect-typed language with capability security and affine types.

---

## Phase 4: Minimal Viable Runtime

**Timeline:** Months 7-10 (Jul 2026 - Oct 2026)  
**Focus:** Get the killer demos working end-to-end

### 4.1: Standard Library (Minimal)
**Estimated time:** 2-3 weeks  
**Scope:**
- Core types: `Result<T, E>`, `Option<T>`
- Primitives: String, Int, Float, Bool
- Collections: `Vec<T>`, `Map<K, V>`, `Set<T>` (simple implementations)
- File I/O: `read_file`, `write_file` (capability-gated)
- HTTP: `http_get`, `http_post` (capability-gated, wrapper around curl/reqwest)
- Time: `now()`, `log()` (capability-gated)
- AI: `ai_analyze`, `ai_generate_steps` (wrappers around OpenAI/Anthropic APIs)
- Utilities: `hash()`, `len()`, `format()`

**Deferred to v0.2:**
- Advanced collections (trees, graphs)
- JSON parsing (use strings for v0.1)
- Crypto beyond hashing
- Database drivers
- Advanced HTTP (streaming, etc.)

### 4.2: Effect Tracing Runtime
**Estimated time:** 1-2 weeks  
**Scope:**
- Intercept all effectful operations
- Log to structured trace (JSON)
- Trace format: timestamp, effect type, parameters, result
- Seedable RNG and Time (for determinism)
- CLI flag: `--trace output.json`

**Design consideration: Toggleable tracing for performance**

Effect tracing should be optional to avoid overhead in performance-critical scenarios:

```bash
# Default: Tracing enabled (audit trails are the value proposition)
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
    {"timestamp": "...", "effect": "FS.Read", "path": "/model.pkl", "bytes": 1048576},
    {"timestamp": "...", "effect": "Net.Post", "url": "https://...", "status": 200},
    ...
  ],
  "result": "Ok(\"deploy-123\")",
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

### 4.4: WASM Compilation (Basic)
**Estimated time:** 2-3 weeks  
**Scope:**
- Lower AST → WASM instructions
- Basic function calls
- Arithmetic and logic operations
- Memory management (arena-per-function)
- FFI stubs for effects

**Deferred to v0.2:**
- Optimization passes
- Advanced WASM features
- Custom VM with bytecode

**Phase 4 Result:** Working killer demos with effect traces and deterministic replay.

---

## Phase 5: Hardening & Launch

**Timeline:** Months 10-14 (Oct 2026 - Feb 2027)  
**Focus:** Make v0.1 production-ready and ship it

### 5.1: Developer Experience
**Estimated time:** 2-3 weeks  
**Scope:**
- Improved error messages with suggestions
- Warnings for common mistakes
- Help text and documentation strings
- Stack traces with source spans

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
- Standard library reference
- Examples repository

### 5.4: Examples & Demos
**Estimated time:** 2-3 weeks  
**Scope:**
- Polish the two killer demos
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
- JSON/serialization
- Database drivers
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

**Phase 3 (In Progress):** Effect System - Effect checking, Capabilities, Affine Types
**Months 4-7** (Issue 008 ✅, Issues 009-010 next)

**Phase 4:** Runtime - Stdlib, Tracing, Replay, WASM  
**Months 7-10**

**Phase 5:** Hardening & Launch - DX, Tooling, Docs, Release  
**Months 10-14**

**Total: 10-14 months from project start to v0.1 launch**

---

## Current Status (v0.0.8)

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

**Next Up:**
- Issue 009: Capability Types
- Issue 010: Affine Types (Linear Capabilities)
- Issue 011: WASM Runtime + Effect Traces

**Project Stats:**
- 354 tests (all passing)
- 4 crates (ast, parse, types, cli)
- ~7,000+ lines of Rust
- 0 clippy warnings (enforced)

---

## Success Metrics

### Technical Milestones
- ✅ Parser working
- ✅ Basic type checking
- ✅ HM inference with polymorphism
- ✅ Control flow (if/while/return)
- ✅ ADTs and pattern matching
- ✅ Effect system complete
- ⏳ Capability types
- ⏳ Affine types (linearity)
- ⏳ Working killer demos

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
- [`KILLER_DEMOS.md`](KILLER_DEMOS.md) - v0.1 demo specifications
- [`issues/`](../issues/) - Detailed issue specifications

---

**Last Updated:** February 5, 2026
**Current Version:** v0.0.8
**Next Review:** After Issue 011 completion