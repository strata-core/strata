# Strata - Implemented Features

> Last updated: February 3, 2026
> Status: Issues 001-006 complete, hardening complete (v0.0.7.0)

## ✅ Working Features (v0.0.7.0)

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
- Function calls: `f(a, b, c)` (parsed but not implemented)

**Declarations:**
- Let bindings: `let x = expr;`
- Optional type annotations: `let x: Int = 1;`

**Test Coverage:**
- 13+ integration tests covering precedence, calls, literals
- All example files parse successfully

**What Works:**
```strata
let x = 1;
let y = (1 + 2) * (3 + 4);
let z = true && false;
let w = 1 < 2;
```

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

**Unification:**
- Full unification algorithm with occurs check
- Substitution with composition
- Type error reporting: Mismatch, Occurs, Arity

**Type Context:**
- Basic TypeCtx for managing fresh variables

**Test Coverage:**
- 11 tests covering tuples, lists, vars, occurs check, new type constants
- All pass

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

**Test Coverage:**
- 34 comprehensive tests in `checker_tests.rs`
  - 19 positive tests (valid programs)
  - 11 negative tests (type errors)
  - 4 edge case tests
- All tests pass

**CLI Integration:**
- Type checker runs automatically after parsing
- Type errors displayed with spans before evaluation
- Programs with type errors exit with error code 1

**What Works:**
```strata
let x: Int = 42;           // Type annotation
let y = 2 + 3;             // Type inference
let z = true && false;     // Boolean logic
let w = 1 < 2;             // Comparison
let f: Float = 3.14;       // Float type
let s: String = "hello";   // String type
let result = (1 + 2) * 3;  // Complex expression
```

**Catches Errors:**
```strata
let bad1 = 1 + true;       // Type mismatch: Int vs Bool
let bad2 = "hi" * 5;       // Type mismatch: String vs Int
let bad3: Bool = 123;      // Annotation mismatch
let bad4 = unknown_var;    // Unknown variable
```

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

**Test Coverage:** 49 tests after Issue 005-b

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

// Mutual recursion works
fn is_even(n: Int) -> Bool {
    if n == 0 { true } else { is_odd(n - 1) }
}
fn is_odd(n: Int) -> Bool {
    if n == 0 { false } else { is_even(n - 1) }
}
```

**Location:** `crates/strata-cli/src/eval.rs`, `crates/strata-types/src/infer/constraint.rs`

**Status:** Complete. Full control flow with working evaluator.

---

### Security Hardening (Issue 006-Hardening) ✨ NEW

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

**Test Coverage:** 163 tests (30 new hardening tests)

**Location:** Across all crates (strata-parse, strata-types, strata-cli)

**Status:** Complete. Production-ready security posture for v0.1.

---

### CLI (Issue 001, updated through 006)

**Commands:**
```bash
# Parse and dump AST
strata-cli path/to/file.strata

# Pretty print (not implemented)
strata-cli --format pretty file.strata

# JSON output (not implemented)
strata-cli --format json file.strata

# Evaluate (simple interpreter)
strata-cli --eval file.strata
```

**Type Checking:**
- Runs automatically before evaluation
- Clear error messages with spans
- Exits with error code 1 on type errors

**Evaluator:**
- Arithmetic on Int and Float with mixed-mode
- Relational comparisons
- Logical operators with short-circuit
- String equality
- Block expressions with scoping
- If/else and while loops
- Return statements
- Function calls with closures
- Mutable variable assignment

**Status:** Working for all implemented syntax. Type checking integrated.

---

## ❌ Not Yet Implemented

**Deferred from Issue 001:**
- Match expressions
- ADTs (struct/enum definitions)
- Method chaining: `x.map(f).filter(g)`
- Pipe operator: `x |> f |> g`

**From Later Issues:**
- ADTs & Pattern Matching (Issue 007) ← **NEXT**
- Effect enforcement (Issue 008)
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

# Run all tests (163 tests)
cargo test --workspace

# Run with clippy (enforced in CI)
cargo clippy --workspace --all-targets -- -D warnings

# Run parser tests
cargo test -p strata-parse

# Run type tests
cargo test -p strata-types

# Try an example
cargo run -p strata-cli -- examples/hello.strata
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

## Project Stats (v0.0.7.0)

- **Crates:** 4 (ast, parse, types, cli)
- **Total Tests:** 163 (parser: 38, types: 82, cli: 13, hardening: 30)
- **Lines of Code:** ~5500+ (estimate)
- **Issues Completed:** 6 + hardening (Parser, Effects, Type Scaffolding, Basic Type Checking, Functions, Blocks, Security Hardening)
- **Example Files:** 20+

---

## Next Up

**Issue 007: ADTs & Pattern Matching**
- Struct and enum definitions
- Generic type parameters
- Match expressions
- Pattern matching
- Exhaustiveness checking
