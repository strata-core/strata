# Strata — Layered Safety by Design

> **Status:** Early development — Issue 007 (ADTs & Pattern Matching) complete
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
- **292 comprehensive tests** (all passing)

✅ **Security Hardening:**
- DoS protection: source size (1MB), token count (200K), parser nesting (512), inference depth (512), eval call depth (1000)
- Soundness: `Ty::Never` only unifies with itself
- Removed panic!/expect() from type checker
- Universal lexer error surfacing
- Parser depth guards balanced on all error paths

📋 **Next Up:**
- Issue 008: Effect system enforcement
- Issue 009-010: Capabilities, profiles

**Target v0.1:** November 2026 - February 2027

See [`docs/IN_PROGRESS.md`](docs/IN_PROGRESS.md) for details.

## Quick Start

```bash
# Clone and build
git clone https://github.com/strata-core/strata.git
cd strata
cargo build --workspace

# Run an example
cargo run -p strata-cli -- examples/add.strata

# Run tests (292 tests)
cargo test --workspace
```

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

## Development

```bash
# Build everything
cargo build --workspace

# Run all tests (292 tests)
cargo test --workspace

# Run clippy (enforced in CI)
cargo clippy --workspace --all-targets -- -D warnings

# Format code
cargo fmt
```

## Roadmap

**Phase 1 (Complete):** Parser, AST, basic type checking ✅
**Phase 2 (Complete):** Functions ✅, hardening ✅, blocks ✅, ADTs ✅
**Phase 3 (Current):** Effect system, capabilities, profiles
**Phase 4:** Runtime, stdlib, WASM compilation, replay
**Phase 5:** Tooling, docs, killer demos, v0.1 launch  

**v0.1 Target:** November 2026 - February 2027

See [`docs/roadmap.md`](docs/roadmap.md) for detailed plans.

## Philosophy

**"Boring but Clear"** — We prioritize:
- Developer understanding over language novelty
- Explicit over implicit
- Safety over convenience
- Clarity over cleverness
- **Soundness over speed** — A type system that lies is worse than no type system

## Contributing

This is currently a personal project. Once it reaches a more stable state (post-Issue 010), contributions will be welcome.

## License

MIT License - see [`LICENSE`](LICENSE) for details.

---

**Taglines:**  
*"Layered safety by design."*  
*"It explains itself."*
