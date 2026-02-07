# Issue 014: `strata plan` + CLI Polish

## Status: Planning

## Goal
Ship `strata plan` — static effect/capability analysis without executing.
"Terraform plan for automation."

**Already shipped (011a):**
- `strata run <file>` — type-check and execute
- `strata run <file> --trace <path>` — execute with audit trace
- `strata run <file> --trace-full <path>` — execute with replay-capable trace
- `strata replay <trace> <file>` — replay trace against source
- `strata replay <trace>` — print trace summary
- `strata parse <file>` — dump AST (pretty or JSON)

**New in this issue:**
- `strata plan <file>` — static analysis showing:
  - Every effect the program declares
  - Every capability it requires
  - Every extern function it would call
  - Call graph with effect annotations
  - Per-function capability requirements
- `strata check <file>` — type-check only (no execution)
- `--deny <cap>` flag — reject programs requiring specific capabilities
- Output formats: human-readable (default), JSON (`--format json`)

`strata plan` is the killer adoption demo: show security teams exactly what
a script will do before it runs. Like `terraform plan` but for arbitrary
automation code.

## Prerequisites
- Issue 013 (stdlib — plans need real host functions to be meaningful)

## Open Questions
- Output format for `strata plan`? (Human-readable? JSON? Both?)
- How detailed should plan output be? (Per-function? Per-call-site?)
- Should plan show the call graph or just top-level effects?
- Color output for terminals?

## Design Review: Not yet started
