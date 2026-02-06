# Issue 009: Capability Types

**Status:** Complete
**Estimated time:** 5-7 days
**Phase:** 3 (Effect System)
**Depends on:** Issue 008 (Effect System)
**Blocks:** Issue 010 (Affine Types)

## Goal

Introduce capability types that gate effect usage. A function can only perform an effect if the corresponding capability is in scope. This enforces "no ambient authority" — capabilities must be explicitly passed, not conjured from thin air.

## The Problem We're Solving

With Issue 008, we have effect tracking:
```strata
fn read_config() -> String & {Fs} {
    read_file("config.toml")  // Compiler knows this does Fs
}
```

But there's no *enforcement*. Anyone can call `read_file`. The effect system tracks what happens but doesn't prevent it.

With capabilities:
```strata
fn read_config(fs: FsCap) -> String & {Fs} {
    read_file("config.toml", fs)  // Must have capability to use effect
}
```

Now `read_file` requires a `FsCap` to call. If you don't have one, you can't perform filesystem effects.

## Syntax

### Capability Types
```strata
// Built-in capability types (opaque, cannot be constructed by user code)
FsCap    // Gates {Fs} effects
NetCap   // Gates {Net} effects  
TimeCap  // Gates {Time} effects
RandCap  // Gates {Rand} effects
AiCap    // Gates {Ai} effects
```

### Functions Requiring Capabilities
```strata
// Capability as explicit parameter
fn read_file(path: String, fs: FsCap) -> String & {Fs} {
    // ...
}

// Multiple capabilities
fn fetch_and_save(url: String, fs: FsCap, net: NetCap) -> () & {Fs, Net} {
    let data = http_get(url, net);
    write_file("cache.txt", data, fs)
}
```

### Entry Point
```strata
// main receives capabilities from the runtime
fn main(fs: FsCap, net: NetCap, time: TimeCap) -> () & {Fs, Net, Time} {
    let config = read_config(fs);
    // ...
}
```

Capabilities flow down from `main`. There is no other way to obtain them.

## Scope

### In Scope (v0.1)
- Five capability types: `FsCap`, `NetCap`, `TimeCap`, `RandCap`, `AiCap`
- Capabilities as function parameters
- Compiler enforces: effect `{X}` requires corresponding `XCap` in scope
- Error messages when capability is missing
- Capabilities cannot be stored in ADTs (already enforced by design)

### Out of Scope (v0.2+)
- Capability attenuation (restricting a capability to subset of operations)
- Capability revocation
- Sub-capabilities (e.g., `ReadOnlyFsCap`)
- Capability delegation patterns

## Type System Changes

### New Types
```rust
// In Ty enum
Ty::Cap(CapKind)

enum CapKind {
    Fs,
    Net,
    Time,
    Rand,
    Ai,
}
```

### Capability-Effect Mapping
```rust
impl CapKind {
    fn gates_effect(&self) -> Effect {
        match self {
            CapKind::Fs => Effect::Fs,
            CapKind::Net => Effect::Net,
            CapKind::Time => Effect::Time,
            CapKind::Rand => Effect::Rand,
            CapKind::Ai => Effect::Ai,
        }
    }
}
```

### Checking Rule

When checking a function body with declared effects `& {E1, E2, ...}`:
1. Collect all capability parameters from the function signature
2. For each effect `Ei` in the declared effects, verify a corresponding capability is in scope
3. If effect `Ei` is used but no `EiCap` is in scope, emit error
```
Error: Effect {Fs} requires FsCap capability

  --> src/main.strata:12:5
   |
12 |     read_file("config.toml")
   |     ^^^^^^^^^^^^^^^^^^^^^^^^ this performs {Fs} effect
   |
   = help: add `fs: FsCap` parameter to this function
```

## Extern Functions and Capabilities

Extern functions that perform effects must require the corresponding capability:
```strata
// Valid: capability parameter matches effect
extern fn read_file(path: String, fs: FsCap) -> String & {Fs};

// Invalid: performs {Fs} without requiring FsCap
extern fn read_file(path: String) -> String & {Fs};  // ERROR
```

The checker enforces: for extern functions, every declared effect must have a matching capability parameter.

## Definition of Done

- [ ] AST: Capability type syntax (`FsCap`, etc.)
- [ ] Parser: Parse capability types in type positions
- [ ] Type system: `Ty::Cap(CapKind)` variant
- [ ] Checker: Enforce capability-effect correspondence
- [ ] Checker: Extern functions must have capability params for their effects
- [ ] Error messages: Clear "missing capability" errors with help text
- [ ] Tests: Positive cases (capability present, effect allowed)
- [ ] Tests: Negative cases (capability missing, error emitted)
- [ ] Tests: Extern function validation
- [ ] Tests: Multiple capabilities in one function
- [ ] Documentation: Update ADR for capability design

## Test Cases
```strata
// PASS: Capability matches effect
fn good(fs: FsCap) -> () & {Fs} {
    extern fn do_fs(fs: FsCap) -> () & {Fs};
    do_fs(fs)
}

// FAIL: Effect without capability
fn bad() -> () & {Fs} {
    extern fn do_fs(fs: FsCap) -> () & {Fs};
    do_fs(/* no capability! */)  // Error: missing FsCap
}

// PASS: Pure function needs no capabilities
fn pure(x: Int) -> Int & {} {
    x + 1
}

// PASS: Multiple capabilities
fn multi(fs: FsCap, net: NetCap) -> () & {Fs, Net} {
    // ...
}

// FAIL: Extern with effect but no capability param
extern fn bad_extern(path: String) -> String & {Fs};  // Error
```

## Security Considerations

- Capabilities are opaque types — no constructor syntax
- Capabilities can only enter the program via `main` parameters
- Capabilities cannot be stored in ADTs (prevents ambient capability pools)
- Issue 010 will add affine types to prevent capability duplication

## Open Questions (Resolved)

1. **Can capabilities be returned from functions?** Yes, for now. Issue 010 will add move semantics.
2. **Can capabilities be in tuples?** No — same restriction as ADTs.
3. **Capability syntax: `fs: FsCap` vs `using fs: FsCap`?** Using plain parameter syntax for v0.1. `using` syntax is a v0.2 consideration for ergonomics.

## Known Limitations Discovered

- **Arrow type effect annotations silently dropped:** Effect annotations on arrow types in type expressions (e.g., `fn(Int) -> Int & {Fs}` in an ADT field) are parsed but silently dropped by the type checker, defaulting the slot to pure — sound but a UX trap; discovered during adversarial security testing.