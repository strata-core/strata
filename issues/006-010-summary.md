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

**Status:** Design Review  
**Estimated time:** 10-14 days  
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
        Some(x) => x,
        None => default,
    }
}
```

## Scope
- Struct definitions
- Enum definitions with variants
- Generic type parameters
- Match expressions
- Pattern matching (literals, vars, constructors)
- Exhaustiveness checking

## Design Decisions Under Review

| ID | Decision | Recommendation |
|----|----------|----------------|
| D1 | Nested patterns (`Some(Some(x))`) | Yes |
| D2 | Match guards (`if x > 0`) | Defer |
| D3 | Tuple types `(Int, String)` | Defer |
| D4 | Variant syntax (unit/tuple/struct) | All three |
| D5 | Struct update (`..other`) | Defer |
| D6 | Visibility (`pub struct`) | Defer |

## Soundness Concerns (from Gemini)

1. **Constructor Variance** — `Option<Never>` problem
   - Solution: Constructors as polymorphic functions (`None : ∀T. () -> Option<T>`)

2. **Pattern Scoping** — Bindings introduced by patterns
   - Solution: `PatternBindings` struct, mutability tracked per-binding

3. **Exhaustiveness + Never** — Match type when arms return Never
   - Solution: Match type = join of non-Never arm types

4. **Redundant Patterns** — Dead arms as security risk
   - Solution: Maranget algorithm with warnings

## Definition of Done
- [ ] AST for struct/enum/match
- [ ] Parser for ADT syntax
- [ ] Type checking with generics
- [ ] Pattern compilation
- [ ] Exhaustiveness checker (Maranget)
- [ ] Tests for Option/Result patterns
- [ ] Evaluator support

---

# Issue 008: Effect Syntax & Checking

**Status:** Planned  
**Estimated time:** 10 days  
**Phase:** 3 (Effect System)  
**Depends on:** Issue 007

## Goal
Parse and check effect annotations at compile time.

## Note on Elaboration
Issues 005 (constraint solving) and 008 (effect checking) together form 
the elaboration phase: converting inferred types (Ty with variables) to 
surface types (Type with effect rows).

## Syntax
```strata
fn read_file(path: String, using fs: FsCap) 
    -> Result<String, IoErr> & {FS} 
{
    // ...
}
```

## Scope
- Parse `& {Effect1, Effect2}` syntax
- Parse `using cap: CapType` parameters
- Effect inference (extend constraint system)
- Effect checking (callee ⊆ caller)

## Definition of Done
- [ ] Effect annotation syntax in parser
- [ ] Effect row in function types
- [ ] Effect inference algorithm
- [ ] Effect subtyping checker
- [ ] Tests for effect errors
- [ ] Tests for effect polymorphism

---

# Issue 009: Capability Types

**Status:** Planned  
**Estimated time:** 5 days  
**Phase:** 3 (Effect System)  
**Depends on:** Issue 008

## Goal
Define capability types and enforce no-ambient-authority.

## Scope
- Standard caps: `NetCap`, `FsCap`, `TimeCap`, `RandCap`
- Capability must be in scope to use effect
- Compiler error if effect used without cap

## Definition of Done
- [ ] Capability type definitions
- [ ] Compiler checks cap in scope
- [ ] Error when effect used without cap
- [ ] Tests for missing caps

---

# Issue 010: Profile Enforcement

**Status:** Planned  
**Estimated time:** 5 days  
**Phase:** 3 (Effect System)  
**Depends on:** Issue 009

## Goal
Enforce profile restrictions at compile time.

## Syntax
```strata
@profile(Kernel)
fn safe(x: Int) -> Int {
    // Cannot use Net, FS, Time, Rand
    x * 2
}
```

## Scope
- Parse `@profile(...)` annotation
- Check no forbidden effects
- Check no dynamic allocation (Kernel)
- Module-level profiles

## Definition of Done
- [ ] Profile annotation syntax
- [ ] Profile checking pass
- [ ] Tests for each profile
- [ ] Error messages for violations
