# Issue 006: Blocks & Control Flow

**Status:** ✅ Complete (including hardening)
**Completed:** February 2-3, 2026
**Phase:** 2 (Basic Type System)
**Depends on:** Issue 005

## Goal
Add blocks, if/else, while, and return statements.

## Syntax
```strata
fn max(x: Int, y: Int) -> Int {
    if x > y { x } else { y }
}

fn sum(n: Int) -> Int {
    let mut total = 0;
    let mut i = 0;
    while i < n {
        total = total + i;
        i = i + 1;
    };
    total
}
```

## Scope
- Block expressions: `{ stmt; stmt; expr }`
- If/else (both branches must have same type)
- While loops
- Return statements
- Mutable let bindings (simple, no borrow checking)

## Definition of Done
- [x] AST nodes for Block, If, While, Return
- [x] Parser for control flow syntax
- [x] Type checking: if branches same type, block final expr is type
- [x] Evaluator support for all constructs
- [x] Tests for control flow
- [x] Tests for type errors in branches

## Implementation Notes (Core)
- 133 tests passing after core implementation
- Lexer: 5 new keywords (if, else, while, return, mut)
- Parser: Block/Statement/If/While/Return parsing with proper semicolon rules
- Type inference: CheckContext with env, mutability tracking, expected_return
- Evaluator: Scope stack, closures, ControlFlow enum, mutual recursion support

## Issue 006-Hardening ✅

**Goal:** Security hardening and soundness fixes before Issue 007.

### DoS Protection Limits
| Limit | Value | Purpose |
|-------|-------|---------|
| Source size | 1 MB | Prevent memory exhaustion |
| Token count | 200,000 | Bound lexer work |
| Parser nesting | 512 | Prevent stack overflow in parser |
| Inference depth | 512 | Bound type inference recursion |
| Eval call depth | 1,000 | Prevent runaway recursion at runtime |

### Soundness Fixes
- `Ty::Never` only unifies with itself (not a wildcard)
- Divergence handled correctly in if/else and block inference
- Removed panic!/expect() from type checker
- Token limit latching (can't reset mid-parse)
- Scope guard for guaranteed pop_scope()

### Parser Improvements
- `::` qualified type paths
- Span-end fixes for accurate error locations
- Universal lexer error surfacing
- Nesting guards for if/while/else-if chains

### Test Coverage
- 163 tests total (30 new hardening tests)
- Soundness regression tests for Never type
- DoS limit verification tests

---

# Issue 007: ADTs & Pattern Matching

**Status:** ✅ Complete
**Completed:** February 4, 2026
**Phase:** 2 (Basic Type System)
**Depends on:** Issue 006

## Goal
Add structs, enums, generics, and pattern matching.

## Syntax
```strata
struct Point { x: Int, y: Int }

enum Option<T> {
    Some(T),
    None,
}

fn unwrap_or<T>(opt: Option<T>, default: T) -> T {
    match opt {
        Option::Some(x) => x,
        Option::None => default,
    }
}
```

## What Was Built
- Struct definitions with named fields
- Enum definitions with unit + tuple variants
- Generic type parameters on ADTs
- Match expressions with exhaustiveness checking (Maranget algorithm)
- Nested patterns, wildcards, literals, variable binders
- Unreachable arm detection (warnings)
- Irrefutable destructuring let (`let (a, b) = pair;`)
- Capability-in-ADT ban (safety infrastructure)

**Test coverage:** 292 tests

---

# Issue 008: Effect System

**Status:** ✅ Complete
**Completed:** February 5, 2026
**Phase:** 3 (Effect System)
**Depends on:** Issue 007

## Summary

First-class effect tracking with Rémy-style row polymorphism.

**What was built:**
- Bitmask-based EffectRow with 5 effects: Fs, Net, Time, Rand, Ai
- Effect annotations on functions: `fn f() -> T & {Fs, Net}`
- Extern function effect declarations (required, not optional)
- Two-phase constraint solver (type equality then effect subset)
- Effect variable generalization and instantiation in schemes
- Cycle detection and canonical variable resolution in solver
- Comprehensive error messages for effect mismatches

**Test coverage:** 354 tests

**Key design decisions:**
- ADR-0005 documents effect system architecture
- Effects are closed sets (bitmask), not extensible in v0.1
- Constructors are always pure
- Extern functions must declare effects (no silent pure default)

---

# Issue 009: Capability Types

**Status:** Ready to start
**Estimated time:** 5-7 days
**Phase:** 3 (Effect System)
**Depends on:** Issue 008

## Goal

Introduce capability types that gate effect usage. Enforces "no ambient authority."

## Syntax
```strata
// Capability types (opaque, provided by runtime)
FsCap, NetCap, TimeCap, RandCap, AiCap

// Functions requiring capabilities
fn read_file(path: String, fs: FsCap) -> String & {Fs} { ... }

// Entry point receives capabilities
fn main(fs: FsCap, net: NetCap) -> () & {Fs, Net} { ... }
```

## Scope

- Five capability types corresponding to five effects
- Capabilities as function parameters
- Compiler enforces: effect {X} requires XCap in scope
- Extern functions must have capability params for declared effects
- Capabilities cannot be stored in ADTs

## Definition of Done

- [ ] AST/Parser: Capability type syntax
- [ ] Type system: Ty::Cap(CapKind) variant
- [ ] Checker: Enforce capability-effect correspondence
- [ ] Checker: Extern function capability validation
- [ ] Error messages with help text
- [ ] Tests: positive and negative cases
- [ ] Documentation: ADR for capability design

---

# Issue 010: Affine Types (Linear Capabilities)

**Status:** Ready after Issue 009
**Estimated time:** 7-10 days
**Phase:** 3 (Effect System)
**Depends on:** Issue 009

## Goal

Prevent capability duplication and leaking. Capabilities are *affine* — use at most once.

## Core Concept

Types have kinds:
- `*` (Unrestricted): Int, String, Bool — can be used any number of times
- `!` (Affine): FsCap, NetCap, etc. — can be used at most once

## Semantics
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

## Restrictions (v0.1)

- Affine values cannot be captured by closures
- Affine values cannot be used in loops
- All branches must agree on moved state

## Definition of Done

- [ ] Kind system: Unrestricted and Affine
- [ ] Move tracking in checker
- [ ] Error on use-after-move
- [ ] Error on closure capture of affine values
- [ ] Error on affine values in loops
- [ ] Branch consistency validation
- [ ] Clear error messages
- [ ] Tests for all move semantics cases
- [ ] Documentation: ADR for affine types

---

# Issue 011: WASM Runtime + Effect Traces

**Status:** Ready after Issue 010
**Estimated time:** 2-3 weeks
**Phase:** 4 (Runtime)
**Depends on:** Issue 010

## Goal

Compile Strata to WASM. Run with real I/O. Generate effect traces. Support deterministic replay.

## Architecture

- **Compilation:** Emit WAT (WebAssembly Text), convert via wat2wasm
- **Runtime:** WASI host functions for Fs, Net, Time, Rand, Ai
- **Capabilities:** i32 handles into runtime-managed table
- **Tracing:** JSON audit log emitted at host boundary
- **Replay:** Host functions return recorded values from trace

## CLI
```bash
strata build src/app.strata -o app.wasm
strata run app.wasm --trace trace.json
strata run app.wasm --no-trace
strata replay trace.json
```

## Trace Format
```json
{
  "program": "app.strata",
  "started_at": "2026-02-05T10:30:00Z",
  "effects": [
    { "seq": 0, "effect": "Fs", "operation": "read", "inputs": {...}, "outputs": {...} }
  ],
  "result": { "ok": "..." }
}
```

## Definition of Done

- [ ] WAT emitter for Strata AST
- [ ] WASI host functions (fs, net, time, rand, ai)
- [ ] Capability handle threading
- [ ] Trace emission at host boundary
- [ ] JSON trace serialization
- [ ] Replay mode
- [ ] CLI commands: build, run, replay
- [ ] Tests: compile, run, trace, replay
- [ ] Documentation: trace format spec
