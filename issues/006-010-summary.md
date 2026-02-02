# Issue 006: Blocks & Control Flow

**Status:** Planned  
**Estimated time:** 7 days  
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
    }
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
- [ ] AST nodes for Block, If, While, Return
- [ ] Parser for control flow syntax
- [ ] Type checking: if branches same type, block final expr is type
- [ ] Evaluator support for all constructs
- [ ] Tests for control flow
- [ ] Tests for type errors in branches

---

# Issue 007: ADTs & Pattern Matching

**Status:** Planned  
**Estimated time:** 10-14 days  
**Phase:** 2 (Basic Type System)  
**Depends on:** Issue 006

**Scope Note:** This issue now includes traits (originally planned as Issue 004), which makes more sense after we have ADTs to implement traits on.

## Goal
Add structs, enums, generics, traits, and pattern matching.

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

## Definition of Done
- [ ] AST for struct/enum/match
- [ ] Parser for ADT syntax
- [ ] Type checking with generics
- [ ] Pattern compilation
- [ ] Exhaustiveness checker
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
