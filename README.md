# Strata — Layered Safety by Design

> **Status:** Early development (v0.0.6) — Issue 006 complete
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

## Current Status (v0.0.6)

✅ **Working:**
- Parser with full expression and control flow support
- Constraint-based type inference system
- Function declarations with multi-parameter arrows
- Polymorphic types (let-polymorphism)
- Function calls with type checking
- Higher-order functions
- Two-pass module checking (forward references, mutual recursion)
- **Block expressions** with tail/semicolon semantics
- **If/else expressions** with branch type unification
- **While loops** with proper control flow
- **Return statements** propagating through nested blocks
- **Mutable bindings** with mutability checking
- Sound type system with real error messages
- Scope-stack evaluator with closures
- CLI with automatic type checking
- 133 comprehensive tests (all passing)

✅ **Recently Completed (Issue 006):**
- Block expressions: `{ stmt; stmt; expr }`
- If/else and while loops
- Return statements
- Mutable let bindings (`let mut x = ...`)
- Assignment statements with mutability enforcement
- Closures with captured environments
- Self-recursion and mutual recursion

📋 **Next Up:**
- Issue 007: ADTs, generics, pattern matching, basic traits
- Issue 008-010: Effect system, capabilities, profiles

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

# Run tests (133 tests)
cargo test --workspace
```

## Example Code

```strata
// Current (v0.0.6) - Blocks & control flow work!
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

let result = sum_to(10);  // 55

// Mutual recursion works!
fn is_even(n: Int) -> Bool {
    if n == 0 { true } else { is_odd(n - 1) }
}
fn is_odd(n: Int) -> Bool {
    if n == 0 { false } else { is_even(n - 1) }
}

// Polymorphic identity function
fn identity(x) { x }
let a = identity(42);     // Works with Int
let b = identity(true);   // Works with Bool
```

## Development

```bash
# Build everything
cargo build --workspace

# Run all tests (133 tests)
cargo test --workspace

# Run clippy
cargo clippy --workspace

# Format code
cargo fmt
```

## Roadmap

**Phase 1 (Complete):** Parser, AST, basic type checking ✅
**Phase 2 (Current):** Functions ✅, hardening ✅, blocks ✅, ADTs (next)
**Phase 3:** Effect system, capabilities, profiles
**Phase 4:** Runtime, stdlib, WASM compilation, replay
**Phase 5:** Tooling, docs, killer demos, v0.1 launch  

**v0.1 Target:** November 2026 - February 2027

See [`docs/ROADMAP.md`](docs/ROADMAP.md) for detailed plans.

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
