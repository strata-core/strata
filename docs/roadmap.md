# Strata Roadmap

**Last Updated:** February 3, 2026  
**Current Version:** v0.0.7.0 (Issue 007 design review)  
**Target v0.1:** November 2026 - February 2027 (10-14 months)

This document describes the planned roadmap for Strata, organized by development phases. It reflects strategic decisions made in January 2026 to focus on a **minimal, shippable v0.1** that proves core value propositions.

---

## Quick Navigation

- [Strategic Focus](#strategic-focus-for-v01)
- [Phase 2: Type System](#phase-2-type-system-foundations-current) (Current)
- [Phase 3: Effect System](#phase-3-effect-system-your-differentiator)
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
The evaluator (`eval.rs`) is a temporary smoke-test tool for validating type checker output. It has known limitations (no closures, no real I/O, limited error handling). This is intentional - it will be replaced by WASM compilation in Phase 4.

### Performance Philosophy
Performance optimization is explicitly **deferred to v0.2+**. v0.1 prioritizes correctness, safety, and usability over speed.

**Why performance is not a v0.1 concern:**
- Target users (automation engineers) care about safety and audit trails, not microsecond optimizations
- Automation scripts are I/O-bound (network, disk, AI APIs), not CPU-bound
- Variable lookup overhead is noise compared to HTTP request latency
- Premature optimization would delay shipping and add complexity

**Exception:** Effect tracing is designed to be toggleable (see Phase 4.2) to avoid overhead when not needed, but tracing is **enabled by default** since audit trails are Strata's core value proposition.

**When to optimize:** After v0.1 ships, after profiling real-world usage, if users report performance issues.

---

## Phase 2: Type System Foundations (Current)

**Timeline:** Months 1-4 (Jan 2026 - Apr 2026)  
**Focus:** Get the type system solid before tackling effects

### Issue 005: Functions & Inference ✅ COMPLETE
**Completed:** February 1, 2026

**What was built:**
- Constraint-based type inference (Algorithm W style)
- Function definitions with type signatures
- Function calls with type checking
- Generalization/instantiation (let-polymorphism)
- Higher-order functions
- Two-pass module checking (forward references, mutual recursion)
- Soundness hardening (Issue 005-b): 7 critical fixes

**Test coverage:** 49 tests

### Issue 006: Blocks & Control Flow ✅ COMPLETE
**Completed:** February 2, 2026

**What was built:**
- Block expressions: `{ stmt; stmt; expr }`
- If/else expressions (branches must unify)
- While loops
- Return statements
- Mutable let bindings (simple, no borrow checking)
- `Ty::Never` with correct bottom semantics
- Closures with captured environments
- Mutual recursion support

**Security Hardening (006-Hardening):**
- DoS protection: nesting (512), call depth (1000), tokens (200K), inference (512), source (1MB)
- Soundness: Never type only unifies with itself
- Removed panic!/expect() from checker

**Test coverage:** 163 tests (84 new + 30 hardening)

### Issue 007: ADTs & Pattern Matching ← CURRENT
**Status:** Design review in progress  
**Estimated time:** 10-14 days  
**Scope:**
- Struct definitions
- Enum definitions with variants
- Generic type parameters (e.g., `Option<T>`, `Result<T, E>`)
- Match expressions with exhaustiveness checking
- Pattern matching (literals, variables, constructors)

**Key decisions under review:**
- Nested patterns (rec: yes)
- Match guards (rec: defer)
- Tuple types (rec: defer)
- Variant syntax (rec: all three — unit, tuple, struct)
- Struct update syntax (rec: defer)
- Visibility modifiers (rec: defer)

**Soundness concerns (from Gemini):**
1. Constructor variance — constructors as polymorphic functions
2. Pattern scoping — mutability tracking per-arm
3. Exhaustiveness + Never — match type = join of non-Never arms
4. Redundant patterns — dead arms as security risk

**Deferred to v0.2:**
- Associated types in traits
- Default methods
- Trait bounds in where clauses (beyond simple bounds)

---

## Phase 3: Effect System (Your Differentiator)

**Timeline:** Months 5-7 (May 2026 - Jul 2026)  
**Focus:** Effects and capabilities - this is what makes Strata unique

### Issue 008: Effect Syntax & Checking
**Status:** Planned  
**Estimated time:** 2-3 weeks  

**Note on Elaboration:**  
Issues 005 (constraint solving) and 008 (effect checking) together form the elaboration phase: converting inferred types (Ty with variables) to surface types (Type with effect rows).

**Scope:**
- Parse `& {Effect1, Effect2}` syntax on function types
- Parse `using cap: CapType` parameters
- Effect row representation (sets, not lists)
- Effect inference (extend constraint system)
- Effect subtyping: callee ⊆ caller
- Effect checking pass

**Example:**
```strata
fn deploy(path: String, using fs: FsCap, using net: NetCap) 
    -> Result<Id, Error> & {FS, Net}
{
    let data = read_file(path, using fs)?;
    upload(data, using net)
}
```

**Key decision:** Effect rows are concrete sets in v0.1
- No row polymorphism (e.g., `& E` where E is a variable)
- Design syntax to allow this later, but don't implement inference
- Deferred to v0.2

### Issue 009: Capability Types
**Status:** Planned  
**Estimated time:** 1 week  
**Scope:**
- Standard capability types: `NetCap`, `FsCap`, `TimeCap`, `RandCap`, `AiCap`
- Compiler enforces: to use effect, must have capability in scope
- Error when effect used without corresponding capability
- Capabilities are values (can be passed around)

**Key insight:** Capabilities gate access to effects:
```strata
// ❌ Compile error: Net effect used without NetCap
fn bad() -> String & {Net} {
    http_get("https://...")  // Error: No NetCap in scope
}

// ✅ OK: NetCap explicitly provided
fn good(using net: NetCap) -> String & {Net} {
    http_get("https://...", using net)
}
```

### Issue 010: Profile Enforcement
**Status:** Planned  
**Estimated time:** 1 week  
**Scope:**
- Parse `@profile(Kernel|Realtime|General)` annotations
- Profile checking pass:
  - **Kernel**: No Net, FS, Time, Rand; no dynamic allocation
  - **Realtime**: Bounded latency, backpressure, limited effects
  - **General**: Full language (default)
- Module-level profile declarations
- Compiler errors for profile violations

**Example:**
```strata
@profile(Kernel)
fn embedded_controller(sensor: u32) -> u32 {
    // ✅ OK: Pure computation only
    sensor * 2 + 1
}

@profile(Kernel)
fn bad_kernel(using net: NetCap) -> u32 & {Net} {
    // ❌ Compile error: Kernel profile forbids Net effect
    ...
}
```

---

## Phase 4: Minimal Viable Runtime

**Timeline:** Months 8-10 (Aug 2026 - Oct 2026)  
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

### 4.3: Replay Runner
**Estimated time:** 1 week  
**Scope:**
- Read effect trace JSON
- Mock effectful operations with trace values
- Re-execute program deterministically
- Verify same result
- CLI: `strata replay trace.json`

**Key insight:** This is MUCH simpler than a full VM:
- No bytecode, just replay with mocked effects
- No JIT, just interpreter or compiled code with mocked I/O
- Proves the replay concept without full VM complexity

### 4.4: WASM Compilation Target (Basic)
**Estimated time:** 2-3 weeks  
**Scope:**
- Compile Strata → WASM (via wasm-encoder or LLVM)
- Basic runtime support (no WASI yet, just compute)
- Effect boundaries map to imports/exports
- Focus on getting compiled code to work

**Deferred to v0.2:**
- Full WASI support
- WASM Component Model integration
- Edge runtime optimization
- Browser support (nice-to-have, not focus)

**Rationale:** WASM is a better first runtime target than a custom VM because:
- Deterministic by design
- Sandboxed (aligns with capability model)
- Runs everywhere (server, edge, eventually browser)
- Simpler than building a VM from scratch

---

## Phase 5: Hardening & Launch

**Timeline:** Months 11-14 (Nov 2026 - Feb 2027)  
**Focus:** Polish, docs, examples, and v0.1 release

### 5.1: Developer Experience
- Error messages polish (clear, helpful, with suggestions)
- Span tracking improvements (accurate source locations)
- Better type error explanations
- Warning system (unused variables, dead code, etc.)

### 5.2: Tooling
- LSP basics (syntax highlighting, go-to-definition)
- Code formatter (`strata fmt`)
- Test runner (`strata test`)
- Documentation generator

### 5.3: Documentation
- Language guide (tutorial)
- Effect system explained
- Capability model guide
- Standard library docs
- Migration guide (from Python/Bash)
- Architecture docs (for contributors)

### 5.4: Examples & Demos
- Polish the two killer demos
- Add 10-15 example programs
- Blog posts explaining key features
- Video walkthrough of demos

### 5.5: Testing & CI
- Comprehensive test suite (>90% coverage)
- CI pipeline (GitHub Actions)
- Property tests for type system
- Integration tests for standard library
- Determinism tests (replay validation)

### 5.6: v0.1 Release
- Release notes
- Announcement blog post
- Hacker News / Reddit / Lobsters posts
- Initial documentation site
- GitHub releases with binaries

---

## Explicitly Deferred to v0.2+

These features are out of scope for v0.1 to keep the timeline realistic:

### Row Polymorphism
**Why deferred:** Adds significant complexity to inference  
**When:** v0.2 (after v0.1 proves core value)  
**Example of what we're missing:**
```strata
// v0.1: Must specify concrete effects
fn map<T, U>(f: T -> U, opt: Option<T>) -> Option<U> & {FS, Net}

// v0.2: Can be polymorphic over effects
fn map<T, U, E>(f: T -> U & E, opt: Option<T>) -> Option<U> & E
```

### Actors & Supervision
**Why deferred:** Complex runtime, less critical for v0.1 demos  
**When:** v0.2 (after basic runtime works)  
**Design note:** Effect system is designed to accommodate actors later:
```strata
// v0.2 preview:
actor Worker {
    fn handle(msg: Job, using net: NetCap) -> Result & {Net} {
        ...
    }
}
```

### Async/Await Syntax
**Why deferred:** Runtime complexity, unclear design space  
**When:** v0.2+ (after actor experience informs design)  
**Design note:** Effect types are async-ready, but no syntax commitment yet

### Ownership & Borrow Checking
**Why deferred:** High complexity, arena model sufficient for v0.1  
**When:** v0.3+ (if demand exists)  
**Current approach:** Arena-per-function with explicit `drop`

### Advanced Trait Features
**Why deferred:** Complex to implement, not needed for v0.1  
**When:** v0.2  
**Includes:**
- Associated types
- Default methods
- Higher-kinded types
- Trait aliases

### Logic Programming (Datalog)
**Why deferred:** Separate feature, not needed for automation demos  
**When:** v0.2-v0.3  
**Design note:** Will integrate as library, not compiler phase

### Self-Hosting
**Why deferred:** Language needs to be stable first  
**When:** v0.3+ (18-24 months post-v0.1)  
**Possible earlier:** Self-host the type checker in v0.2 as proof-of-concept

### Full VM with Bytecode
**Why deferred:** WASM is better first runtime target  
**When:** Post-v0.1 (if demand exists for non-WASM runtime)  
**Alternative:** May not need custom VM if WASM works well

---

## Timeline Summary

| Phase | Duration | Completion Date | Key Deliverables | Status |
|-------|----------|-----------------|------------------|--------|
| Phase 2 (Type System) | 3-4 months | Apr 2026 | Issues 005-007 complete | 🔄 In Progress |
| Phase 3 (Effects) | 2-3 months | Jul 2026 | Issues 008-010 complete | Planned |
| Phase 4 (Runtime) | 2-3 months | Oct 2026 | Demos working, basic WASM | Planned |
| Phase 5 (Hardening) | 3-4 months | Dec 2026 - Feb 2027 | v0.1 release | Planned |
| **Total** | **10-14 months** | **Feb 2027** | **Strata v0.1** | |

**Confidence level:** Medium-high (based on actual Issue 005-006 velocity)

**Velocity check (Feb 2026):**
- Issues 001-006 + hardening completed in ~2 months
- On track for Phase 2 completion by Apr 2026
- 163 tests, solid foundation

**Risk factors:**
- ADT/pattern matching complexity (Issue 007 is substantial)
- Effect system interactions (Issues 008-010 may reveal edge cases)
- WASM compilation challenges (Phase 4 is less predictable)

**Mitigation:** Timebox each issue. If something takes >2x estimated time, cut scope rather than delay.

---

## Post-v0.1 Vision (v0.2-v0.5)

**v0.2 (6-9 months post-v0.1):**
- Row polymorphism
- Actors & supervision
- Advanced trait features
- Improved tooling (better LSP, debugger)
- Expanded standard library

**v0.3 (12-18 months post-v0.1):**
- Logic programming (Datalog integration)
- Self-hosting experiments
- Async/await design
- Performance optimizations

**v0.4-v0.5 (18-30 months post-v0.1):**
- Production hardening
- Ownership/borrow checking (if needed)
- Full WASI support
- Ecosystem building (package manager, registry)

---

## Success Metrics for v0.1

**Minimum success:**
- 2-3 early adopters use Strata for production automation
- Killer demos prove value proposition
- No major architectural regrets (can build v0.2 on v0.1 foundation)

**Good success:**
- 10+ GitHub stars/week after launch
- Blog posts/discussions in Rust/Haskell/ops communities
- 1-2 conference talk submissions accepted
- At least one "I rewrote my deploy scripts in Strata" testimonial

**Great success:**
- HN/Reddit front page for a day
- 50+ GitHub stars/week
- Other language designers cite Strata's effect system
- First external contribution merged
- Job posting mentioning Strata

---

## Philosophy Reminders

**"Boring but Clear":**
- Developer understanding over language novelty
- Explicit over implicit
- Safety over convenience
- Clarity over cleverness

**Ship, Don't Perfect:**
- v0.1 is a foundation, not the final form
- Better to ship with 80% of features than 100% never
- Learn from real users, then iterate

**Focus on Value:**
- Every feature must serve the automation engineer use case
- If it doesn't help with the killer demos, defer it
- Prove core value first, expand scope later

---

## Related Documents

- **[KILLER_DEMOS.md](KILLER_DEMOS.md)** - Detailed v0.1 demo scenarios
- **[IN_PROGRESS.md](IN_PROGRESS.md)** - Current work and status
- **[IMPLEMENTED.md](IMPLEMENTED.md)** - What actually works right now
- **[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md)** - Key design choices and rationale
