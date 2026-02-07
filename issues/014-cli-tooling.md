# Issue 014: CLI Tooling

## Status: Planning

## Goal
Ship `strata check`, `strata run`, and `strata plan`.

- `strata check <file>` — type-check without executing
- `strata run <file>` — type-check and execute in WASM runtime
- `strata plan <file>` — static effect/capability analysis (dry run)

`strata plan` is the killer demo: show every effect, required capability, and external
call that would occur — without executing. "Terraform plan for automation."

## Prerequisites
- Issue 011 (runtime for `strata run`)
- Issue 013 (stdlib for meaningful plans)

## Open Questions
- Output format for `strata plan`? (Human-readable? JSON? Both?)
- How detailed should plan output be? (Per-function? Per-call-site?)

## Design Review: Not yet started
