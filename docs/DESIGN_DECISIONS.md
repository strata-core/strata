# Strata Design Decisions

> Key technical and architectural choices that guide implementation  
> Updated: January 30, 2026

This document captures important design decisions and their rationale. These are the choices that inform implementation but often don't make it into user-facing docs.

---

## Type System

### No Classes - Use Traits + ADTs

**Decision:** Strata does NOT have classes or classical inheritance.

**Instead:**
- Structs for data
- Enums for sum types
- Traits for polymorphism
- Composition over inheritance

**Rationale:**
- Classes encourage ambient authority (`this.openFile()`)
- Inheritance complicates the capability model
- Traits provide sufficient abstraction without complexity
- Proven approach (Rust, Haskell, OCaml)

**Status:** Locked in - do not drift toward Java-style classes.

---

### Hindley-Milner Core with Explicit Boundaries

**Decision:** Local inference inside functions, explicit types at module boundaries.

**Specifics:**
- Local `let` bindings → inferred
- `pub` functions → require type + effect annotations
- Internal expressions → fully inferred
- Cross-module → no inference leakage

**Rationale:**
- Keeps ergonomics good inside functions
- Preserves auditability at API boundaries
- Stable cross-module interfaces
- Clear mental model: "explicit at boundaries, inferred locally"

**Implementation:**
- Issue 004: Direct type checking (no polymorphism yet)
- Issue 005: Constraint-based inference with generalization
- Later: Let-polymorphism

---

### Effect Rows Are Sets, Not Lists

**Decision:** Effect rows are unordered, canonical sets.

**Properties:**
- `{Net, FS}` == `{FS, Net}` (order irrelevant)
- `{Net, Net}` == `{Net}` (no duplicates)
- Operations: subset, union, intersection
- Representation: canonical sorted set

**Rationale:**
- Effects don't have ordering semantics
- Simplifies checking (subset test)
- Matches mathematical intuition
- Easier to optimize

**Deferred:** Row polymorphism (effect variables like `E`)

**Status:** Implemented in Issue 002 (`strata-types/src/effects.rs`)

---

### Effects Are Grouped, Not Granular (MVP)

**Decision:** MVP uses coarse-grained effects.

**Groups:**
- `Net` (not `DNS`, `TCP`, `HTTP` separately)
- `FS` (not `Read`, `Write`, `Delete`)
- `Time` (not `Clock`, `Sleep`, `Timeout`)
- `Rand` (not `SecureRng`, `FastRng`)

**Rationale:**
- Simpler to implement and reason about
- Avoids effect explosion
- Sufficient for MVP auditing goals
- Can refine later if needed

**Future:** Could add sub-effect taxonomy in v0.2+

---

## Syntax Choices

### Effect Syntax: `& {Net, FS}`

**Decision:** Effects appear as `& {Effect1, Effect2}` after return type.

**Example:**
```strata
fn read_file(path: String) -> Result<String, IoErr> & {FS}
```

**Rationale:**
- Visually separates "what" (return type) from "how" (effects)
- Reads as annotation/refinement, not wrapping
- Keeps Rust-adjacent readability ("boring but clear")
- Better for composition than monadic encoding

**Alternatives rejected:**
- `IO<T>` style (too heavyweight, hides effects)
- `@effects(Net, FS)` (less composable with type system)

---

### Capability Syntax: `using cap: Type`

**Decision:** Capabilities passed with `using` keyword.

**Example:**
```strata
fn http_get(url: Url, using net: NetCap) -> Result<Bytes, NetErr> & {Net}
```

**Call site:**
```strata
http_get(url, using net)?
```

**Rationale:**
- Makes authority explicit and greppable
- Reads naturally ("use this capability")
- Avoids hidden global context
- Audit-friendly (can see all capability uses)

**Alternatives rejected:**
- Implicit/ambient capabilities
- Dependency injection magic
- Global runtime context

---

### `layer` Blocks (Sugar Only)

**Decision:** `layer` is syntactic sugar, nothing more.

**Example:**
```strata
layer Analytics(using net: NetCap, fs: FsCap) & {Net, FS} {
    let data = http_get(url, using net)?;
    write_file("/tmp/out.txt", data, using fs)?;
}
```

**Compiles to:** Explicit capability passing and effect checking.

**Rationale:**
- Matches "layers" metaphor (geology theme)
- Creates clean scoping
- Good pedagogy and diagnostics
- But: NO semantic magic, just desugaring

**Status:** Syntax planned, not yet implemented.

---

### "Boring But Clear" Style

**Decision:** Syntax prioritizes clarity over cleverness.

**Specifics:**
- Braces required `{ ... }`
- Semicolons optional
- Method chaining canonical, `|>` optional
- Rust/Swift/Kotlin-adjacent readability

**Rationale:**
- Unambiguous parsing
- Approachable to mainstream devs
- Easy to read during audits
- Familiar to target audience

---

## Architecture

### No Crate Cycles

**Decision:** Strict DAG of crate dependencies.

**Order:**
```
strata-ast → strata-parse → strata-types → (strata-ir, strata-vm, strata-codegen)
```

**Rationale:**
- Clean compilation order
- Easier to test in isolation
- Forces good boundaries
- Prevents circular complexity

**Enforcement:** Review PRs for new dependencies.

---

### Parse/Type Bridge in Parse Crate

**Decision:** AST → Constraints mapping lives in `strata-parse`, not `strata-types`.

**Rationale:**
- Avoids crate cycle (parse would need AST, types would need parse)
- Keeps `strata-types` focused on type theory
- Bridge can grow into elaboration/lowering later

**Future:** May extract to `strata-elaborate` crate if it grows large.

---

### "Kernel First" Principle

**Decision:** Anything requiring I/O belongs in runtime crates, not core.

**Implication:**
- Parser: pure, no I/O
- Types: pure, no I/O
- VM/Runtime: handles I/O, effects, capabilities

**Rationale:**
- Enables Kernel profile (no I/O)
- Makes deterministic testing easier
- Clear separation of concerns

---

## Open Questions

These are design questions still under consideration:

### 1. Async/Await + Effects

**Question:** How do async functions interact with effects?

**Options:**
- Treat `await` as an effect?
- Async functions carry effects separately?
- Implement via algebraic effects/handlers?
- Separate lowering pass?

**Status:** Deferred to v0.2

---

### 2. Logic Engine Integration Depth

**Question:** Is the Datalog engine a library or a language sublanguage?

**Options:**
- Library crate (Issue 012+)
- Compiler-aware with `logic { ... }` syntax
- Proof trace format stability

**Status:** Library first, syntax later

---

### 3. Trait Coherence Rules

**Question:** How strict are trait impl rules?

**Options:**
- Orphan rules (like Rust)?
- Module-level coherence?
- Global coherence?
- Allow specialization?

**Status:** Probably Rust-like orphan rules, no specialization in v0.1

---

### 4. Row Polymorphism Timing

**Question:** When do we add effect variables `E`?

**Example:**
```strata
fn map<T, U, E>(f: fn(T) -> U & E, xs: [T]) -> [U] & E
```

**Status:** Not in Issue 004-007, probably Issue 008+

---

## False Starts / What NOT To Do

### Don't: Rust Macro Bootstrap Path

**What was considered:** Building Strata as Rust procedural macros, transpiling to Rust.

**Why rejected:** Native compiler is the canonical path. Don't accidentally re-architect around macros.

---

### Don't: Hyper-Granular Effects

**What was considered:** Splitting effects into fine-grained variants (HTTP, DNS, TCP for Net).

**Why rejected:** Effect explosion, complexity, MVP doesn't need it.

---

### Don't: "Experimental" Directory

**What was suggested:** `experimental/` for parallel Rust-macro path.

**Why rejected:** Implies dual implementation. Don't add scaffolding for paths you're not taking.

---

## Decision Log Format

When adding new decisions, use this format:

```markdown
### [Decision Title]

**Decision:** [What was decided]

**Rationale:** [Why]

**Alternatives rejected:** [What else was considered]

**Status:** [Locked in / Under review / Deferred]

**Date:** [When decided]
```
