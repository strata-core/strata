# Strata Design Decisions

> Key technical and architectural choices that guide implementation  
> Updated: February 1, 2026

This document captures important design decisions and their rationale. These are the choices that inform implementation but often don't make it into user-facing docs.

For detailed analysis of major decisions, see the ADR (Architecture Decision Records) files:
- [ADR-0001: Effect Rows Are Set-like, Orderless](adr-0001.md)
- [ADR-0002: Hierarchical Capability Bundles](adr-0002.md)
- [ADR-0003: Multi-Parameter Functions Over Currying](adr-0003.md)

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

### Multi-Parameter Functions Over Currying

**Decision:** Use multi-parameter syntax familiar to ops teams.

**Syntax:**
```strata
fn add(x: Int, y: Int) -> Int {
  x + y
}
```

**Type Signature:**
```strata
add: (Int, Int) -> Int
```

**Rationale:**
- Target audience (ops teams) expects Python/Go/Rust style multi-param syntax
- Arity mismatch errors are clearer: "Expected 2 args, got 1" vs silent partial application
- Capability passing is more visually distinct
- Partial application can be explicit with lambdas when needed

**See:** [ADR-0003](adr-0003.md) for full analysis and alternatives considered.

**Status:** Locked in - implemented in Issue 005

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

**See:** [ADR-0001](adr-0001.md) for detailed specification.

**Status:** Implemented in Issue 002 (`strata-types/src/effects.rs`)

---

### Effects Are Grouped, Not Granular (MVP)

**Decision:** MVP uses coarse-grained effects.

**Groups:**
- `Net` - All network operations (HTTP, TCP, DNS, WebSockets)
- `FS` - All filesystem operations (read, write, delete, metadata)
- `Time` - All time operations (clock, sleep, timeout)
- `Rand` - All randomness (secure RNG, fast RNG)
- `Fail` - Failure/exception raising (will be parameterized later: `Fail[E]`)

**Rationale:**
- Simpler to implement and reason about
- Avoids effect explosion (don't need 50+ effect types)
- Sufficient for MVP auditing goals
- Can refine later if production needs demand it

**Example of what we're NOT doing (yet):**
```strata
// Don't split Net into sub-effects in MVP
& {Http, Tcp, Dns, WebSocket}  // ❌ Too granular

// Use coarse-grained instead
& {Net}  // ✅ Simple, clear
```

**Future:** Could add sub-effect taxonomy in v0.2+ if there's demand. For example, read-only vs read-write filesystem access, or secure vs fast randomness.

**Status:** Implemented, working well

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

### Hierarchical Capability Bundles

**Decision:** Support RBAC-style capability bundles to manage complex permission sets.

**Problem:** Real-world functions need many capabilities. Writing them individually is verbose and unmaintainable.

**Solution:** Named capability bundles with hierarchical sub-roles.

**Example:**
```strata
// Define reusable role
capability IncidentResponder = {
  GitHub: {Read, Comment, Issues},
  Slack: {Read, Write},
  K8s: {Read, Logs},
  PagerDuty: {Trigger, Acknowledge}
}

// Define specialized sub-role
capability IncidentResponder.Scribe = {
  GitHub: {Comment, Issues},
  Slack: {Write}
  // Can document but can't investigate or escalate
}

// Use in function
fn handle_incident(
  alert: Alert,
  using cap: IncidentResponder
) -> Result<Resolution, Error> & {Net, GitHub, Slack, K8s, PagerDuty}
```

**Tooling support:**
```fish
strata describe role IncidentResponder
# Shows all capabilities, sub-roles, usage
```

**See:** [ADR-0002](adr-0002.md) for full design including compiler warnings for over-permission.

**Status:** Planned for v0.2 (after fine-grained capabilities work in v0.1)

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
- Multi-parameter functions (not curried)

**Rationale:**
- Unambiguous parsing
- Approachable to mainstream devs (Python, Go, Rust backgrounds)
- Easy to read during audits
- Familiar to target audience (ops teams, not FP researchers)

**Design Philosophy:**
> "Strata targets automation practitioners who need safe, auditable systems. Syntax should be boring, familiar, and clear—not theoretically elegant but practically confusing."

---

## Profiles & Constraints

### Execution Profiles

**Decision:** Three execution profiles with different effect/capability constraints.

**Profiles:**

1. **Kernel Profile**
   - **Allowed effects:** None (pure computation only)
   - **No:** Net, FS, Time, Rand
   - **No:** Dynamic allocation (arena-based only)
   - **Guarantees:** Deterministic, bounded memory, no I/O
   - **Use case:** Core algorithms, deterministic replay, embedded systems

2. **Realtime Profile**
   - **Allowed effects:** Limited Time, seeded Rand
   - **Constrained:** Net and FS (bounded latency operations only)
   - **No:** Garbage collection
   - **Guarantees:** Bounded latency, backpressure, predictable timing
   - **Use case:** Time-sensitive automation, telemetry processing

3. **General Profile** (default)
   - **Allowed effects:** All effects within capability model
   - **No constraints:** Full language features available
   - **Use case:** Standard automation scripts, agent orchestration

**Enforcement:**
```strata
// Declared at module or package level
profile Kernel;

fn compute(x: Int, y: Int) -> Int {
  x + y  // OK - pure computation
}

fn bad_fetch(url: Url) -> String & {Net} {
  // Compile error: Net effect not allowed in Kernel profile
}
```

**Status:** Types exist, enforcement layer planned for v0.2

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

## Logic Engine & Explainability

### Datalog-Lite Integration

**Decision:** Integrate logic engine as library first, syntax sugar later.

**Capabilities:**
- Facts and rules (Datalog-like)
- Returns both query results AND proof traces (derivation trees)
- Pure and side-effect free
- Integrates with host types (no separate type system)

**Use case:**
```strata
// Define facts and rules
logic SecurityPolicy {
  fact: can_access(User, Resource)
  rule: can_access(u, r) :- has_role(u, "admin")
  rule: can_access(u, r) :- owns(u, r)
}

// Query with proof trace
let (result, proof) = query(can_access(alice, prod_db));
if result {
  log_audit("Access granted", proof);  // Proof explains WHY
} else {
  log_audit("Access denied", proof);   // Proof explains WHY NOT
}
```

**Goal:** AI agent decisions can be explained via proof traces showing the reasoning chain.

**Status:** Library planned for v0.2, compiler syntax sugar v0.3+

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

### Don't: Automatic Currying

**What was considered:** Haskell-style curried functions where `fn(a, b)` is really `fn(a) -> fn(b)`.

**Why rejected:** Unfamiliar to target audience (ops teams), partial application errors are confusing, type signatures harder to read.

**See:** [ADR-0003](adr-0003.md) for full rationale.

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

For major decisions that need extensive analysis, create an ADR file instead:
- Name: `adr-NNNN.md` where NNNN is a zero-padded number
- Link from this doc: `**See:** [ADR-NNNN](adr-NNNN.md)`
- Keep this doc focused, use ADRs for deep dives
