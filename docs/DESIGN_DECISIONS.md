# Strata Design Decisions

> Key technical and architectural choices that guide implementation  
> Updated: February 3, 2026

This document captures Strata's design philosophy and specific technical decisions. These guide implementation and explain "why Strata is this way."

For detailed analysis of major decisions, see the ADR (Architecture Decision Records) files:
- [ADR-0001: Effect Rows Are Set-like, Orderless](adr-0001.md)
- [ADR-0002: Hierarchical Capability Bundles](adr-0002.md)
- [ADR-0003: Multi-Parameter Functions Over Currying](adr-0003.md)

---

## Design Philosophy & Core Principles

Strata is an **anti-single-axis language** that refuses to sacrifice any core pillar for the sake of others. This section explains the philosophy that guides all design decisions.

### The Anti-Single-Axis Stance

Most programming languages optimize for ONE primary goal:
- Python: Ease of use
- Rust: Memory safety + performance
- Go: Simplicity
- Haskell: Correctness + purity

**Strata refuses to pick one.** Instead, it treats these as **equally important pillars**:

1. **Type Soundness** - Compile-time safety guarantees
2. **Operational Reality** - Explicit handling of failure, concurrency, and effects
3. **Human Factors** - Preventing misuse, enabling debugging, maintaining clarity
4. **Security** - Making authority boundaries explicit and enforceable
5. **Long-term Maintainability** - Audit trails, governance, and explainability

### Why This Matters

Modern automation runs critical infrastructure:
- AI agents making autonomous decisions
- Deployment scripts touching production systems
- Security playbooks responding to incidents
- ML pipelines processing sensitive data

**When these fail, the consequences are real.** Not just "program crashed" but:
- Financial losses
- Security breaches
- Compliance violations
- Patient safety risks

**Strata treats automation as a governed act, not just computation.**

### The 8 Design Guardrails

These principles prevent Strata from falling into traps that killed similar languages (Pony, Oz, etc.). For full context and historical analysis, see the private repo `strata-strategy/docs/design-principles.md`.

#### 1. Power Must Be Visible

**Principle:** Any action with real-world consequences must be explicit in the code.

**Why:** Hidden authority is a security and maintainability disaster. If you can't see what code can do by reading it, you can't audit it, review it, or debug it.

**In practice:**
```strata
// ❌ BAD: Implicit authority
fn deploy(model: Model) {
    upload(model);  // Where did network access come from?
}

// ✅ GOOD: Explicit authority
fn deploy(model: Model, using net: NetCap) 
    -> Result<Deployment> & {Net} {
    upload(model, using net)
}
```

#### 2. Inference Reduces Syntax, Never Meaning

**Principle:** Type inference is valuable, but never infer effects or authority.

**Why:** When debugging at 3am, you need to understand what code does without consulting compiler internals. Clever inference that hides meaning is a debugging nightmare.

**In practice:**
- ✅ Infer expression types: `let x = 5` (Int inferred)
- ❌ Don't infer effects: `& {FS, Net}` must be explicit
- ❌ Don't infer capabilities: `using cap: Cap` must be explicit

#### 3. Failure Story Before Success Story

**Principle:** Every language feature must define its failure behavior before we implement the happy path.

**Why:** Systems are defined by how they fail, not how they succeed. Postponing failure handling leads to undefined behavior in production.

**In practice:**
- All functions return `Result<T, E>`, not raw values
- Effect traces capture failures
- Deterministic replay for debugging
- No exceptions (explicit error handling only)

#### 4. Legible to Competent Engineers

**Principle:** Code must be understandable by a competent engineer who didn't design the language.

**Why:** Systems are maintained by tired people, new hires, and stressed responders. If correct usage requires deep language knowledge or folklore, the language has failed.

**In practice:**
- Clear, self-explanatory syntax
- Helpful error messages
- Minimal "magic"
- No hidden conventions

#### 5. Governed, Not Forbidden

**Principle:** Dangerous operations should be tracked, not eliminated.

**Why:** Real systems need dangerous operations (shell commands, network calls, file writes). The question is whether they're visible, bounded, and auditable.

**In practice:**
```strata
// ✅ Shell commands allowed, but governed
fn run_command(
    cmd: String,
    using shell: ShellCap,
    using audit: AuditCap
) -> Result<o> & {Shell, Audit} {
    // Explicit capability required
    // Effect tracked
    // Logged to audit trail
}
```

#### 6. Composition Preserves Responsibility

**Principle:** When safe components compose, the aggregate effect must be visible and require appropriate authority.

**Why:** Composition is where intent dissolves. If five safe steps combine to do something unsafe, the language must surface that escalation.

**In practice:**
```strata
fn workflow(
    using fs: FsCap,
    using net: NetCap,
    using db: DbCap
) -> Result<o> & {FS, Net, DB} {  // All effects visible
    step1(using fs)?;
    step2(using net)?;
    step3(using db)?;
}
```

#### 7. Integrity Over Adoption

**Principle:** The first users should be people who want the constraints.

**Why:** Early adopters define your culture forever. If we compromise core safety for easier adoption, we've already lost.

**In practice:**
- No `@unsafe` escape hatches in v0.1
- No "skip effect checking" flags
- Performance optimization is v0.2+, not v0.1
- Target users who need audit trails, not users who want Python-style freedom

#### 8. Encode Ethics Into Mechanics

**Principle:** Safety must be enforced by the type system, not conventions or documentation.

**Why:** Languages outlive their creators. If Strata only works because of careful human discipline, it will fail when those humans leave.

**In practice:**
- Capabilities enforced by compiler
- Effects tracked automatically
- Can't bypass safety without explicit FFI
- No reliance on "best practices"

### Design Mantras

Short principles to internalize:

- **On Safety:** "Power should be visible, not eliminated."
- **On Failure:** "Failure story before success story."
- **On Inference:** "Inference may reduce syntax, never meaning."
- **On Humans:** "Legible to a competent engineer who didn't design it."
- **On Culture:** "Early adopters define culture forever."
- **On Time:** "Encode ethics into mechanics."

### When to Apply These Principles

**Always reference for:**
- Designing major features (generics, effects, actors)
- Architectural decisions
- Scope additions or cuts

**Don't overthink for:**
- Straightforward implementation (parser, simple types)
- Obvious code that doesn't affect architecture
- Trivial decisions

**The goal:** Refuse compromise on core principles, but don't let philosophy paralyze development.

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

**See:** [ADR-0003](adr/adr-0003.md) for full analysis and alternatives considered.

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

**See:** [ADR-0001](adr/adr-0001.md) for detailed specification.

**Status:** Implemented in Issue 002 (`strata-types/src/effects.rs`)

---

### Capability Types Gate Effects (Issue 009)

**Decision:** Each effect has a corresponding capability type. A function that performs concrete effects must have the matching capability type(s) in its parameter list.

**Mapping:**
- `FsCap` gates `{Fs}` effects
- `NetCap` gates `{Net}` effects
- `TimeCap` gates `{Time}` effects
- `RandCap` gates `{Rand}` effects
- `AiCap` gates `{Ai}` effects

**Design choices:**
- **`Ty::Cap(CapKind)` is a first-class leaf type**, not an ADT. This prevents capabilities from being confused with user-defined types.
- **Validation is a separate pass** after effect inference. The effect solver remains clean; capability checking is additive.
- **Only concrete effects are checked.** Open tail variables (from higher-order function callbacks) don't require capabilities at the function level.
- **Unused capabilities are not errors.** Functions may accept capabilities to pass downstream (pass-through pattern).
- **Capability check activates when a function has at least one Cap-typed parameter.** Pre-capability code (without any Cap params) continues to work.

**Known limitation:** Capability provisioning (how `main` gets capabilities from the runtime) is deferred to Issue 011 (WASM Runtime).

**Status:** Implemented in Issue 009 (`strata-types/src/effects.rs`, `strata-types/src/checker.rs`)

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

### Affine Types for Capabilities (Issue 010)

**Decision:** Capabilities are affine (use at most once). Enforcement is a post-inference pass, not a modification to unification.

**Kind system:**
- `Kind::Unrestricted` — standard types (Int, String, Bool, ADTs, etc.)
- `Kind::Affine` — capability types (FsCap, NetCap, TimeCap, RandCap, AiCap)
- Kind is intrinsic to the type — no user annotation needed

**Move checker design:**
- Separate pass that runs after type inference and constraint solving
- Does NOT modify unification, constraint generation, or the solver
- Tracks each affine binding with a generation-based ID (handles shadowing correctly)
- Pessimistic branch join: if consumed in ANY branch (if/else/match), treat as consumed afterward
- Loop rejection: capability use inside while body is always an error
- Let-bindings transfer ownership: `let a = fs;` consumes `fs`, `a` becomes alive

**Why a separate pass (not in the unifier):**
- Type inference should remain purely about types — mixing linearity into unification creates complexity
- Separate pass is easier to reason about, test, and extend
- Same approach used by Rust (borrow checker is a separate pass from type inference)
- Error messages are clearer when move checking has its own error types

**Why "permission/authority" vocabulary (not "moved value"):**
- Target audience is SRE/DevOps, not PL researchers
- "Capability 'fs' has already been used" is more intuitive than "use of moved value"
- "Permission was transferred" maps to mental model of authority delegation

**Why affine (not linear):**
- Affine = at most once (can drop without using). Linear = exactly once (must use).
- Forcing use of every capability would require complex "must-use" tracking
- Dropping a capability is safe — it just means the authority wasn't exercised
- Matches real-world intuition: having a key you never use is fine

**Pessimistic branch join:**
- If a capability is consumed in ANY branch (if/else/match), treat as consumed after the branch
- Conservative but safe — the alternative (tracking which branch was taken) requires runtime info
- Consistent with Rust's approach to conditionally-moved values

**Generation-based binding IDs for shadowing:**
- Each `let x = ...;` gets a unique generation counter, so `BindingId = (name, generation)`
- Shadowing creates a new generation: `let fs = fs;` → old `fs` consumed (gen 1), new `fs` alive (gen 2)
- Type lookup keyed by BindingId (not String) so shadowing can't corrupt type resolution

**Left-to-right evaluation order for arguments:**
- `f(a, b, c)` evaluates `a` then `b` then `c` with cumulative move state
- `f(fs, fs)` correctly fails: first arg consumes `fs`, second arg sees it consumed
- Deterministic and matches user expectations from reading left-to-right

**Error vocabulary: permission/transfer/single-use (not Rust's moved/borrow/affine):**
- Target audience is SRE/DevOps, not PL researchers
- "capability 'fs' has already been used; permission was transferred" is clearer than "use of moved value"
- Consistent with Strata's "governed, not forbidden" philosophy

**Status:** Implemented, working.

---

## Strategic Decisions

### S1: Durable Execution is a Runtime Product, Not a Language Feature
Strata's type system enables durable execution (no ambient authority = serializable state),
but the checkpoint/resume machinery belongs in `strata-rt`, not `strata-types`.

### S2: Effect Virtualization Before Durable Execution
Handlers enable testing, sandboxing, and dry-run. Durable execution depends on
virtualization to handle world-state differences on resume. Build the foundation first.

### S3: `strata plan` is the v0.1 Killer Demo
Static effect/capability analysis without execution. "Terraform plan for automation."
No new language features required — walks the typed AST. Ships as Issue 014.

### S4: Capability Attenuation Uses Wrapper Model, Not Subtyping
Consuming broad cap and producing narrow cap via proxy. Does not require changes to
type inference.

### S5: Reason Tracing Must Be Structured
Free-form strings become "needed for deployment" everywhere. Require structured
reason types. API first, syntax sugar later.

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

**See:** [ADR-0002](adr/adr-0002.md) for full design including compiler warnings for over-permission.

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

## Known Limitations

### Arrow Type Effect Annotations Silently Dropped

Effect annotations on arrow types in type expressions (e.g., `fn(Int) -> Int & {Fs}` in an ADT field or type annotation) are parsed but silently dropped by the type checker, defaulting the slot to pure — this is sound (rejects effectful functions, conservative default) but is a UX trap that should be addressed in a future issue.

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

**See:** [ADR-0003](adr/adr-0003.md) for full rationale.

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
- Link from this doc: `**See:** [ADR-NNNN](adr/adr-NNNN.md)`
- Keep this doc focused, use ADRs for deep dives