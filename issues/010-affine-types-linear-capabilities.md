# Issue 010: Affine Types (Linear Capabilities)

**Status:** Ready after Issue 009
**Estimated time:** 7-10 days
**Phase:** 3 (Effect System)
**Depends on:** Issue 009 (Capability Types)
**Blocks:** Issue 011 (WASM Runtime)

## Goal

Prevent capability duplication and leaking by making capabilities *affine* — they can be used at most once. This ensures capabilities cannot be copied, stored globally, or otherwise escaped from their intended scope.

## The Problem We're Solving

After Issue 009, capabilities gate effects. But without linearity:
```strata
fn leak(fs: FsCap) -> () & {} {
    let copy1 = fs;
    let copy2 = fs;  // Duplicated!
    store_in_global(copy1);  // Leaked!
    ()  // Returns pure, but fs is now ambient
}
```

This defeats capability security. With affine types:
```strata
fn leak(fs: FsCap) -> () & {} {
    let copy1 = fs;      // fs moved to copy1
    let copy2 = fs;      // ERROR: fs was already moved
}
```

## Core Concept: Kinds

Types have *kinds* that describe their usage constraints:

| Kind | Name | Meaning |
|------|------|---------|
| `*` | Unrestricted | Can be used any number of times (Int, String, Bool, etc.) |
| `!` | Affine | Can be used *at most once* (capabilities) |

All capability types (`FsCap`, `NetCap`, etc.) have kind `!`.

## Syntax

No new surface syntax required. Affinity is intrinsic to capability types.
```strata
// FsCap is implicitly affine — this is a type system property
fn use_twice(fs: FsCap) -> () & {Fs} {
    do_thing_1(fs);  // fs moved here
    do_thing_2(fs);  // ERROR: fs was moved on line above
}
```

## Semantics

### Move Semantics

Using an affine value *moves* it. After a move, the binding is no longer valid.
```strata
fn example(fs: FsCap) -> () & {Fs} {
    let a = fs;       // fs moved to a
    let b = fs;       // ERROR: use of moved value `fs`
    
    use_cap(a);       // a moved into function
    use_cap(a);       // ERROR: use of moved value `a`
}
```

### Affine Values Can Be Dropped

Unlike linear types (use-exactly-once), affine values can go out of scope unused:
```strata
fn drop_ok(fs: FsCap) -> () & {} {
    // fs is never used — that's fine (affine, not linear)
    ()
}
```

### Return Transfers Ownership

Returning an affine value transfers ownership to the caller:
```strata
fn passthrough(fs: FsCap) -> FsCap & {} {
    fs  // ownership transferred to caller
}

fn caller(fs: FsCap) -> () & {Fs} {
    let fs2 = passthrough(fs);  // fs moved in, fs2 received
    use_cap(fs2);  // valid
}
```

### Branching

All branches must agree on what's moved:
```strata
fn branch_ok(fs: FsCap, cond: Bool) -> () & {Fs} {
    if cond {
        use_cap(fs);  // fs moved
    } else {
        use_cap(fs);  // fs moved (same binding, both branches)
    }
    // fs is moved regardless of which branch
}

fn branch_bad(fs: FsCap, cond: Bool) -> () & {Fs} {
    if cond {
        use_cap(fs);  // fs moved
    }
    // ERROR: fs may or may not be moved (inconsistent)
    use_cap(fs);
}
```

### Loops

Affine values cannot be used inside loops (would require multiple uses):
```strata
fn loop_bad(fs: FsCap) -> () & {Fs} {
    while true {
        use_cap(fs);  // ERROR: fs would be moved multiple times
    }
}
```

### Closures (v0.1: Banned)

For v0.1, affine values **cannot be captured by closures**:
```strata
fn closure_bad(fs: FsCap) -> () & {} {
    let f = || use_cap(fs);  // ERROR: cannot capture affine value in closure
    f();
}
```

This is the simplest safe design. v0.2 may allow affine closures (closures that capture affine values become affine themselves).

## Type System Changes

### Kind Tracking
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Unrestricted,  // *
    Affine,        // !
}

impl Ty {
    fn kind(&self) -> Kind {
        match self {
            Ty::Cap(_) => Kind::Affine,
            // All other types are unrestricted for now
            _ => Kind::Unrestricted,
        }
    }
}
```

### Move Tracking in Checker

The checker maintains a set of "moved" bindings:
```rust
struct CheckContext {
    // ... existing fields ...
    moved: HashSet<String>,  // bindings that have been moved
}
```

When a variable is used:
1. If it's in `moved`, emit error: "use of moved value"
2. If its type is affine, add it to `moved`

### Unification with Kinds

When unifying types, kinds must be compatible:
- `*` unifies with `*`
- `!` unifies with `!`
- `*` does not unify with `!` (prevents treating affine as unrestricted)

For type variables, track kind constraints:
- Fresh type var defaults to `*`
- If unified with affine type, constrained to `!`
- If later unified with unrestricted type, error

## Error Messages
```
Error: use of moved value `fs`

  --> src/main.strata:5:5
   |
 4 |     use_cap(fs);
   |             -- value moved here
 5 |     use_cap(fs);
   |             ^^ value used after move
   |
   = note: `FsCap` is an affine type and can only be used once
   = help: consider restructuring to use the capability only once
```
```
Error: cannot capture affine value in closure

  --> src/main.strata:3:14
   |
 3 |     let f = || use_cap(fs);
   |              ^^^^^^^^^^^^^ closure captures `fs`
   |
   = note: `FsCap` is an affine type and cannot be captured
   = help: pass the capability as a parameter instead
```
```
Error: affine value may not be moved in all branches

  --> src/main.strata:7:5
   |
 4 |     if cond {
 5 |         use_cap(fs);  // moved here
   |                 --
 6 |     }
 7 |     use_cap(fs);  // but might not be moved
   |             ^^ value may or may not be valid here
   |
   = help: ensure all branches either use or don't use `fs`
```

## Definition of Done

- [ ] Kind system: `Kind::Unrestricted` and `Kind::Affine`
- [ ] Ty::kind() method returning appropriate kind
- [ ] CheckContext: track moved bindings
- [ ] Checker: error on use-after-move for affine values
- [ ] Checker: error on capturing affine values in closures
- [ ] Checker: validate branch consistency for affine values
- [ ] Checker: error on affine values in loops
- [ ] Inference: kind constraints on type variables
- [ ] Error messages: clear, actionable, with help text
- [ ] Tests: basic move semantics
- [ ] Tests: use-after-move errors
- [ ] Tests: branch consistency
- [ ] Tests: loop errors
- [ ] Tests: closure capture errors
- [ ] Tests: return transfers ownership
- [ ] Tests: drop is allowed (affine, not linear)
- [ ] Documentation: Update ADR for affine types

## Test Cases
```strata
// PASS: Single use
fn single_use(fs: FsCap) -> () & {Fs} {
    use_cap(fs);
}

// PASS: Unused (drop is ok)
fn unused(fs: FsCap) -> () & {} {
    ()
}

// PASS: Return transfers ownership
fn passthrough(fs: FsCap) -> FsCap & {} {
    fs
}

// FAIL: Double use
fn double_use(fs: FsCap) -> () & {Fs} {
    use_cap(fs);
    use_cap(fs);  // ERROR: moved
}

// FAIL: Use after binding move
fn rebind(fs: FsCap) -> () & {Fs} {
    let a = fs;
    use_cap(fs);  // ERROR: moved to a
}

// PASS: Consistent branches
fn branch_consistent(fs: FsCap, c: Bool) -> () & {Fs} {
    if c { use_cap(fs); } else { use_cap(fs); }
}

// FAIL: Inconsistent branches
fn branch_inconsistent(fs: FsCap, c: Bool) -> () & {Fs} {
    if c { use_cap(fs); }
    use_cap(fs);  // ERROR: may be moved
}

// FAIL: Loop
fn in_loop(fs: FsCap) -> () & {Fs} {
    while true { use_cap(fs); }  // ERROR: multiple moves
}

// FAIL: Closure capture
fn closure_cap(fs: FsCap) -> () & {} {
    let f = || use_cap(fs);  // ERROR: cannot capture
}

// PASS: Unrestricted types work normally
fn normal(x: Int) -> Int & {} {
    let a = x;
    let b = x;  // Fine, Int is unrestricted
    a + b
}
```

## v0.2 Considerations (Documented Now)

1. **Affine closures:** If a closure captures affine values, the closure itself becomes affine. Requires propagating kind through closure types.

2. **Borrowing:** `&FsCap` as a way to "use" a capability without consuming it. Requires lifetime tracking.

3. **Effect handlers with continuations:** If continuations capture affine values, continuations must be affine (use-at-most-once). Multi-shot continuations are incompatible with affine captures.

## Security Rationale

Affine types provide:
- **No duplication:** Capabilities cannot be copied
- **No ambient pools:** Combined with ADT ban, no global capability storage
- **Explicit flow:** Capabilities must be threaded through call chains
- **Auditability:** Capability usage is visible in function signatures

This achieves "capabilities cannot be forged, cloned, or leaked" for v0.1.