# Strata Feature Candidates (Post-v0.1)

## Assessed Features

### Effect Virtualization (Algebraic Effect Handlers)
**Priority: v0.2 | Complexity: High (shallow) / Very High (full)**

Intercept and redirect effects via typed handlers. Enables: safe dry-runs, testing
without infra, chaos testing, sandboxing untrusted code, degraded-mode operation.

Phase 1 (v0.2): Shallow handlers — no resumption, handler returns a value.
Phase 2 (v0.3): Full algebraic handlers with delimited continuations (resume keyword).

WASM limitation: Standard WASM 1.0 lacks stack switching. Requires Asyncify transform
or native runtime. Strong argument for native runtime roadmap.

Design constraint: One handler per effect per scope, lexical scoping, inner overrides
outer. No handler chains, no dynamic rebinding, no handler mutation.

Security concern: Handlers can lie or escalate. Mitigate with sealed effects (handler
can observe but not modify effect semantics) and audited handler installation.

### Capability Attenuation
**Priority: v0.2 | Complexity: Medium**

Narrow capabilities at runtime: `fs.read_only("/etc")` consumes broad cap, produces
restricted cap.

Implementation model: Wrapper/Proxy, NOT subtyping. Consumes original (affine move),
returns new cap with internal constraint.

Granularity: Operation-level first (read/write/list), scope-level second
(directory/bucket/namespace). Avoid time-bounded attenuation initially.

Security concern: Path injection risk if string-based. Use validated parsers and
nominal types, not raw strings. Ensure no reverse-attenuation or cap-combination to
reconstruct broad authority.

### Reason Tracing (Structured Audit)
**Priority: v0.3 | Complexity: Low**

Attach structured rationale to capability use for audit trails. MUST be structured
and machine-consumable (not free-form strings). OTel-compatible.

Implementation: API/library first, syntax sugar later.
Bad: `use fs "needed for deployment"`
Good: `use fs reason DeployArtifact(version, target)`

Security caveat: Reason strings are spoofable — never use for security decisions,
only for post-mortem audit.

### Durable Execution (Checkpoint/Resume)
**Priority: v0.3+ | Complexity: Very High**

Long-running workflows that survive restarts. Strata's no-ambient-authority and effect
tracking make state theoretically serializable.

Critical decision: This is a RUNTIME PRODUCT, not a language feature. Build as a
Strata-aware executor that consumes effect traces, not as a compiler primitive.

Security requirement: Signed/encrypted snapshots, capability watermarking, idempotency
tracking for non-idempotent effects. Serialization invites tampering.

Prerequisite: Effect virtualization (to handle world-state differences on resume).

### Capability Delegation (Safe FFI)
**Priority: v0.3+ | Complexity: High**

Map Strata capabilities to WASM Component Model handles. Ensures FFI code (C, Rust
components) only accesses resources passed by the Strata type system.

### Temporal/Leased Capabilities
**Priority: Research | Complexity: Medium**

Time-bounded authority: `fs.lease(duration: "1h")`. Runtime invalidates after expiry.
Adds runtime clock semantics — defer until runtime is mature.

### Taint Propagation for AI-Generated Code
**Priority: Research | Complexity: High**

Mark AI-generated code as tainted. Restrict to virtualized handlers only — no real
effects until human review. Unique to the AI agent safety use case.

## Feature Interaction Map

- Effect Virtualization ENABLES: Durable Execution, enhanced plan mode, testing
- Capability Attenuation ENABLES: Delegation, least-privilege AI agents
- Reason Tracing ENHANCES: plan output, audit compliance
- Durable Execution REQUIRES: Effect Virtualization, mature runtime
