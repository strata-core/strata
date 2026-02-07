# Strata - Implemented Features

> Last updated: February 2026
> Status: Issues 001-010 + 011a complete

## ✅ Working Features

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
- Effect enum: `Fs`, `Net`, `Time`, `Rand`, `Ai`
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
- Block-level let bindings: best-effort check (types may be unsolved; post-solving validation in Issues 008-009 catches most cases)
- `check_let()` now uses `CheckContext` with ADT registry (fixes struct expressions in top-level lets)

### Known Limitations (v0.1)

- **Exhaustiveness deferred on unresolved types:** When the scrutinee has unresolved type variables (e.g., matching on a freshly constructed value without type annotation), exhaustiveness checking is skipped. Workaround: annotate the type or bind to a variable first.
- **Nested enum patterns:** Complex nested patterns may not report exhaustiveness correctly in all cases.
- **Generic type annotations in let bindings:** `let x: Result<Int, String> = ...` not yet supported. Workaround: rely on type inference.
- **Block-level capability check is best-effort:** When types contain unresolved inference variables inside a function body, the capability check may not trigger. Post-solving validation (Issues 008-009) catches most cases.

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

### Effect System (Issue 008) ✨ NEW

**Effect Annotations:**
- Function effect declarations: `fn read(path: String) -> String & {Fs} { ... }`
- Multiple effects: `fn both() -> () & {Fs, Net} { ... }`
- Explicit pure: `fn add(x: Int, y: Int) -> Int & {} { x + y }`
- Implicit pure: no annotation needed for pure functions
- 5 built-in effects: `Fs`, `Net`, `Time`, `Rand`, `Ai`

**Extern Functions:**
- Declaration without body: `extern fn read_file(path: String, fs: FsCap) -> String & {Fs};`
- Pure extern functions: `extern fn calc(x: Int) -> Int;`

**Effect Inference:**
- Unannotated functions infer their effects from the body
- Call site effects propagate to enclosing function
- Multiple effects accumulate across call sites
- Transitive effect propagation (a calls b calls c)

**Effect Checking:**
- Body effects must be a subset of declared effects
- Pure functions cannot call effectful functions
- Unknown effect names produce compile-time errors
- ADT constructors are always pure

**Implementation:**
- Bitmask-based EffectRow with open/closed rows
- Remy-style row unification for effect variables
- Two-phase constraint solver (type equality then effect subset)
- Fixpoint accumulation for multi-call-site propagation
- Effect variable generalization in polymorphic schemes

**What Works:**
```strata
extern fn read_file(path: String, fs: FsCap) -> String & {Fs};
extern fn fetch(url: String, net: NetCap) -> String & {Net};

// Pure function - no effects needed
fn add(x: Int, y: Int) -> Int & {} { x + y }

// Effectful function - must declare its effects and have matching caps
fn download_and_save(fs: FsCap, net: NetCap, url: String, path: String) -> () & {Fs, Net} {
    let data = fetch(url, net);
    read_file(path, fs)
}

// Effects inferred when no annotation
fn load(fs: FsCap, path: String) -> String {
    read_file(path, fs)
}

// Higher-order effect propagation
extern fn single_read(path: String, fs: FsCap) -> String & {Fs};
fn apply2(f, x, y) { f(x, y) }
fn use_it(fs: FsCap) -> String & {Fs} { apply2(single_read, "test.txt", fs) }
```

**Error Messages:**
- `Effect mismatch: expected {}, found {Fs}` — pure fn calls effectful fn
- `Unknown effect 'Foo'; known effects are Fs, Net, Time, Rand, Ai`
- Clear span information for all effect errors

**DoS Protection:**
| Limit | Value | Purpose |
|-------|-------|---------|
| Effect variables | 4,096 | Bound effect inference work |
| Effect solver iterations | 64 | Prevent fixpoint divergence |

**Example Files:**
- `examples/effects_basic.strata` — Basic effectful functions and extern declarations
- `examples/effects_hof.strata` — Higher-order functions with effect propagation

**Location:**
- `crates/strata-types/src/effects.rs` — Effect types and bitmask algebra
- `crates/strata-types/src/infer/solver.rs` — Two-phase constraint solver
- `crates/strata-types/src/infer/unifier.rs` — Remy-style row unification
- `crates/strata-types/src/infer/constraint.rs` — Effect inference at call sites
- `crates/strata-types/src/checker.rs` — Effect checking at function boundaries
- `crates/strata-parse/src/parser.rs` — Effect annotation parsing
- `crates/strata-ast/src/lib.rs` — ExternFnDecl and effect fields

**Status:** Complete. Compile-time effect enforcement working.

---

### Issue 009: Capability Types (Complete ✅)

**Completed:** February 6-7, 2026
**Duration:** 1 week
**Test Coverage:** 402 tests (48 new)

**What was implemented:**

**Capability Types:**
- Five built-in capability types: `FsCap`, `NetCap`, `TimeCap`, `RandCap`, `AiCap`
- First-class `Ty::Cap(CapKind)` in the type system (leaf type, not ADT)
- 1:1 mapping: each capability gates exactly one effect

**Mandatory Capability Validation:**
- `validate_caps_against_effects()` shared helper — mandatory for ALL functions and externs
- Functions with concrete effects MUST have matching capability parameters (no exceptions)
- Extern functions with declared effects MUST have matching capability parameters
- Open tail effect variables (from HOF callbacks) are not checked (correct — parametric effects)
- Unused capabilities allowed (pass-through pattern)

**Name Shadowing Protection:**
- ADT definitions cannot use reserved capability type names
- `struct FsCap {}` or `enum NetCap { ... }` produce `ReservedCapabilityName` error

**Ergonomic Error Messages:**
- `Function 'bad' requires capability FsCap because its effect row includes {Fs}` with help text
- Extern: `Add a 'fs: FsCap' parameter. Alternatively, remove {Fs} from the effect annotation if this extern is actually pure.`
- Hints for FsCap/Fs confusion: `Did you mean {Fs}? Capability types like FsCap go in parameter types, not effect annotations`
- Hints for Fs/FsCap confusion: `Did you mean FsCap? Effects like Fs go in & {...}, capability types are parameter types`

**Security Milestone: Zero-Cap Skip Eliminated**

The initial implementation included a conditional guard (`if !param_caps.is_empty()`) that silently skipped capability validation for functions with zero capability parameters. This meant:

```strata
fn bad() -> () & {Fs} {}        // Should ERROR, compiled silently
fn middle() -> () & {Fs} { bad() }
fn main() -> () { middle() }    // Entire chain bypassed security
```

Three independent external reviewers flagged this as critical. The fix:
- Removed conditional skip at checker.rs `check_fn()` capability validation
- Removed conditional skip at extern function validation (pass 1c)
- Made `validate_caps_against_effects()` mandatory — called unconditionally
- Test `adversarial_zero_caps_entire_call_chain` verifies the fix holds

**External Validation (3 Independent Sources):**

> Security Review (7/10, up from 5/10): "The core bypass (ambient via skip) is patched—zero-cap functions/externs with concrete effects now error as MissingCapability/ExternMissingCapability, enforcing 'no ambient' soundly."

> PL Researcher: "Green Light — Strata now possesses a mathematically sound capability-security model."

> Technical Assessment (0.87-0.90, up from 0.80-0.85): "You removed the single biggest security/DX footgun (skip-on-zero-caps). That wasn't a minor bug; it was a 'silent bypass' class."

**What Works:**
```strata
// Extern functions must have cap params matching their effects
extern fn read_file(path: String, fs: FsCap) -> String & {Fs};
extern fn fetch(url: String, net: NetCap) -> String & {Net};

// Capability gates effect usage
fn load_config(fs: FsCap, path: String) -> String & {Fs} {
    read_file(path, fs)
}

// Multiple capabilities for multiple effects
fn download_and_save(fs: FsCap, net: NetCap, url: String, path: String) -> () & {Fs, Net} {
    let data = fetch(url, net);
    write_file(path, data, fs)
}

// Pass-through: capability param without corresponding effect is valid
fn passthrough(fs: FsCap, x: Int) -> Int { x + 1 }
```

**Known Limitations:**
- Open effect tail polymorphism at instantiation sites may need additional checking
- No warnings for unused capabilities (intentional — pass-through pattern)
- Capability provisioning (how `main` gets capabilities from runtime) deferred to Issue 011
- No `using` keyword syntax yet (planned for v0.2 ergonomics)
**What's Next:** Issue 010 (Affine Types) — prevent capability duplication and leaking

**Example Files:**
- `examples/capabilities.strata` — Capability types with effect gating

**Location:**
- `crates/strata-types/src/effects.rs` — `CapKind` enum and cap-effect mapping
- `crates/strata-types/src/infer/ty.rs` — `Ty::Cap` variant
- `crates/strata-types/src/checker.rs` — Capability validation in `check_fn()` and extern fns
- `crates/strata-types/src/adt.rs` — Updated `contains_capability()` for `Ty::Cap`

**Status:** Complete. Mandatory capability-effect enforcement working. Externally validated.

---

### Pre-011 Security Hardening (v0.0.10.1)

**Completed:** February 2026
**Test Coverage:** 442 tests (7 new hardening tests)

**Security fix: `Ty::kind()` propagates affinity through compound types**

- `Ty::kind()` now recurses into ADT type args, tuple elements, and list inner type
- Generic ADTs containing capabilities (e.g., `Smuggler<FsCap>`) are correctly classified as `Kind::Affine`
- Prevents capability laundering: wrapping a cap in a generic ADT and copying the wrapper no longer bypasses affine enforcement
- 4 new tests covering exploit chain, negative case (unrestricted generic), nested generics, and pattern extraction

**Defense-in-depth: Move checker resolves generic field types**

- Move checker accepts `AdtRegistry` for ADT definition lookup
- `introduce_pattern_bindings` for `Pat::Variant` and `Pat::Struct` resolves generic field types by substituting scrutinee type args into ADT definition field types
- When caps-in-ADTs ban is lifted, extracted fields will be correctly tracked as affine

**Closure affinity documentation**

- `Ty::kind()` documented with the rule that closures capturing affine values must themselves be affine
- Not enforceable until closures gain capture tracking; current enforcement is absence of closure syntax
- Test verifies closure syntax does not parse

---

### Issue 010: Affine Types / Linear Capabilities (Complete)

**Completed:** February 2026
**Test Coverage:** 435 tests (33 new move checker tests)

**What was implemented:**

**Kind System:**
- `Kind` enum: `Unrestricted` (free use) and `Affine` (at-most-once)
- `Ty::kind()` method: `Ty::Cap(_)` returns `Affine`; compound types propagate affinity from contents
- Kind is intrinsic to the type — no user annotation needed

**Move Checker (post-inference pass):**
- Separate validation pass after type inference — does NOT modify unification or solver
- Tracks binding consumption: each affine binding can be used at most once
- Generation-based binding IDs for correct shadowing handling
- Let-binding transfers ownership: `let a = fs;` consumes `fs`, makes `a` alive
- Function call arguments evaluated left-to-right with cumulative move state
- Pessimistic branch join: if consumed in ANY branch, consumed after if/else/match
- Loop rejection: capability use inside while loops is an error
- Polymorphic return type resolution via manual scheme instantiation

**CapabilityInBinding Replaced:**
- `let cap = fs;` is now a valid ownership transfer (was an error in Issue 009)
- The move checker enforces single-use semantics instead
- ADT field capability ban remains (capabilities cannot be stored in struct/enum fields)

**Error Messages (permission/authority vocabulary):**
- `capability 'fs' has already been used; permission was transferred at ...; 'fs' is no longer available`
- `cannot use single-use capability 'fs' inside loop; 'fs' would be used on every iteration`
- No Rust-style "moved value" language — designed for SRE/DevOps audience

**What Works:**
```strata
// Single use — valid
fn save(fs: FsCap, data: String) -> () & {Fs} {
    write_file(fs, data)
}

// Let transfer — valid
fn transfer(fs: FsCap) -> () & {Fs} {
    let cap = fs;
    use_cap(cap)
}

// Branch consistency — valid (each branch uses fs once)
fn conditional(fs: FsCap, c: Bool) -> () & {Fs} {
    if c { use_cap(fs) } else { use_cap(fs) }
}

// Unused drop — valid (affine = at-most-once, not exactly-once)
fn passthrough(fs: FsCap) -> () & {} { () }

// ERRORS:
fn double(fs: FsCap) -> () & {Fs} {
    use_cap(fs);
    use_cap(fs)  // ERROR: capability 'fs' has already been used
}

fn looped(fs: FsCap) -> () & {Fs} {
    while true { use_cap(fs) }  // ERROR: cannot use inside loop
}
```

**Security Milestone: Capability Model Complete**

| Property | Enforced by | Issue |
|----------|-------------|-------|
| No forging | Built-in types, reserved names | 009 |
| No ambient authority | Effects require matching capabilities | 009 |
| No duplication | Affine single-use enforcement | 010 |
| No leaking | Cannot store in ADTs or closures | 009 + 010 |
| Explicit flow | Capabilities threaded through calls | 009 + 010 |
| Auditability | Capability usage visible in signatures | 009 |

**Known Limitations (v0.1):**
- Affine ADTs partially supported: generic ADTs with cap type args are affine (v0.0.10.1), but direct cap fields in ADT definitions remain banned
- No closure/lambda in the AST, so closure capture checking is deferred
- No borrowing/referencing of capabilities (`&FsCap` for read-only access)
- Module-level let bindings with capability types are not move-checked

**Location:**
- `crates/strata-types/src/infer/ty.rs` — `Kind` enum and `Ty::kind()` method
- `crates/strata-types/src/move_check.rs` — Move checker module
- `crates/strata-types/src/checker.rs` — Integration into `check_fn()`

**Status:** Complete. Affine capability enforcement working.

---

### Issue 011a: Traced Runtime (Complete)

**Completed:** February 2026
**Test Coverage:** 493 tests (41 new across 5 phases + hardening)

**What was implemented:**

**Phase 1: Extern-Only Borrowing (`&CapType`)**
- `&T` syntax in extern fn params — borrow a capability without consuming it
- `Ty::Ref(Box<Ty>)` — reference type, always `Kind::Unrestricted`
- Move checker treats borrows as non-consuming (capability survives)
- Restriction: `&T` only allowed in extern fn params (not regular fns, returns, let bindings)

**Phase 2: Host Function Dispatch**
- `HostRegistry` with built-in host functions: `read_file`, `write_file`, `now`, `random_int`
- Capability injection: `fn main(fs: FsCap, time: TimeCap)` receives capabilities from runtime
- `Value::Cap(CapKind)` and `Value::HostFn(String)` runtime values
- Position-aware dispatch via `ExternFnMeta` (walks type signature, not runtime values)

**Phase 3: Effect Trace Emission**
- Streaming JSONL trace: every host fn call records effect, operation, capability access, inputs, output, duration
- SHA-256 content hashing for outputs > 1KB (configurable via `--trace-full`)
- `TraceEmitter` with `dispatch_traced()` — single dispatch path for all host calls
- ISO 8601 timestamps without chrono dependency

**Phase 4: Deterministic Replay**
- `TraceReplayer` loads JSONL traces and substitutes recorded outputs
- Validates operation names and inputs match the trace
- `ReplayError` enum with structured mismatch reporting
- `run_module_replay()` entry point with `verify_complete()` check

**Phase 5: CLI Integration**
- `strata run <file>` — execute program
- `strata run <file> --trace <path>` — execute with audit trace (large values hashed)
- `strata run <file> --trace-full <path>` — execute with replay-capable trace
- `strata replay <trace-path> <file>` — replay trace against source
- `strata replay <trace-path>` — print trace summary
- `strata parse <file>` — dump AST (pretty or JSON)

**What Works:**
```strata
extern fn read_file(fs: &FsCap, path: String) -> String & {Fs};
extern fn write_file(fs: &FsCap, path: String, content: String) -> () & {Fs};
extern fn now(time: &TimeCap) -> String & {Time};

fn main(fs: FsCap, time: TimeCap) -> () & {Fs, Time} {
    let config = read_file(&fs, "/tmp/config.txt");
    let timestamp = now(&time);
    write_file(&fs, "/tmp/output.txt", config);
}
```

```bash
# Run with trace
strata run example.strata --trace trace.jsonl

# Replay
strata replay trace.jsonl example.strata
```

**Example Files:**
- `examples/deploy.strata` — Full run-trace-replay demo
- `examples/README.md` — Demo workflow documentation

**Location:**
- `crates/strata-cli/src/host.rs` — Host registry, trace types, TraceEmitter, TraceReplayer
- `crates/strata-cli/src/eval.rs` — Evaluator with capability injection, trace threading, replay dispatch
- `crates/strata-cli/src/main.rs` — CLI with run/replay/parse subcommands

**Hardening (post-Phase 5):**

6 hardening fixes + 16 new tests:

1. **Ty::Ref second-class enforcement** — `is_first_class()` on Ty, post-solving settle points in check_let/check_fn, pre-solve check in infer_stmt, RefInAdtField check BEFORE capability check in register_struct/register_enum
2. **TraceValue tagged serialization** — `#[serde(tag = "t", content = "v")]` replaces lossy serialize/deserialize, BTreeMap for deterministic key order
3. **Input hashing** — Audit mode (`--trace`) hashes large inputs AND outputs; replay requires `--trace-full`
4. **Trace write failures abort execution** — `emit()` and `finalize()` return `Result<(), HostError>`, propagated through dispatch_traced and eval
5. **TraceRecord envelope** — Header/Effect/Footer with schema version validation
6. **Cap rejection in deserialization** — TraceValue enum only has data variants; Cap/Closure/Tuple tags rejected by serde

**Security regression tests:**
- Schema version validation (rejects unknown versions)
- Footer completeness detection (truncated traces detected)
- Tagged value type confusion (Str("42") vs Int(42) round-trip preserved)
- Cap/Closure/Tuple rejection in trace deserialization

**Status:** Complete. End-to-end traced runtime with deterministic replay. Hardened for security.

---

### CLI (Issue 001, updated through 011a)

**Commands:**
```bash
# Execute a program
strata run file.strata

# Execute with effect trace
strata run file.strata --trace trace.jsonl

# Execute with replay-capable trace (all values recorded)
strata run file.strata --trace-full trace.jsonl

# Replay a trace against source
strata replay trace.jsonl file.strata

# Print trace summary
strata replay trace.jsonl

# Parse and dump AST
strata parse file.strata

# JSON AST output
strata parse file.strata --format json
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
- Host function dispatch with capability injection
- Effect trace emission and deterministic replay

**Status:** Working for all implemented syntax. Full traced runtime integrated.

---

## ❌ Not Yet Implemented

**Deferred from Issue 001:**
- Method chaining: `x.map(f).filter(g)`
- Pipe operator: `x |> f |> g`

**From Later Issues:**
- Profile enforcement
- Actors & supervision
- Datalog/logic engine
- Bytecode VM / WASM compilation
- AOT compilation
- WASI target
- `--deny` capability flags (deny specific caps at runtime)
- Network host functions (http_get, etc.)
- Error handling / Result types in Strata

---

## Build & Test

```bash
# Build all crates
cargo build --workspace

# Run all tests (493 tests)
cargo test --workspace

# Run with clippy (enforced in CI)
cargo clippy --workspace --all-targets -- -D warnings

# Run parser tests
cargo test -p strata-parse

# Run type tests
cargo test -p strata-types

# Run a program
cargo run -p strata-cli -- run examples/deploy.strata

# Run with trace
cargo run -p strata-cli -- run examples/deploy.strata --trace /tmp/trace.jsonl

# Parse an example
cargo run -p strata-cli -- parse examples/option.strata
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

## Project Stats

- **Crates:** 4 (ast, parse, types, cli)
- **Total Tests:** 493 (parser: 48, types: 174, cli: 26, host_integration: 29, cli_integration: 4, effects: 41, capabilities: 66, move_check: 38, others: 67)
- **Lines of Code:** ~12,000+ (estimate)
- **Issues Completed:** 10 + hardening + 011a (Parser, Effects, Type Scaffolding, Basic Type Checking, Functions, Blocks, ADTs, Security Hardening, Effect System, Capability Types, Affine Types, Pre-011 Hardening, Traced Runtime)
- **Example Files:** 30+

---

## Next Up

- Standard library functions (string manipulation, collections)
- Network host functions (http_get, etc.)
- Error handling / Result types
- `--deny` capability flags for CLI
- WASM compilation target
