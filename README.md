# Strata — Layered Safety by Design

> **Status:** v0.0.9 — Capability Types complete
> **License:** MIT
> **Language:** Rust

Strata is a general-purpose, strongly typed programming language designed for **safe automation**, **explainable AI**, and **resilient distributed systems**.

## Core Principles

- **Effect-typed safety** — Every side effect appears in function types
- **Capability security** — No ambient I/O; explicit capabilities only
- **Supervised concurrency** — Actors with structured scopes
- **Profiles** — Kernel / Realtime / General modes
- **Explainability** — Datalog with proof traces
- **Deterministic replay** — Seedable RNG/Time with audit logs
- **Multi-target** — Native AOT, bytecode VM, WASM/WASI

---

## Why Strata?

### The Problem: Automation Safety Gap

Modern infrastructure runs on scripts that can "silently nuke prod." Current options force a choice:

**High ergonomics, low safety:**
- Python/Bash for automation
- Fast to write, dangerous in production
- No type safety, no effect tracking
- Scripts grow into unmaintainable systems

**High safety, high complexity:**
- Rust/Haskell for critical systems
- Strong guarantees, steep learning curve
- Too complex for ops teams under pressure

**The gap:** No language offers both safety AND ergonomics for automation.

### Strata's Approach: Zero-Tax Safety

Strata makes the **secure way the easy way**:

```strata
// Capabilities make authority explicit
fn deploy(
    model: Model,
    using fs: FsCap,
    using net: NetCap
) -> Result<Deployment> & {Fs, Net} {
    validate_model(model, using fs)?;
    upload(model, using net)
}
```

**What you get:**
- **Type safety** without ceremony (Hindley-Milner inference)
- **Effect tracking** without complexity (visible in function types)
- **Capability security** without sandboxes (no ambient authority)
- **Deterministic replay** without instrumentation (built-in tracing)

### Four Problems Strata Solves

**1. Supply Chain Security**

When you import a library in Python/Node.js, it can read your SSH keys, scan your network, and exfiltrate data. Strata's capability security provides **intra-process isolation** - a JSON library with `JsonCap` mathematically cannot perform network I/O.

**2. Audit Trails**

To understand what a script does today, you read every line of code. In Strata, the function signature is a contract:

```strata
fn deploy(cfg: Config) -> Result<o> & {Fs, Net, Env}
```

Security teams can grep for `{Net}` effects in sensitive environments. **Compliance becomes type checking.**

**3. Reproducible Failures**

When automation fails at 3 AM, you can't reproduce it locally. Strata's effect traces capture every I/O operation. Run `strata replay trace.json` and debug the exact failure deterministically.

**4. AI Agent Safety**

AI agents making autonomous decisions need guardrails. Strata's capability security prevents agents from doing unauthorized actions:

```strata
// Agent gets explicit capabilities, can't exceed them
fn ai_agent(
    task: Task,
    using read_only: FsCap,  // Can read files
    using ai: AiCap           // Can call AI models
) -> Result<Analysis> & {Fs, Ai} {
    // Cannot write files or access network
    // Capabilities enforced by type system
}
```

### When to Use Strata

**Use Strata for:**
- ✅ Deployment scripts that touch production
- ✅ AI agent orchestration workflows
- ✅ Automation requiring audit trails (SOC 2, ISO 27001)
- ✅ Scripts that call AI models or APIs
- ✅ Incident response playbooks
- ✅ CI/CD pipelines with sensitive access

**Don't use Strata for:**
- ❌ High-performance computing (use Rust)
- ❌ System programming (use Rust/C++)
- ❌ Quick one-off scripts (use Python/Bash)
- ❌ Applications with millisecond latency requirements

### What Makes Strata Different

**vs Python/Bash:**
- ✅ Type safety catches bugs at compile time
- ✅ Effect tracking makes I/O visible in types
- ✅ Capability security prevents unauthorized access
- ✅ Deterministic replay for debugging

**vs Rust:**
- ✅ No lifetimes, no borrow checker
- ✅ Hindley-Milner inference (feels like Python)
- ✅ Effect types built-in (not "just code")
- ✅ Designed for ops teams, not systems programmers

**vs Workflow Platforms (n8n, Temporal, Airflow):**
- ✅ Type-safe by default
- ✅ Effect-tracked workflows
- ✅ Capability-gated operations
- ✅ Code-first (not visual)

---

## Current Status

✅ **Working:**
- Parser with full expression and control flow support
- Constraint-based type inference system
- Function declarations with multi-parameter arrows
- Polymorphic types (let-polymorphism)
- Function calls with type checking
- Higher-order functions
- Two-pass module checking (forward references, mutual recursion)
- Block expressions with tail/semicolon semantics
- If/else expressions with branch type unification
- While loops with proper control flow
- Return statements propagating through nested blocks
- Mutable bindings with mutability checking
- Structs with named fields (nominal)
- Enums with unit + tuple variants
- Generic type parameters on ADTs (`Option<T>`, `Result<T, E>`)
- Match expressions with exhaustiveness checking (Maranget algorithm)
- Nested patterns, wildcards, literals, variable binders
- Unreachable arm detection (warnings)
- Irrefutable destructuring let (`let (a, b) = pair;`)
- Capability-in-ADT ban (safety infrastructure)
- `Ty::Never` with correct bottom semantics
- Sound type system with real error messages
- Scope-stack evaluator with closures
- CLI with automatic type checking
- **Compile-time effect system** with 5 built-in effects (Fs, Net, Time, Rand, Ai)
- Effect annotations: `fn f() -> Int & {Fs, Net} { ... }`
- Extern function declarations: `extern fn read(path: String, fs: FsCap) -> String & {Fs};`
- Effect inference for unannotated functions
- Higher-order effect propagation
- **Capability types** — `FsCap`, `NetCap`, `TimeCap`, `RandCap`, `AiCap`
- Mandatory capability validation: effect `{X}` requires `XCap` in scope
- No ambient authority — capabilities must be explicitly passed
- Reserved capability name protection (ADTs cannot shadow capability types)
- **402 comprehensive tests** (all passing)

✅ **Security Hardening:**
- DoS protection: source size (1MB), token count (200K), parser nesting (512), inference depth (512), eval call depth (1000), effect vars (4096)
- Soundness: `Ty::Never` only unifies with itself
- Removed panic!/expect() from type checker
- Universal lexer error surfacing
- Parser depth guards balanced on all error paths

📋 **Next Up:**
- Issue 010: Affine types (linear capabilities)
- Issue 011: WASM runtime + effect traces + deterministic replay

**Target v0.1:** November 2026 - February 2027

See [`docs/IN_PROGRESS.md`](docs/IN_PROGRESS.md) for details.

---

## Quick Start

```bash
# Clone and build
git clone https://github.com/strata-core/strata.git
cd strata
cargo build --workspace

# Run an example
cargo run -p strata-cli -- examples/add.strata

# Run tests (402 tests)
cargo test --workspace
```

---

## Example Code

```strata
// Functions & control flow
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

// Mutual recursion
fn is_even(n: Int) -> Bool {
    if n == 0 { true } else { is_odd(n - 1) }
}
fn is_odd(n: Int) -> Bool {
    if n == 0 { false } else { is_even(n - 1) }
}

// Polymorphic identity function
fn identity(x) { x }
let a = identity(42);     // Int
let b = identity(true);   // Bool
```

```strata
// ADTs & pattern matching
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

let pair = (10, 20);
let (a, b) = pair;

struct Point { x: Int, y: Int }
let p = Point { x: a, y: b };
```

```strata
// Effect system - compile-time side effect tracking
extern fn read_file(path: String, fs: FsCap) -> String & {Fs};
extern fn fetch(url: String, net: NetCap) -> String & {Net};

// Capabilities gate effect usage — no FsCap, no {Fs} effects
fn load_config(fs: FsCap, path: String) -> String & {Fs} {
    read_file(path, fs)
}

// Multiple capabilities for multiple effects
fn download_and_save(fs: FsCap, net: NetCap, url: String, path: String) -> () & {Fs, Net} {
    let data = fetch(url, net);
    write_file(path, data, fs)
}

// Pure functions need no capabilities or annotations
fn add(x: Int, y: Int) -> Int { x + y }
```

**Built-in effects:** `Fs` (filesystem), `Net` (network), `Time` (clock), `Rand` (randomness), `Ai` (model calls)
**Capability types:** `FsCap`, `NetCap`, `TimeCap`, `RandCap`, `AiCap` — each gates its corresponding effect

---

## Development

```bash
# Build everything
cargo build --workspace

# Run all tests (402 tests)
cargo test --workspace

# Run clippy (enforced in CI)
cargo clippy --workspace --all-targets -- -D warnings

# Format code
cargo fmt
```

---

## Roadmap

**Phase 1 (Complete):** Parser, AST, basic type checking ✅  
**Phase 2 (Complete):** Functions ✅, hardening ✅, blocks ✅, ADTs ✅  
**Phase 3 (Current):** Effect system ✅, capabilities ✅, affine types
**Phase 4:** Runtime, stdlib, WASM compilation, replay  
**Phase 5:** Tooling, docs, killer demos, v0.1 launch  

**v0.1 Target:** November 2026 - February 2027

See [`docs/roadmap.md`](docs/roadmap.md) for detailed plans.

---

## Philosophy

**"Boring but Clear"** — We prioritize:
- Developer understanding over language novelty
- Explicit over implicit
- Safety over convenience
- Clarity over cleverness
- **Soundness over speed** — A type system that lies is worse than no type system

**"Rigor over Ergonomics"** — If an automation engineer has to type `using fs: FsCap` to delete a file, that explicitness is the feature, not a bug. Strata makes authority visible, not hidden.

---

## Contributing

This is currently a personal project. Once it reaches a more stable state (post-Issue 010), contributions will be welcome.

---

## License

MIT License - see [`LICENSE`](LICENSE) for details.

---

**Taglines:**  
*"Layered safety by design."*  
*"It explains itself."*  
*"The secure way is the easy way."*
