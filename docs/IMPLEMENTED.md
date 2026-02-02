# Strata - Implemented Features

> Last updated: January 31, 2026  
> Status: Issues 001-004 complete (v0.0.4)

## ✅ Working Features (v0.0.4)

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

### Basic Type Checking (Issue 004) ✨ NEW

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

### CLI (Issue 001, updated in 004)

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
- Error on unknown variables or unsupported calls

**Status:** Working for all implemented syntax. Type checking integrated.

---

## ❌ Not Yet Implemented

**Deferred from Issue 001:**
- Blocks: `{ stmt; stmt; expr }`
- Match expressions
- ADTs (struct/enum definitions)
- Function definitions: `fn f(x: Int) -> Int { ... }` ← Issue 005
- Method chaining: `x.map(f).filter(g)`
- Pipe operator: `x |> f |> g`

**From Later Issues:**
- Function type checking and inference (Issue 005)
- Constraint-based inference (Issue 005)
- Polymorphism (Issue 005)
- Traits (Issue 007)
- Effect enforcement (Issue 008)
- Capability checking (Issue 009)
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

# Run all tests (45+ tests)
cargo test --workspace

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

## Project Stats (v0.0.4)

- **Crates:** 4 (ast, parse, types, cli)
- **Total Tests:** 45+ (parser: 13, types: 11, checker: 34+)
- **Lines of Code:** ~2000+ (estimate)
- **Issues Completed:** 4 (Parser, Effects, Type Scaffolding, Basic Type Checking)
- **Example Files:** 17
- **Time Spent:** ~1 month actual work (spread over 3 months)

---

## Next Up

**Issue 005: Functions & Inference**
- Function definitions and calls
- Constraint-based type inference
- Polymorphism and let-polymorphism
- Function type checking
