# Issue 012: Hierarchical Capability Bundles

## Status: Planning

## Goal
Replace coarse `FsCap` with granular `FsCap<Read>`, `FsCap<Write>`, `FsCap<ReadWrite>`.
Minimum viable least-privilege for real automation scripts.

## Prerequisites
- Issue 011 (WASM runtime — capabilities need enforcement points)

## Open Questions
- Subtyping vs. separate types vs. parameterized caps?
- How does this interact with attenuation (post-v0.1)?
- What granularity levels for each cap kind?

## Design Review: Not yet started
