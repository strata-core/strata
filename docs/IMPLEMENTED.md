# Strata - Implemented Features

> Last updated: February 4, 2026
> Status: Issues 001-007 complete (v0.0.8.0)

## ✅ Working Features (v0.0.8.0)

### Parser & AST (Issue 001)

**Lexer:**
- All token types: keywords, identifiers, literals, operators, punctuation
- String literals with escape sequences
- Int and Float literals
- Comments (line and block)

**Expressions:**
- Literals: `1`, `3.14`, `"hello"`, `true`, `false`, `nil`
- Variables: `x`, `myVar`
- Unary operators: `!expr`, `-expr`
- Binary operators with correct precedence:
  - Logical: `||`, `&&`
  - Equality: `==`, `!=`
  - Relational: `<`, `<=`, `>`, `>=`
  - Arithmetic: `+`, `-`, `*`, `/`
- Parentheses: `(expr)`
- Function calls: `f(a, b, c)`

**Declarations:**
- Let bindings: `let x = expr;`
- Optional type annotations: `let x: Int = 1;`

**Test Coverage:**
- 13+ integration tests covering precedence, calls, literals
- All example files parse successfully

---

### Effects & Profiles (Issue 002)

**Effects:**
- Effect enum: `Net`, `FS`, `Time`, `Rand`, `Fail`
- EffectRow as canonical set
- Set operations: `is_subset_of()`, `union()`

**Profiles:**
- Profile enum: `Kernel`, `Realtime`, `General`
- `allowed_effects()` mapping

**Location:** `crates/strata-types/src/{effects.rs, profile.rs}`

**Status:** Core types exist, ready for enforcement layer.

---

### Type Inference Scaffolding (Issue 003)

**Inference Types (`Ty`):**
- Type variables: `TypeVarId(u32)`
- Constants: `Unit`, `Bool`, `Int`, `Float`, `String`
- Functions: `Arrow(A, B)`
- Tuples: `Tuple(Vec<Ty>)` - heterogeneous
- Lists: `List(Box<Ty>)` - homogeneous
- ADTs: `Adt { name, args }` - generic algebraic data types

**Unification:**
- Full unification algorithm with occurs check
- Substitution with composition
- Type error reporting: Mismatch, Occurs, Arity

**Type Context:**
- Basic TypeCtx for managing fresh variables

**Location:** `crates/strata-types/src/infer/`

**Status:** Solid foundation. Unifier is correct and well-tested.

---

### Basic Type Checking (Issue 004)

**Type Checker:**
- Expression type inference for all AST nodes
- Type environment for let bindings
- Variable lookup and scoping
- Optional type annotations with verification
- Clear error messages with source spans

**Type Rules:**
- Literals: Int, Float, Bool, String, Unit
- Unary operations: `!` (Bool → Bool), `-` (Int/Float → Int/Float)
- Binary operations:
  - Arithmetic: `+`, `-`, `*`, `/` (Int+Int→Int, Float+Float→Float)
  - Comparison: `<`, `<=`, `>`, `>=` (Int/Float → Bool)
  - Equality: `==`, `!=` (same type → Bool)
  - Logical: `&&`, `||` (Bool+Bool → Bool)
- Let bindings with inference and annotation checking
- Parenthesized expressions

**Location:** `crates/strata-types/src/checker.rs`

**Status:** Complete. Basic type checking working for all implemented syntax.

---

### Functions & Type Inference (Issue 005)

**Constraint-Based Inference:**
- InferCtx for fresh variable generation and constraint collection
- Solver for constraint solving via unification
- Generalization (∀ introduction) and instantiation (∀ elimination)
- Polymorphic type schemes (let-polymorphism)

**Functions:**
- Function declarations with multi-param arrows
- Function call type checking
- Higher-order function support
- Two-pass module checking (forward references, mutual recursion)

**Soundness Hardening (005-b):**
- Unknown identifiers error properly
- Real unification errors (no placeholders)
- Correct generalization boundaries
- Numeric constraints on arithmetic
- Constraint provenance (span tracking)
- Determinism audit

**What Works:**
```strata
fn add(x: Int, y: Int) -> Int { x + y }
fn double(n: Int) -> Int { add(n, n) }

// Polymorphic identity
fn identity(x) { x }
let a = identity(42);    // Int
let b = identity(true);  // Bool

// Forward references
fn f() -> Int { g() }
fn g() -> Int { 42 }
```

**Location:** `crates/strata-types/src/infer/`, `crates/strata-types/src/checker.rs`

**Status:** Complete. Sound type inference with polymorphism.

---

### Blocks & Control Flow (Issue 006)

**Block Expressions:**
- `{ stmt; stmt; expr }` with tail expression semantics
- Proper semicolon rules (trailing semicolon = Unit)
- Nested blocks with lexical scoping

**Control Flow:**
- If/else expressions (branches must unify)
- While loops
- Return statements (propagate through nested blocks)

**Mutable Bindings:**
- `let mut x = expr;` declarations
- Assignment statements: `x = expr;`
- Mutability checking (immutable assignment errors)

**Evaluator:**
- Scope stack with push/pop for blocks
- Closures with captured environments
- Self-recursion and mutual recursion support
- Control flow enum (Value, Return, Break, Continue)

**Ty::Never (Bottom Type):**
- Diverging expressions (return, infinite loops) have type `Never`
- Never only unifies with itself (conservative, not wildcard)
- Sound handling in if/else and block inference

**What Works:**
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

**Location:** `crates/strata-cli/src/eval.rs`, `crates/strata-types/src/infer/constraint.rs`

**Status:** Complete. Full control flow with working evaluator.

---

### Security Hardening (Issue 006-Hardening)

**DoS Protection Limits:**
| Limit | Value | Purpose |
|-------|-------|---------|
| Source size | 1 MB | Prevent memory exhaustion |
| Token count | 200,000 | Bound lexer work |
| Parser nesting | 512 | Prevent stack overflow in parser |
| Inference depth | 512 | Bound type inference recursion |
| Eval call depth | 1,000 | Prevent runaway recursion at runtime |

**Soundness Fixes:**
- `Ty::Never` no longer unifies with arbitrary types
- Divergence handled correctly in inference (if/else, blocks)
- Removed panic!/expect() from type checker
- Token limit latching (can't reset mid-parse)
- Scope guard for guaranteed pop_scope()

**Parser Improvements:**
- `::` qualified type paths
- Span-end fixes for accurate error locations
- Universal lexer error surfacing
- Nesting guards for if/while/else-if chains

**Location:** Across all crates (strata-parse, strata-types, strata-cli)

**Status:** Complete. Production-ready security posture for v0.1.

---

### ADTs & Pattern Matching (Issue 007) ✨ NEW

**Struct Definitions:**
- Named fields: `struct Point { x: Int, y: Int }`
- Generic type parameters: `struct Pair<T, U> { first: T, second: U }`
- Struct construction: `Point { x: 1, y: 2 }`
- Struct patterns in match: `Point { x, y } => ...`

**Enum Definitions:**
- Unit variants: `None`
- Tuple variants: `Some(T)`, `Ok(T)`, `Err(E)`
- Generic enums: `Option<T>`, `Result<T, E>`
- Variant construction: `Option::Some(42)`, `Option::None`
- Variant patterns in match: `Option::Some(x) => ...`

**Tuple Types:**
- Tuple expressions: `(1, 2, 3)`
- Tuple types: `(Int, Bool, String)`
- Empty tuple (unit): `()`
- Nested tuples: `((1, 2), 3)`
- Tuple patterns: `(a, b) => ...`

**Pattern Matching:**
- Match expressions: `match x { pat => expr, ... }`
- Pattern types:
  - Wildcard: `_`
  - Variable binding: `x`
  - Literal: `0`, `true`, `"hello"`
  - Tuple: `(a, b, c)`
  - Struct: `Point { x, y: 0 }`
  - Variant: `Option::Some(x)`
  - Nested patterns: `Option::Some((a, b))`

**Exhaustiveness Checking:**
- Maranget's algorithm for exhaustiveness
- Non-exhaustive match errors with witness patterns
- Redundant arm detection (unreachable patterns)
- DoS protection limits

**What Works:**
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

fn is_some(opt: Option<Int>) -> Bool {
    match opt {
        Option::Some(_) => true,
        Option::None => false
    }
}

enum Result<T, E> {
    Ok(T),
    Err(E)
}

fn unwrap_result(r: Result<Int, String>) -> Int {
    match r {
        Result::Ok(x) => x,
        Result::Err(_) => 0
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

fn swap(pair: (Int, Int)) -> (Int, Int) {
    match pair {
        (a, b) => (b, a)
    }
}
```

**Destructuring Let:**
- Irrefutable patterns in let bindings: `let (a, b) = (1, 2);`
- Tuple destructuring: `let (x, y, z) = triple;`
- Nested patterns: `let ((a, b), c) = nested_tuple;`
- Wildcard patterns: `let _ = expr;`
- Refutable pattern detection with helpful error messages

**Capability Check on Bindings (Code Review Fix):**
- Top-level let bindings checked for capability types after constraint solving
- Block-level let bindings: best-effort check (types may be unsolved; full coverage in Issue 008)
- `check_let()` now uses `CheckContext` with ADT registry (fixes struct expressions in top-level lets)

### Known Limitations (v0.1)

- **Exhaustiveness deferred on unresolved types:** When the scrutinee has unresolved type variables (e.g., matching on a freshly constructed value without type annotation), exhaustiveness checking is skipped. Workaround: annotate the type or bind to a variable first.
- **Nested enum patterns:** Complex nested patterns may not report exhaustiveness correctly in all cases.
- **Generic type annotations in let bindings:** `let x: Result<Int, String> = ...` not yet supported. Workaround: rely on type inference.
- **Capability binding semantics:** `let x = cap;` (bare rebinding) is currently rejected alongside container storage. Will be refined when capability system lands in Issue 008/009. Conservative default is intentional.
- **Block-level capability check is best-effort:** When types contain unresolved inference variables inside a function body, the capability check may not trigger until the effect system provides post-solving analysis (Issue 008).

**Example Files:**
- `examples/option.strata` - Option enum with unwrap_or, is_some, map_option
- `examples/result.strata` - Result enum with unwrap_result, is_ok, map_result
- `examples/tuple.strata` - Tuple operations: swap, first, second, nested tuples
- `examples/struct.strata` - Point struct with construction and pattern matching

**Location:**
- `crates/strata-ast/src/lib.rs` - AST nodes for structs, enums, patterns
- `crates/strata-parse/src/parser.rs` - Parser for ADT syntax
- `crates/strata-types/src/adt.rs` - ADT registry and definitions
- `crates/strata-types/src/exhaustive.rs` - Exhaustiveness checker
- `crates/strata-types/src/infer/constraint.rs` - Type inference for ADTs
- `crates/strata-cli/src/eval.rs` - Evaluator for ADT expressions

**Status:** Complete. Full ADT support with exhaustiveness checking.

---

### CLI (Issue 001, updated through 007)

**Commands:**
```bash
# Parse and dump AST
strata-cli path/to/file.strata

# Pretty print
strata-cli --format pretty file.strata

# JSON output
strata-cli --format json file.strata

# Evaluate (calls main() and prints result)
strata-cli --eval file.strata
```

**Type Checking:**
- Runs automatically before evaluation
- Clear error messages with spans
- Exits with error code 1 on type errors

**Evaluator:**
- Arithmetic on Int and Float
- Relational comparisons
- Logical operators with short-circuit
- String equality
- Block expressions with scoping
- If/else and while loops
- Return statements
- Function calls with closures
- Mutable variable assignment
- Tuple construction and destructuring
- Struct construction and pattern matching
- Enum variant construction and matching

**Status:** Working for all implemented syntax. Type checking integrated.

---

## ❌ Not Yet Implemented

**Deferred from Issue 001:**
- Method chaining: `x.map(f).filter(g)`
- Pipe operator: `x |> f |> g`

**From Later Issues:**
- Effect enforcement (Issue 008) ← **NEXT**
- Capability checking (Issue 009)
- Profile enforcement (Issue 010)
- Actors & supervision
- Datalog/logic engine
- Bytecode VM
- Deterministic replay
- AOT compilation
- WASI target

---

## Build & Test

```bash
# Build all crates
cargo build --workspace

# Run all tests (292 tests)
cargo test --workspace

# Run with clippy (enforced in CI)
cargo clippy --workspace --all-targets -- -D warnings

# Run parser tests
cargo test -p strata-parse

# Run type tests
cargo test -p strata-types

# Try an example
cargo run -p strata-cli -- examples/option.strata
cargo run -p strata-cli -- --eval examples/option.strata
```

---

## Crate Dependencies

```
strata-ast       (no deps)
  ├── strata-parse (depends on strata-ast)
  │     └── strata-cli (depends on strata-ast, strata-parse, strata-types)
  └── strata-types (depends on strata-ast)
```

---

## Project Stats (v0.0.8.0)

- **Crates:** 4 (ast, parse, types, cli)
- **Total Tests:** 292 (parser: 38, types: 161, cli: 26, others: 67)
- **Lines of Code:** ~7500+ (estimate)
- **Issues Completed:** 7 + hardening (Parser, Effects, Type Scaffolding, Basic Type Checking, Functions, Blocks, ADTs, Security Hardening)
- **Example Files:** 28+

---

## Next Up

**Issue 008: Effect Enforcement**
- Effect annotations on functions
- Effect propagation through calls
- Effect checking at compile time
