# Strata: In Progress

**Last Updated:** February 4, 2026
**Current Version:** Post-Issue 007 (pre-tag)
**Current Focus:** Issue 008 next
**Progress:** Issues 001-007 complete + code review fixes

---

## Current Status

### Issue 007: ADTs & Pattern Matching ✅
**Completed:** February 4, 2026

**What was built:**

**Algebraic Data Types:**
- Struct definitions with named fields
- Enum definitions with variants (unit, tuple)
- Generic type parameters (`Option<T>`, `Result<T, E>`)
- ADT registry for type lookup

**Tuple Types:**
- Tuple expressions: `(1, 2, 3)`
- Tuple types: `(Int, Bool, String)`
- Nested tuples supported

**Pattern Matching:**
- Match expressions with multiple arms
- Pattern types:
  - Wildcard: `_`
  - Variable binding: `x`
  - Literal: `0`, `true`, `"hello"`
  - Tuple: `(a, b, c)`
  - Struct: `Point { x, y }`
  - Variant: `Option::Some(x)`, `Option::None`

**Exhaustiveness Checking:**
- Maranget's algorithm implementation
- Non-exhaustive match errors with witness patterns
- Redundant/unreachable arm detection
- DoS limits (MAX_PATTERN_MATRIX_SIZE, MAX_EXHAUSTIVENESS_DEPTH)

**Evaluator Support:**
- Tuple construction and destructuring
- Struct construction and field access
- Variant construction
- Pattern matching in match expressions

**Parser Updates:**
- Struct expression syntax: `Point { x: 1, y: 2 }`
- Enum variant access: `Option::Some(42)`, `Option::None`
- Match expressions with pattern arms

**Test stats:** 292 tests passing

**Example programs:**
```strata
enum Option<T> {
    Some(T),
    None
}

fn unwrap_or(opt: Option<Int>, default: Int) -> Int {
    match opt {
        Option::Some(x) => x,
        Option::None => default
    }
}

struct Point { x: Int, y: Int }

fn add_points(p1: Point, p2: Point) -> Point {
    match p1 {
        Point { x: x1, y: y1 } => match p2 {
            Point { x: x2, y: y2 } => Point { x: x1 + x2, y: y1 + y2 }
        }
    }
}
```

---

## Recent Completions

### Issue 006-Hardening ✅
**Completed:** February 2-3, 2026

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

---

### Issue 006: Blocks & Control Flow ✅
**Completed:** February 2, 2026

- Block expressions with tail/semicolon semantics
- If/else expressions with branch type unification
- While loops with proper control flow
- Return statements with propagation
- Mutable let bindings and assignment
- Closures with captured environments
- Self-recursion and mutual recursion

---

## Next Up

### Issue 008: Effect Enforcement
**Status:** Not started
**Estimated time:** TBD

**Goal:** Enforce effect declarations on functions and track effect propagation.

---

## Development Lessons

### Key Learnings from Issue 007
- **Exhaustiveness is critical:** Maranget's algorithm catches subtle bugs
- **Unit variants as values:** Unit variants should be values, not zero-arg constructors
- **Parser lookahead:** Struct expressions need lookahead for disambiguation
- **Evaluator completeness:** All expression types need eval support

### Development Philosophy
- **Soundness over speed** — A type system that lies is worse than no type system
- **Foundation integrity** — Fix bugs before they compound
- **Post-completion review** — Always audit before moving forward

---

## Project Statistics

**Codebase:**
- 4 crates (strata-ast, strata-parse, strata-types, strata-cli)
- ~7,000+ lines of Rust code
- 292 tests (all passing)
- 0 clippy warnings (enforced by CI)

**Velocity:**
- Issues 001-007: ~2.5 months
- On track for v0.1 timeline
