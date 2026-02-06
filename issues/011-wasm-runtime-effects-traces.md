# Issue 011: WASM Runtime + Effect Traces

**Status:** Ready after Issue 010
**Estimated time:** 2-3 weeks
**Phase:** 4 (Runtime)
**Depends on:** Issue 010 (Affine Types)

## Goal

Compile Strata programs to WASM and run them with real I/O. Generate effect traces (JSON audit logs) for every side effect. Support deterministic replay from traces.

## Architecture Overview
```
┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
│  Strata Source  │ ──▶  │  WAT Emission   │ ──▶  │  .wasm Binary   │
└─────────────────┘      └─────────────────┘      └─────────────────┘
                                                          │
                                                          ▼
                         ┌─────────────────────────────────────────┐
                         │            WASI Runtime                 │
                         │  ┌─────────────────────────────────┐    │
                         │  │      Host Functions             │    │
                         │  │  ┌──────┐ ┌──────┐ ┌──────┐    │    │
                         │  │  │fs_*  │ │net_* │ │time_*│    │    │
                         │  │  └──┬───┘ └──┬───┘ └──┬───┘    │    │
                         │  │     │        │        │         │    │
                         │  │     ▼        ▼        ▼         │    │
                         │  │  ┌─────────────────────────┐    │    │
                         │  │  │     Trace Emitter       │    │    │
                         │  │  └─────────────────────────┘    │    │
                         │  └─────────────────────────────────┘    │
                         └─────────────────────────────────────────┘
                                          │
                                          ▼
                                   ┌──────────────┐
                                   │ trace.json   │
                                   └──────────────┘
```

Key insight from external review: **Trace emission happens at the host boundary, not in codegen.** This keeps the compiler simple and makes determinism achievable.

## Compilation Strategy: WAT for v0.1

Emit human-readable WAT (WebAssembly Text Format), compile to binary via `wat2wasm`.

**Why WAT:**
- Easier debugging (diff compiled output, inspect stack ops)
- Decoupled concerns (Strata compiler doesn't own binary encoding)
- Faster iteration while linearity semantics stabilize

**Migration path:** v0.2 switches to raw bytecode emission for zero-dependency compiler.

## Capabilities as WASI Handles

Capabilities are represented as `i32` indices into a runtime-managed table:
```
┌─────────────────────────────────────┐
│         Capability Table            │
├─────────┬───────────────────────────┤
│ Index 0 │ FsCap (file descriptor)   │
│ Index 1 │ NetCap (socket pool)      │
│ Index 2 │ TimeCap (clock access)    │
│ Index 3 │ RandCap (entropy source)  │
│ Index 4 │ AiCap (API credentials)   │
└─────────┴───────────────────────────┘
```

WASM guest code cannot forge handles — it can only use integers the runtime provides.

## Host Functions (WASI Interface)
```wat
;; File system operations (require FsCap handle)
(import "strata" "fs_read" (func $fs_read (param i32 i32 i32) (result i32)))
(import "strata" "fs_write" (func $fs_write (param i32 i32 i32 i32) (result i32)))

;; Network operations (require NetCap handle)
(import "strata" "net_get" (func $net_get (param i32 i32 i32) (result i32)))
(import "strata" "net_post" (func $net_post (param i32 i32 i32 i32 i32) (result i32)))

;; Time operations (require TimeCap handle)
(import "strata" "time_now" (func $time_now (param i32) (result i64)))
(import "strata" "time_sleep" (func $time_sleep (param i32 i32) (result i32)))

;; Random operations (require RandCap handle)
(import "strata" "rand_bytes" (func $rand_bytes (param i32 i32 i32) (result i32)))

;; AI operations (require AiCap handle)
(import "strata" "ai_complete" (func $ai_complete (param i32 i32 i32 i32 i32) (result i32)))
```

Every effectful operation requires the capability handle as the first parameter.

## Effect Trace Format
```json
{
  "program": "deploy.strata",
  "started_at": "2026-02-05T10:30:00.000Z",
  "finished_at": "2026-02-05T10:30:01.500Z",
  "result": { "ok": "deploy-456" },
  "effects": [
    {
      "seq": 0,
      "timestamp": "2026-02-05T10:30:00.100Z",
      "effect": "Fs",
      "operation": "read",
      "inputs": { "path": "/models/v2.pkl" },
      "outputs": { "bytes": 1048576, "checksum": "abc123" },
      "duration_ms": 50
    },
    {
      "seq": 1,
      "timestamp": "2026-02-05T10:30:00.200Z",
      "effect": "Net",
      "operation": "post",
      "inputs": { "url": "https://api.example.com/upload", "body_bytes": 1048576 },
      "outputs": { "status": 200, "body": "{\"id\": \"upload-789\"}" },
      "duration_ms": 800
    }
  ]
}
```

Fields:
- `seq`: Monotonic sequence number for ordering
- `timestamp`: Wall-clock time (for debugging, not determinism)
- `effect`: Which effect type (Fs, Net, Time, Rand, Ai)
- `operation`: Specific operation (read, write, get, post, etc.)
- `inputs`: Arguments to the operation
- `outputs`: Return values
- `duration_ms`: How long the operation took

## Deterministic Replay

Replay mode re-executes a program using recorded effect outputs:
```
Normal execution:
  fs_read("/config.toml") → [actual file contents]
  
Replay execution:
  fs_read("/config.toml") → [contents from trace.effects[0].outputs]
```

Implementation:
1. Load trace JSON
2. Host functions read from trace instead of performing real I/O
3. Inputs are validated (if `fs_read` is called with different path than trace, error)
4. Outputs are returned from trace
5. Program executes deterministically

This enables:
- Reproducing failures exactly
- Debugging without re-running real effects
- Testing with recorded production data

## CLI Interface
```bash
# Compile Strata to WASM
strata build src/deploy.strata -o deploy.wasm

# Run with tracing (default)
strata run deploy.wasm --trace trace.json

# Run without tracing (performance mode)
strata run deploy.wasm --no-trace

# Replay from trace
strata replay trace.json

# Replay with modified inputs (for testing)
strata replay trace.json --override 'effects[1].outputs.status=500'
```

## Scope

### In Scope (v0.1)
- WAT code emission from Strata AST
- Basic WASM operations: functions, locals, arithmetic, control flow
- WASI host functions for Fs, Net, Time, Rand, Ai
- Trace emission at host boundary
- JSON trace format
- Deterministic replay from traces
- CLI: build, run, replay commands

### Out of Scope (v0.2+)
- Raw bytecode emission (no wat2wasm dependency)
- Optimization passes
- Streaming traces (for long-running programs)
- Trace compression
- Remote trace storage
- Trace diffing tools

## Definition of Done

- [ ] WAT emitter: expressions, statements, functions
- [ ] WAT emitter: capability handle threading
- [ ] WAT emitter: effect calls to host imports
- [ ] Runtime: WASI host functions (fs, net, time, rand, ai)
- [ ] Runtime: capability table management
- [ ] Runtime: trace emission on every host call
- [ ] Runtime: JSON trace serialization
- [ ] Runtime: replay mode (read from trace)
- [ ] CLI: `strata build` command
- [ ] CLI: `strata run` with `--trace` / `--no-trace`
- [ ] CLI: `strata replay` command
- [ ] Tests: compile and run simple programs
- [ ] Tests: trace output matches expected format
- [ ] Tests: replay produces identical results
- [ ] Tests: replay with wrong inputs errors
- [ ] Documentation: trace format specification
- [ ] Documentation: replay usage guide

## Test Cases
```strata
// Simple pure function → compiles to WASM, runs
fn add(x: Int, y: Int) -> Int & {} {
    x + y
}

// Effectful function → generates trace
fn read_config(fs: FsCap) -> String & {Fs} {
    read_file("config.toml", fs)
}

// Multiple effects → trace contains all
fn fetch_and_save(fs: FsCap, net: NetCap) -> () & {Fs, Net} {
    let data = http_get("https://example.com", net);
    write_file("cache.txt", data, fs)
}
```

Expected trace for `fetch_and_save`:
```json
{
  "effects": [
    { "seq": 0, "effect": "Net", "operation": "get", ... },
    { "seq": 1, "effect": "Fs", "operation": "write", ... }
  ]
}
```

Replay test:
1. Run `fetch_and_save`, capture trace
2. Replay trace
3. Assert program completes with same result
4. Assert no actual network/filesystem calls made during replay

## Security Considerations

- Capabilities are opaque i32 handles — cannot be forged in WASM
- Host functions validate capability handles before performing operations
- Trace files may contain sensitive data — document this, consider encryption for v0.2
- Replay mode never performs real I/O — safe for debugging production traces

## Dependencies

- `wasmtime` (or similar WASI runtime) for execution
- `wat` crate for WAT parsing/validation (optional, for testing)
- `wat2wasm` tool for binary compilation (bundled or documented dependency)