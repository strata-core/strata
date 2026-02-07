# Issue 016: Hierarchical Capability Bundles

## Status: Deferred to v0.2

## Goal
Replace coarse `FsCap` with granular `FsCap<Read>`, `FsCap<Write>`, `FsCap<ReadWrite>`.
Minimum viable least-privilege for real automation scripts.

## Why Deferred
Coarse-grained capabilities (FsCap, NetCap, etc.) are sufficient for v0.1.
Granular Read/Write distinction is an ergonomic refinement, not a soundness
requirement. The current model already enforces no-ambient-authority and
single-use semantics. Hierarchical bundles add value for least-privilege
policies but don't change the security model.

Renumbered from Issue 012 during post-011a roadmap reorganization (Feb 2026).

## Prerequisites
- Issue 013 (stdlib — granular caps need host functions to distinguish operations)

## Open Questions
- Subtyping vs. separate types vs. parameterized caps?
- How does this interact with attenuation (post-v0.1)?
- What granularity levels for each cap kind?

## Design Review: Not yet started
