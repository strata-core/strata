# Issue 011a: Traced Runtime (Interpreter-based)

Read `docs/DEV_WORKFLOW.md` and `docs/lessons-learned.md` before starting.

## Overview

This is the inflection point: Strata programs will execute with real effects and produce
structured audit trails. We're extending the existing tree-walking interpreter (eval.rs)
with host function dispatch, capability enforcement at runtime, effect trace emission,
and deterministic replay.

**This issue also adds extern-only borrowing (`&` on extern fn capability params).**
The cohort unanimously agreed that capability threading (`let (result, fs) = read(fs, path)`)
is not ergonomically viable — it interacts badly with error handling (`?` operator loses
the capability on the error path). Extern-only borrowing is the v0.1 solution.

## What Ships

After this issue, a user can write:

```strata
extern fn read_file(fs: &FsCap, path: String) -> String & {Fs};
extern fn write_file(fs: &FsCap, path: String, content: String) -> () & {Fs};
extern fn now(time: &TimeCap) -> String & {Time};

fn main(fs: FsCap, time: TimeCap) -> () & {Fs, Time} {
    let config = read_file(&fs, "/tmp/config.yml");
    let timestamp = now(&time);
    write_file(&fs, "/tmp/output.txt", config);
}
```

And run it:
```fish
strata run example.strata --trace trace.jsonl
strata replay trace.jsonl
```

The trace file records every host function call with inputs, outputs, timing, and
capability usage — a complete audit trail.

## Architecture Decisions (LOCKED — do not deviate)

These were decided after multi-LLM review (Gemini, Grok, ChatGPT):

1. **Interpreter-first (Option D).** No WASM, no IR, no codegen. Extend eval.rs.
   The interpreter becomes the permanent semantic oracle for future compilation targets.

2. **Extern-only borrowing.** `&CapType` syntax ONLY on extern fn capability parameters.
   User-defined Strata functions still consume capabilities (affine). The `&` means
   "this host function uses the capability but doesn't consume it."

3. **Streaming trace format.** JSONL (one JSON object per line), streamed to disk
   incrementally. Content hashing for payloads above a threshold.

4. **Capability enforcement at runtime.** The interpreter validates capabilities at
   the host function dispatch boundary — not just at compile time.

5. **Entry function receives capabilities from runtime.** `fn main(fs: FsCap, ...)` —
   the runtime inspects main's signature and provides the requested capabilities.

## Phase 1: Extern-Only Borrowing (Type System)

**Goal:** Add `&CapType` syntax to extern fn params. Update parser, type checker,
and move checker.

### 1a. Parser Changes (`strata-parse`)

Add `&` prefix to type expressions, but ONLY in function parameter position:

```
// In extern fn declarations:
extern fn read_file(fs: &FsCap, path: String) -> String & {Fs};
//                      ^ new: borrow marker

// In regular fn declarations (for calling externs):
fn deploy(fs: FsCap) -> () & {Fs} {
    read_file(&fs, "/tmp/config");  // <-- & in call expression, not type
}
```

Two parse sites:
1. **Type expression:** `&FsCap` as a type — `TypeExpr::Ref(Box<TypeExpr>)` — allowed
   in extern fn params only (checker enforces this, parser allows it generally)
2. **Expression:** `&fs` as an expression — `Expr::Borrow(Box<Expr>)` — borrow a
   variable at a call site

### 1b. AST Changes (`strata-ast`)

```rust
// Add to TypeExpr enum:
Ref(Box<TypeExpr>, Span),  // &T

// Add to Expr enum:
Borrow(Box<Expr>, Span),  // &x
```

### 1c. Type System Changes (`strata-types`)

```rust
// Add to Ty enum:
Ref(Box<Ty>),  // &T — reference to a type

// In ty.rs, Ty::kind():
Ty::Ref(inner) => Kind::Unrestricted,  // References are always unrestricted
// (The referent is affine, but borrowing it doesn't consume it)
```

**Checker changes:**
- `ty_from_type_expr` handles `TypeExpr::Ref` → `Ty::Ref(inner_ty)`
- **Restriction:** `Ty::Ref` is only allowed in extern fn parameter types.
  If `Ty::Ref` appears in a non-extern fn param, return type, let binding,
  or ADT field → emit error:
  `"Reference types (&T) are only allowed in extern function parameters in v0.1"`
- When type-checking a call to an extern fn with `&CapType` param, the argument
  must be `Expr::Borrow(var)` where `var` has type `CapType`. Unify the borrow
  expression type with `Ty::Ref(CapType)`.

### 1d. Move Checker Changes (`move_check.rs`)

This is the critical change. When the move checker encounters `Expr::Borrow(var)`:
- Look up `var` in tracked bindings
- Verify `var` is **alive** (not consumed)
- Do NOT mark `var` as consumed — this is a borrow, not a move
- The capability remains alive for future borrows or final consumption

When checking a call to an extern fn with `&CapType` param:
- The argument is `Expr::Borrow(var)` — validated as above
- The extern fn borrows the cap; it remains alive after the call

When checking a call to a regular Strata fn with `CapType` param (no `&`):
- Existing behavior — the argument is consumed (moved)

### 1e. Tests

Add to `tests/capabilities.rs` and `tests/move_check.rs`:

**Positive tests:**
- `extern_borrow_cap_single_use` — extern borrows cap once, OK
- `extern_borrow_cap_multiple_use` — extern borrows cap 3 times, OK (borrow doesn't consume)
- `extern_borrow_then_strata_consume` — borrow cap in extern calls, then pass to Strata fn that consumes it
- `borrow_multiple_caps` — extern borrows FsCap and NetCap, both survive

**Negative tests:**
- `borrow_after_consume_error` — pass cap to Strata fn (consumed), then try to borrow → error
- `consume_after_consume_error` — existing test, should still work
- `ref_type_in_strata_fn_error` — `fn bad(fs: &FsCap)` → error (& only in extern fn)
- `ref_type_in_return_error` — `fn bad() -> &FsCap` → error
- `ref_type_in_let_error` — `let x: &FsCap = ...` → error
- `borrow_non_cap_error` — `extern fn bad(x: &Int)` → error (& only on cap types)

**Verification:**
```fish
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Report new test count. Commit after Phase 1 passes.

---

## Phase 2: Host Function Dispatch

**Goal:** When the interpreter encounters a call to an extern fn, dispatch it to
a Rust implementation that performs real I/O.

### 2a. Host Function Registry

Create a host function registry in eval.rs (or a new module `eval/host.rs`):

```rust
use std::collections::HashMap;

pub type HostFn = fn(&[Value], &mut TraceEmitter) -> Result<Value, HostError>;

pub struct HostRegistry {
    functions: HashMap<String, HostFn>,
}

impl HostRegistry {
    pub fn new() -> Self {
        let mut reg = Self { functions: HashMap::new() };
        reg.register("read_file", host_read_file);
        reg.register("write_file", host_write_file);
        reg.register("now", host_now);
        reg.register("random_int", host_random_int);
        reg
    }

    pub fn call(&self, name: &str, args: &[Value], tracer: &mut TraceEmitter)
        -> Result<Value, HostError>
    {
        let f = self.functions.get(name)
            .ok_or_else(|| HostError::UnknownFunction(name.to_string()))?;
        f(args, tracer)
    }
}
```

### 2b. Host Function Implementations

Implement basic host functions. These take `&[Value]` (the non-capability args — 
capabilities are validated at dispatch, not passed to the host fn) and return `Value`.

```rust
fn host_read_file(args: &[Value], _tracer: &mut TraceEmitter) -> Result<Value, HostError> {
    // args[0] = path (String) — cap was validated at dispatch, not passed here
    let path = match &args[0] {
        Value::Str(s) => s,
        _ => return Err(HostError::TypeError("read_file: expected String path".into())),
    };
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Value::Str(content)),
        Err(e) => Err(HostError::IoError(format!("read_file: {}", e))),
    }
}

fn host_write_file(args: &[Value], _tracer: &mut TraceEmitter) -> Result<Value, HostError> {
    let path = match &args[0] {
        Value::Str(s) => s,
        _ => return Err(HostError::TypeError("write_file: expected String path".into())),
    };
    let content = match &args[1] {
        Value::Str(s) => s,
        _ => return Err(HostError::TypeError("write_file: expected String content".into())),
    };
    match std::fs::write(path, content) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => Err(HostError::IoError(format!("write_file: {}", e))),
    }
}

fn host_now(args: &[Value], _tracer: &mut TraceEmitter) -> Result<Value, HostError> {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| HostError::RuntimeError(format!("now: {}", e)))?;
    Ok(Value::Str(format!("{}.{:03}", now.as_secs(), now.subsec_millis())))
}

fn host_random_int(args: &[Value], _tracer: &mut TraceEmitter) -> Result<Value, HostError> {
    // Simple random using system time (no external crate needed for v0.1)
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    Ok(Value::Int((seed % 1000) as i64))
}
```

### 2c. Capability Validation at Dispatch

When the interpreter evaluates a call to an extern fn:

1. Look up the extern fn's type signature (from the type checker's environment)
2. For each capability parameter:
   - If `&CapType` (borrow): verify the capability value is present and valid
   - If `CapType` (consume): verify present, mark as consumed
3. Extract the non-capability arguments
4. Call the host function with non-capability args
5. Return the result

The capability value in the interpreter is `Value::Cap(CapKind)`. Add this variant
to the Value enum if it doesn't exist:

```rust
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
    Cap(CapKind),  // NEW: runtime capability token
    Closure { params: Vec<String>, body: Block, env: Env },
    // ADT variants as needed
}
```

### 2d. Entry Point: Providing Capabilities to main()

When `strata run` executes a program:

1. Find the `main` function (or the last fn definition)
2. Inspect its parameter types
3. For each `CapType` parameter, create a `Value::Cap(kind)` 
4. Call main with the provided capabilities
5. By default, all requested capabilities are granted
6. Future: `--deny Fs` flag to deny specific capabilities

### 2e. Tests

**Integration tests** (create `tests/integration/` or add to existing):

- `host_read_file_ok` — write a temp file, read it via Strata extern call
- `host_write_file_ok` — write via Strata, verify file exists
- `host_now_returns_string` — call now(), verify result is string
- `host_unknown_function_error` — call unregistered extern → clear error
- `host_missing_cap_error` — call extern requiring FsCap without it → error
- `main_receives_caps` — main(fs: FsCap) receives a valid cap value

**Verification:**
```fish
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Report new test count. Commit after Phase 2 passes.

---

## Phase 3: Effect Trace Emission

**Goal:** Every host function call emits a structured trace entry. Traces stream
to disk as JSONL.

### 3a. Trace Format

Each trace entry is one JSON object per line (JSONL):

```json
{"seq":0,"timestamp":"2026-02-06T12:00:00.000Z","effect":"Fs","operation":"read_file","capability":{"kind":"FsCap","access":"borrow"},"inputs":{"path":"/tmp/config.yml"},"output":{"status":"ok","value_hash":"sha256:abc123","value_size":1234,"value":"...content..."},"duration_ms":12}
```

**Fields:**
- `seq`: monotonically increasing sequence number (u64)
- `timestamp`: ISO 8601 with milliseconds
- `effect`: the effect kind (Fs, Net, Time, Rand)
- `operation`: the host function name
- `capability.kind`: cap type used
- `capability.access`: "borrow" or "consume"
- `inputs`: named input arguments (non-capability args only)
- `output.status`: "ok" or "error"
- `output.value`: the returned value (omitted if large, replaced by hash)
- `output.value_hash`: SHA-256 of the value (always present)
- `output.value_size`: byte size of the serialized value
- `duration_ms`: wall-clock time of host function execution

**Content hashing rule:** If `value_size > 1024` bytes (1KB), omit `value` from the
trace and only record `value_hash` and `value_size`. This prevents trace explosion
from large file reads.

### 3b. TraceEmitter

```rust
use std::io::Write;
use std::time::Instant;

pub struct TraceEmitter {
    seq: u64,
    writer: Option<Box<dyn Write>>,  // None = tracing disabled
}

impl TraceEmitter {
    pub fn new(writer: Option<Box<dyn Write>>) -> Self {
        Self { seq: 0, writer }
    }

    pub fn disabled() -> Self {
        Self { seq: 0, writer: None }
    }

    pub fn emit(&mut self, entry: TraceEntry) {
        if let Some(ref mut w) = self.writer {
            let json = serde_json::to_string(&entry).expect("trace serialization");
            writeln!(w, "{}", json).expect("trace write");
        }
        self.seq += 1;
    }

    pub fn next_seq(&self) -> u64 { self.seq }
}
```

### 3c. Wrapping Host Dispatch with Tracing

Modify the host function dispatch to wrap every call with trace emission:

```rust
fn dispatch_host_fn(
    &mut self,
    name: &str,
    cap_kind: &str,
    cap_access: &str,  // "borrow" or "consume"
    args: &[Value],     // non-capability args
    tracer: &mut TraceEmitter,
) -> Result<Value, HostError> {
    let start = Instant::now();
    let inputs = serialize_inputs(name, args);

    let result = self.host_registry.call(name, args, tracer);

    let duration = start.elapsed();
    let (status, output_value, output_hash, output_size) = match &result {
        Ok(val) => {
            let serialized = serialize_value(val);
            let hash = sha256_hex(&serialized);
            let size = serialized.len();
            let value = if size <= 1024 { Some(serialized) } else { None };
            ("ok", value, hash, size)
        }
        Err(e) => {
            let err_str = e.to_string();
            let hash = sha256_hex(&err_str);
            ("error", Some(err_str.clone()), hash, err_str.len())
        }
    };

    tracer.emit(TraceEntry {
        seq: tracer.next_seq(),
        timestamp: now_iso8601(),
        effect: cap_kind.to_string(),
        operation: name.to_string(),
        capability: CapRef { kind: cap_kind.to_string(), access: cap_access.to_string() },
        inputs,
        output: TraceOutput {
            status: status.to_string(),
            value: output_value,
            value_hash: output_hash,
            value_size: output_size,
        },
        duration_ms: duration.as_millis() as u64,
    });

    result
}
```

### 3d. SHA-256

Use the `sha2` crate (add to Cargo.toml):
```toml
sha2 = "0.10"
```

```rust
use sha2::{Sha256, Digest};
fn sha256_hex(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}
```

### 3e. serde for TraceEntry

Add serde + serde_json to the crate that contains the tracer:
```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

```rust
#[derive(Serialize)]
pub struct TraceEntry {
    pub seq: u64,
    pub timestamp: String,
    pub effect: String,
    pub operation: String,
    pub capability: CapRef,
    pub inputs: serde_json::Value,
    pub output: TraceOutput,
    pub duration_ms: u64,
}

#[derive(Serialize)]
pub struct CapRef {
    pub kind: String,
    pub access: String,
}

#[derive(Serialize)]
pub struct TraceOutput {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub value_hash: String,
    pub value_size: usize,
}
```

### 3f. Tests

- `trace_records_read_file` — read a file, verify trace has correct seq/effect/operation/inputs
- `trace_records_write_file` — write a file, verify trace entry
- `trace_hashes_large_output` — read a file > 1KB, verify value is omitted but hash present
- `trace_includes_small_output` — read a file < 1KB, verify value is included
- `trace_records_error` — read nonexistent file, verify trace has status "error"
- `trace_sequential_seq_numbers` — multiple calls, verify seq is 0, 1, 2, ...
- `trace_disabled_no_output` — TraceEmitter::disabled() produces nothing

**Verification:**
```fish
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Report new test count. Commit after Phase 3 passes.

---

## Phase 4: Deterministic Replay

**Goal:** Replay a previously recorded trace. Instead of calling real host functions,
substitute outputs from the trace. Verify inputs match.

### 4a. TraceReplayer

```rust
pub struct TraceReplayer {
    entries: Vec<TraceEntry>,
    cursor: usize,
}

impl TraceReplayer {
    pub fn from_file(path: &str) -> Result<Self, ReplayError> {
        let content = std::fs::read_to_string(path)?;
        let entries: Vec<TraceEntry> = content.lines()
            .enumerate()
            .map(|(i, line)| {
                serde_json::from_str(line)
                    .map_err(|e| ReplayError::ParseError(i, e.to_string()))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { entries, cursor: 0 })
    }

    pub fn next(&mut self, operation: &str, inputs: &serde_json::Value)
        -> Result<Value, ReplayError>
    {
        let entry = self.entries.get(self.cursor)
            .ok_or(ReplayError::UnexpectedEffect(operation.to_string()))?;

        // Validate: same operation
        if entry.operation != operation {
            return Err(ReplayError::OperationMismatch {
                expected: entry.operation.clone(),
                actual: operation.to_string(),
                seq: self.cursor as u64,
            });
        }

        // Validate: same inputs
        if entry.inputs != *inputs {
            return Err(ReplayError::InputMismatch {
                operation: operation.to_string(),
                seq: self.cursor as u64,
                expected: entry.inputs.clone(),
                actual: inputs.clone(),
            });
        }

        self.cursor += 1;

        // Return the recorded output
        match entry.output.status.as_str() {
            "ok" => {
                let value = entry.output.value.as_ref()
                    .ok_or_else(|| ReplayError::MissingValue {
                        operation: operation.to_string(),
                        seq: (self.cursor - 1) as u64,
                    })?;
                deserialize_value(value)
            }
            "error" => {
                let err_msg = entry.output.value.as_ref()
                    .cloned()
                    .unwrap_or_else(|| "unknown error".to_string());
                Err(ReplayError::ReplayedError(err_msg))
            }
            other => Err(ReplayError::UnknownStatus(other.to_string())),
        }
    }

    pub fn verify_complete(&self) -> Result<(), ReplayError> {
        if self.cursor < self.entries.len() {
            Err(ReplayError::UnreplayedEffects(self.entries.len() - self.cursor))
        } else {
            Ok(())
        }
    }
}
```

### 4b. Replay Mode in Evaluator

The evaluator needs to support two modes:
- **Live mode:** call real host functions, emit traces
- **Replay mode:** substitute outputs from trace, validate inputs

This should be a runtime configuration, not a code fork. Use an enum:

```rust
pub enum RuntimeMode {
    Live {
        host_registry: HostRegistry,
        tracer: TraceEmitter,
    },
    Replay {
        replayer: TraceReplayer,
    },
}
```

When dispatching an extern fn call:
- If `Live`: call host fn, emit trace (existing behavior)
- If `Replay`: call `replayer.next()` with operation name and inputs

### 4c. Replay Limitations (v0.1)

- Replay requires the trace to contain actual values (not just hashes) for outputs
  that the program uses. If a value was hashed (> 1KB), replay will fail with a
  clear error: `"Cannot replay: output for read_file at seq 3 was hashed (2.1 KB).
  Re-run with --trace-full to record complete values."`
- `--trace-full` flag records all values regardless of size (for replay-capable traces)
- Default `--trace` uses content hashing (for audit logs that don't need replay)

### 4d. Tests

- `replay_produces_same_result` — run program, capture trace, replay, compare final output
- `replay_detects_input_mismatch` — modify program's inputs, replay → InputMismatch error
- `replay_detects_operation_mismatch` — modify program's extern call order, replay → OperationMismatch
- `replay_detects_extra_effects` — program has fewer extern calls than trace → UnreplayedEffects
- `replay_detects_missing_effects` — program has more extern calls than trace → UnexpectedEffect
- `replay_handles_errors` — trace contains an error, replay returns same error

**Verification:**
```fish
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Report new test count. Commit after Phase 4 passes.

---

## Phase 5: CLI Integration

**Goal:** `strata run` and `strata replay` commands.

### 5a. CLI Commands

Update `strata-cli/src/main.rs` to support:

```
strata run <file>                     # Execute, no trace
strata run <file> --trace <path>      # Execute, write trace to path
strata run <file> --trace-full <path> # Execute, write replay-capable trace
strata replay <trace-path>            # Replay from trace
strata replay <trace-path> <file>     # Replay, verify against source
```

### 5b. Run Command

1. Parse the source file
2. Type-check (existing pipeline)
3. Move-check (existing pipeline)
4. Find `main` function
5. Inspect main's parameters for capability types
6. Create `Value::Cap(kind)` for each capability param
7. Create `TraceEmitter` (with file writer if `--trace`, disabled otherwise)
8. Create `RuntimeMode::Live { host_registry, tracer }`
9. Evaluate main with the capabilities and runtime mode
10. Print the result (or "Program completed successfully")
11. If `--trace`: print "Trace written to <path>"

### 5c. Replay Command

1. Load the trace file
2. If source file provided:
   a. Parse and type-check the source
   b. Find main, create caps
   c. Create `RuntimeMode::Replay { replayer }`
   d. Evaluate main
   e. Call `replayer.verify_complete()`
   f. Print "Replay successful: N effects replayed"
3. If no source file:
   a. Print trace summary (effect count, operations, duration)

### 5d. Error Reporting

Runtime errors should be clear and actionable:

```
Error: Host function failed at example.strata:5
  read_file("/nonexistent.txt")
  
  Cause: No such file or directory
  Effect: Fs
  Capability: FsCap (borrowed)

  Trace: effect recorded at seq 2
```

```
Error: Replay mismatch at seq 3
  Expected: read_file("/config.yml")
  Actual:   read_file("/settings.yml")
  
  The program's inputs don't match the recorded trace.
  This could mean the source code changed since the trace was recorded.
```

### 5e. Tests

- `cli_run_no_trace` — run a simple program, verify stdout
- `cli_run_with_trace` — run program, verify trace file created
- `cli_replay_success` — run, capture trace, replay, verify success message
- `cli_replay_mismatch` — modify source, replay → mismatch error

### 5f. Example Program

Create `examples/deploy.strata`:

```strata
extern fn read_file(fs: &FsCap, path: String) -> String & {Fs};
extern fn write_file(fs: &FsCap, path: String, content: String) -> () & {Fs};
extern fn now(time: &TimeCap) -> String & {Time};

fn main(fs: FsCap, time: TimeCap) -> () & {Fs, Time} {
    let config = read_file(&fs, "/tmp/strata-demo/config.txt");
    let timestamp = now(&time);
    write_file(&fs, "/tmp/strata-demo/output.txt", config);
}
```

Create `examples/README.md` documenting how to run the demo:

```fish
# Create test files
mkdir -p /tmp/strata-demo
echo "database: postgres://localhost/myapp" > /tmp/strata-demo/config.txt

# Run with trace
strata run examples/deploy.strata --trace /tmp/strata-demo/trace.jsonl

# View trace
cat /tmp/strata-demo/trace.jsonl | python3 -m json.tool

# Replay
strata replay /tmp/strata-demo/trace.jsonl examples/deploy.strata
```

**Verification:**
```fish
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Report final test count. Commit after Phase 5 passes.

---

## What NOT To Do

- Do NOT add WASM, IR, or codegen. This is interpreter only.
- Do NOT add http_get or network host functions (needs external crate, defer to stdlib issue).
- Do NOT add error handling / Result types to Strata (defer to future issue).
  Host function errors should crash the program with a clear message for v0.1.
- Do NOT add `--deny` capability flags to CLI (future enhancement).
- Do NOT modify the unifier or constraint solver.
- Do NOT add closures or lambda syntax.
- Do NOT add capability attenuation or delegation.

## Dependencies

Add to the appropriate crate's Cargo.toml:
```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
```

Keep dependencies minimal. No async runtime, no reqwest, no tokio.

## Definition of Done

- [ ] `&CapType` parses in extern fn params
- [ ] `&expr` parses as borrow expression
- [ ] Move checker treats borrows as non-consuming
- [ ] `Ty::Ref` restricted to extern fn params only
- [ ] Host functions dispatch from eval.rs
- [ ] Capability validation at dispatch boundary
- [ ] main() receives capabilities from runtime
- [ ] Effect traces stream to JSONL
- [ ] Content hashing for large payloads (> 1KB)
- [ ] Deterministic replay from trace file
- [ ] Replay validates inputs match
- [ ] `strata run` and `strata replay` CLI commands
- [ ] Example program works end-to-end
- [ ] All tests pass, zero clippy warnings
- [ ] `docs/IMPLEMENTED.md` updated
- [ ] `docs/IN_PROGRESS.md` updated
