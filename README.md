# Strata — Layered Safety by Design

> **Status:** Early development (v0.0.5) — Issue 005 complete  
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

## Current Status (v0.0.5)

✅ **Working:**
- Parser with full expression support
- Constraint-based type inference system
- Function declarations with multi-parameter arrows
- Polymorphic types (let-polymorphism)
- Function calls with type checking
- Higher-order functions
- Two-pass module checking (forward references)
- Type inference for expressions
- Optional type annotations
- Clear error messages with source spans
- **Sound type system** — no silent failures, real error messages
- CLI with automatic type checking
- 49 comprehensive tests (all passing)

✅ **Recently Completed (Issue 005-b):**
- Unknown identifier error handling
- Real unification error messages (no placeholders)
- Correct generalization boundaries
- Numeric type constraints on arithmetic
- Minimal constraint provenance (span tracking)
- Determinism audit (stable, reproducible behavior)

📋 **Next Up:**
- Issue 006: Blocks & control flow
- Issue 007: ADTs, generics, pattern matching, basic traits
- Issue 008-010: Effect system, capabilities, profiles

**Target v0.1:** November 2026 - February 2027

See [`docs/IN_PROGRESS.md`](docs/IN_PROGRESS.md) for details.

## Quick Start

```bash
# Clone and build
git clone https://github.com/YOUR_USERNAME/strata.git
cd strata
cargo build --workspace

# Run an example
cargo run -p strata-cli -- examples/add.strata

# Run tests (49 tests)
cargo test --workspace
```

## Example Code

```strata
// Current (v0.0.5) - Functions work!
fn add(x: Int, y: Int) -> Int {
    x + y
}

fn double(n: Int) -> Int {
    add(n, n)
}

let result = double(21);  // Type checks: Int

// Polymorphic identity function
fn identity(x) { x }
let a = identity(42);     // Works with Int
let b = identity(true);   // Works with Bool

// Higher-order functions
fn apply(f, x) { f(x) }

// Forward references work!
fn f() -> Int { g() }
fn g() -> Int { 42 }
```

## Development

```bash
# Build everything
cargo build --workspace

# Run all tests (49 tests)
cargo test --workspace

# Run clippy
cargo clippy --workspace

# Format code
cargo fmt
```

## Roadmap

**Phase 1 (Complete):** Parser, AST, basic type checking ✅  
**Phase 2 (Current):** Functions ✅, hardening ✅, blocks (next), ADTs  
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