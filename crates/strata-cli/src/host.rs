//! Host function registry and implementations for Strata extern fns.
//!
//! When the interpreter encounters a call to an extern fn, it dispatches
//! to a Rust implementation registered here. Phase 3 adds structured
//! trace emission: every host call records effect, operation, capability
//! access, inputs, output (with SHA-256 hashing), and duration.

use std::collections::HashMap;
use std::io::Write;

use strata_types::CapKind;

use crate::eval::Value;

/// Errors from host function execution.
#[derive(Debug)]
pub enum HostError {
    /// Extern fn name not found in registry
    UnknownFunction(String),
    /// Argument type mismatch at runtime
    TypeError(String),
    /// I/O error from host operation
    IoError(String),
    /// General runtime error
    RuntimeError(String),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::UnknownFunction(name) => write!(f, "unknown host function: {}", name),
            HostError::TypeError(msg) => write!(f, "type error: {}", msg),
            HostError::IoError(msg) => write!(f, "I/O error: {}", msg),
            HostError::RuntimeError(msg) => write!(f, "runtime error: {}", msg),
        }
    }
}

impl std::error::Error for HostError {}

// ---------------------------------------------------------------------------
// Trace data types
// ---------------------------------------------------------------------------

/// A single trace entry recording one host function call.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
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

/// Reference to the capability used in a host call.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CapRef {
    pub kind: String,
    pub access: String,
}

/// Output section of a trace entry.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TraceOutput {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub value_hash: String,
    pub value_size: usize,
}

// ---------------------------------------------------------------------------
// TraceEmitter
// ---------------------------------------------------------------------------

/// Emits structured JSONL trace entries for host function calls.
///
/// When created with a writer, each `emit()` serializes a `TraceEntry` as
/// one JSON line. When disabled (no writer), calls are counted but nothing
/// is written.
pub struct TraceEmitter {
    seq: u64,
    writer: Option<Box<dyn Write + Send>>,
}

impl std::fmt::Debug for TraceEmitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceEmitter")
            .field("seq", &self.seq)
            .field("enabled", &self.writer.is_some())
            .finish()
    }
}

impl Default for TraceEmitter {
    fn default() -> Self {
        Self::disabled()
    }
}

impl TraceEmitter {
    /// Create a trace emitter that writes JSONL to the given writer.
    pub fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            seq: 0,
            writer: Some(writer),
        }
    }

    /// Create a disabled trace emitter (no output).
    pub fn disabled() -> Self {
        Self {
            seq: 0,
            writer: None,
        }
    }

    /// Return the next sequence number and advance the counter.
    pub fn next_seq(&mut self) -> u64 {
        let s = self.seq;
        self.seq += 1;
        s
    }

    /// Emit a trace entry as a JSONL line.
    pub fn emit(&mut self, entry: TraceEntry) {
        if let Some(ref mut w) = self.writer {
            if let Ok(json) = serde_json::to_string(&entry) {
                let _ = writeln!(w, "{}", json);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ExternFnMeta — positional param metadata
// ---------------------------------------------------------------------------

/// Metadata about a single extern fn parameter (cap or data).
#[derive(Debug, Clone)]
pub enum ParamKind {
    /// Capability parameter — records kind and borrow/consume access.
    Cap { kind: CapKind, borrowed: bool },
    /// Data parameter — records the param name for trace inputs.
    Data { name: String },
}

/// Metadata for an extern fn's parameter list, derived from its type signature.
#[derive(Debug, Clone)]
pub struct ExternFnMeta {
    pub params: Vec<ParamKind>,
}

// ---------------------------------------------------------------------------
// HostRegistry
// ---------------------------------------------------------------------------

/// Type alias for host function signatures.
pub type HostFnImpl = fn(&[Value], &mut TraceEmitter) -> Result<Value, HostError>;

/// Registry mapping extern fn names to Rust implementations.
pub struct HostRegistry {
    functions: HashMap<String, HostFnImpl>,
    extern_meta: HashMap<String, ExternFnMeta>,
}

impl std::fmt::Debug for HostRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<_> = self.functions.keys().collect();
        f.debug_struct("HostRegistry")
            .field("functions", &names)
            .field("extern_meta_count", &self.extern_meta.len())
            .finish()
    }
}

impl Default for HostRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HostRegistry {
    /// Create a new registry with all built-in host functions.
    pub fn new() -> Self {
        let mut reg = Self {
            functions: HashMap::new(),
            extern_meta: HashMap::new(),
        };
        reg.register("read_file", host_read_file);
        reg.register("write_file", host_write_file);
        reg.register("now", host_now);
        reg.register("random_int", host_random_int);
        reg
    }

    fn register(&mut self, name: &str, f: HostFnImpl) {
        self.functions.insert(name.to_string(), f);
    }

    /// Register positional param metadata for an extern fn.
    pub fn register_extern_meta(&mut self, name: &str, meta: ExternFnMeta) {
        self.extern_meta.insert(name.to_string(), meta);
    }

    /// Look up extern fn metadata.
    pub fn get_extern_meta(&self, name: &str) -> Option<&ExternFnMeta> {
        self.extern_meta.get(name)
    }

    /// Dispatch a host function call by name (data args only).
    /// Used internally by dispatch_traced() after cap/data separation.
    fn call(
        &self,
        name: &str,
        args: &[Value],
        tracer: &mut TraceEmitter,
    ) -> Result<Value, HostError> {
        let f = self
            .functions
            .get(name)
            .ok_or_else(|| HostError::UnknownFunction(name.to_string()))?;
        f(args, tracer)
    }

    /// Dispatch with trace emission.
    ///
    /// Walks the extern fn's type signature metadata to determine which
    /// positional args are capabilities vs data, extracts named inputs,
    /// calls the host function with data-only args, and emits a trace entry.
    pub fn dispatch_traced(
        &self,
        name: &str,
        all_args: &[Value],
        tracer: &mut TraceEmitter,
    ) -> Result<Value, HostError> {
        let meta = self.extern_meta.get(name);

        let mut cap_kind_str = String::new();
        let mut cap_access = String::new();
        let mut effect_str = String::new();
        let mut data_args = Vec::new();
        let mut inputs = serde_json::Map::new();

        let meta = meta.ok_or_else(|| {
            HostError::RuntimeError(format!(
                "no ExternFnMeta registered for '{}' — all extern fns must have metadata",
                name
            ))
        })?;

        for (i, param) in meta.params.iter().enumerate() {
            match param {
                ParamKind::Cap { kind, borrowed } => {
                    cap_kind_str = kind.type_name().to_string();
                    cap_access =
                        if *borrowed { "borrow" } else { "consume" }.to_string();
                    effect_str = format!("{:?}", kind.gates_effect());
                }
                ParamKind::Data { name } => {
                    if let Some(val) = all_args.get(i) {
                        inputs.insert(
                            name.clone(),
                            serde_json::Value::String(serialize_value(val)),
                        );
                        data_args.push(val.clone());
                    }
                }
            }
        }

        let start = std::time::Instant::now();
        let result = self.call(name, &data_args, tracer);
        let duration = start.elapsed();

        let (status, output_value, output_hash, output_size) = match &result {
            Ok(val) => {
                let serialized = serialize_value(val);
                let hash = sha256_hex(&serialized);
                let size = serialized.len();
                let value = if size <= 1024 {
                    Some(serialized)
                } else {
                    None
                };
                ("ok", value, hash, size)
            }
            Err(e) => {
                let err_str = e.to_string();
                let hash = sha256_hex(&err_str);
                let size = err_str.len();
                ("error", Some(err_str), hash, size)
            }
        };

        let seq = tracer.next_seq();
        tracer.emit(TraceEntry {
            seq,
            timestamp: now_iso8601(),
            effect: effect_str,
            operation: name.to_string(),
            capability: CapRef {
                kind: cap_kind_str,
                access: cap_access,
            },
            inputs: serde_json::Value::Object(inputs),
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
}

// ---------------------------------------------------------------------------
// TraceReplayer — deterministic replay from recorded traces
// ---------------------------------------------------------------------------

/// Errors from trace replay.
#[derive(Debug)]
pub enum ReplayError {
    /// Program performed an extern call not present in the trace.
    UnexpectedEffect(String),
    /// Extern call order diverged from the trace.
    OperationMismatch {
        expected: String,
        actual: String,
        seq: u64,
    },
    /// Inputs to an extern call diverged from the trace.
    InputMismatch {
        operation: String,
        seq: u64,
        expected: serde_json::Value,
        actual: serde_json::Value,
    },
    /// Output was hashed (>1KB), cannot replay without full value.
    MissingValue {
        operation: String,
        seq: u64,
        value_size: usize,
    },
    /// Trace has entries that were never replayed.
    UnreplayedEffects(usize),
    /// The trace recorded an error; replay returns it.
    ReplayedError(String),
    /// Unknown status in trace entry.
    UnknownStatus(String),
    /// JSONL parse error.
    ParseError(usize, String),
    /// I/O error reading trace file.
    Io(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::UnexpectedEffect(op) => {
                write!(f, "replay: unexpected extern call '{}' not in trace", op)
            }
            ReplayError::OperationMismatch {
                expected,
                actual,
                seq,
            } => write!(
                f,
                "replay: operation mismatch at seq {}: expected '{}', got '{}'",
                seq, expected, actual
            ),
            ReplayError::InputMismatch {
                operation,
                seq,
                expected,
                actual,
            } => write!(
                f,
                "replay: input mismatch for '{}' at seq {}: expected {}, got {}",
                operation, seq, expected, actual
            ),
            ReplayError::MissingValue {
                operation,
                seq,
                value_size,
            } => write!(
                f,
                "cannot replay: output for '{}' at seq {} was hashed ({} bytes). \
                 Re-run with --trace-full to record complete values",
                operation, seq, value_size
            ),
            ReplayError::UnreplayedEffects(n) => {
                write!(f, "replay: trace has {} unreplayed entries", n)
            }
            ReplayError::ReplayedError(msg) => write!(f, "{}", msg),
            ReplayError::UnknownStatus(s) => {
                write!(f, "replay: unknown status '{}' in trace", s)
            }
            ReplayError::ParseError(line, msg) => {
                write!(f, "replay: parse error at line {}: {}", line, msg)
            }
            ReplayError::Io(msg) => write!(f, "replay: I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Replays a previously recorded trace, substituting recorded outputs
/// instead of calling real host functions.
#[derive(Debug)]
pub struct TraceReplayer {
    entries: Vec<TraceEntry>,
    cursor: usize,
}

impl TraceReplayer {
    /// Load a trace from JSONL content (one JSON object per line).
    pub fn from_jsonl(content: &str) -> Result<Self, ReplayError> {
        let entries: Vec<TraceEntry> = content
            .lines()
            .filter(|l| !l.is_empty())
            .enumerate()
            .map(|(i, line)| {
                serde_json::from_str(line)
                    .map_err(|e| ReplayError::ParseError(i, e.to_string()))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { entries, cursor: 0 })
    }

    /// Replay the next extern call. Validates operation and inputs match
    /// the trace, then returns the recorded output.
    pub fn next(
        &mut self,
        operation: &str,
        inputs: &serde_json::Value,
    ) -> Result<Value, ReplayError> {
        let entry = self
            .entries
            .get(self.cursor)
            .ok_or_else(|| ReplayError::UnexpectedEffect(operation.to_string()))?;

        if entry.operation != operation {
            return Err(ReplayError::OperationMismatch {
                expected: entry.operation.clone(),
                actual: operation.to_string(),
                seq: self.cursor as u64,
            });
        }

        if entry.inputs != *inputs {
            return Err(ReplayError::InputMismatch {
                operation: operation.to_string(),
                seq: self.cursor as u64,
                expected: entry.inputs.clone(),
                actual: inputs.clone(),
            });
        }

        self.cursor += 1;

        match entry.output.status.as_str() {
            "ok" => {
                let value_str = entry.output.value.as_ref().ok_or_else(|| {
                    ReplayError::MissingValue {
                        operation: operation.to_string(),
                        seq: (self.cursor - 1) as u64,
                        value_size: entry.output.value_size,
                    }
                })?;
                Ok(deserialize_value(value_str))
            }
            "error" => {
                let err_msg = entry
                    .output
                    .value
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| "unknown error".to_string());
                Err(ReplayError::ReplayedError(err_msg))
            }
            other => Err(ReplayError::UnknownStatus(other.to_string())),
        }
    }

    /// Verify that all trace entries were replayed.
    pub fn verify_complete(&self) -> Result<(), ReplayError> {
        if self.cursor < self.entries.len() {
            Err(ReplayError::UnreplayedEffects(
                self.entries.len() - self.cursor,
            ))
        } else {
            Ok(())
        }
    }
}

/// Deserialize a trace value string back into a runtime Value.
///
/// This is a best-effort inverse of serialize_value(). Since the trace
/// format is lossy (all values become strings), we attempt to parse
/// Int, Float, Bool, and Unit, falling back to String.
fn deserialize_value(s: &str) -> Value {
    if s == "()" {
        return Value::Unit;
    }
    if s == "true" {
        return Value::Bool(true);
    }
    if s == "false" {
        return Value::Bool(false);
    }
    if let Ok(n) = s.parse::<i64>() {
        return Value::Int(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        // Only treat as float if it contains a dot (avoid "42" -> 42.0)
        if s.contains('.') {
            return Value::Float(f);
        }
    }
    Value::Str(s.to_string())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute SHA-256 hex digest of a string, prefixed with "sha256:".
fn sha256_hex(data: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Serialize a runtime Value to a compact string for trace output.
pub fn serialize_value(val: &Value) -> String {
    match val {
        Value::Str(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Unit => "()".to_string(),
        other => format!("{}", other),
    }
}

/// Produce an ISO 8601 UTC timestamp without external dependencies.
///
/// Uses the standard civil-from-days algorithm to convert epoch seconds
/// to year-month-day.
fn now_iso8601() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();

    // Civil date from days since epoch (algorithm from Howard Hinnant)
    let days = (secs / 86400) as i64;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    let rem = secs % 86400;
    let hours = rem / 3600;
    let mins = (rem % 3600) / 60;
    let s = rem % 60;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y, m, d, hours, mins, s, millis
    )
}

// ---------------------------------------------------------------------------
// Host function implementations
// ---------------------------------------------------------------------------

fn host_read_file(args: &[Value], _tracer: &mut TraceEmitter) -> Result<Value, HostError> {
    let path = match args.first() {
        Some(Value::Str(s)) => s,
        _ => return Err(HostError::TypeError("read_file: expected String path".into())),
    };
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Value::Str(content)),
        Err(e) => Err(HostError::IoError(format!("read_file: {}", e))),
    }
}

fn host_write_file(args: &[Value], _tracer: &mut TraceEmitter) -> Result<Value, HostError> {
    let path = match args.first() {
        Some(Value::Str(s)) => s,
        _ => return Err(HostError::TypeError("write_file: expected String path".into())),
    };
    let content = match args.get(1) {
        Some(Value::Str(s)) => s,
        _ => {
            return Err(HostError::TypeError(
                "write_file: expected String content".into(),
            ))
        }
    };
    match std::fs::write(path, content) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => Err(HostError::IoError(format!("write_file: {}", e))),
    }
}

fn host_now(_args: &[Value], _tracer: &mut TraceEmitter) -> Result<Value, HostError> {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| HostError::RuntimeError(format!("now: {}", e)))?;
    Ok(Value::Str(format!(
        "{}.{:03}",
        now.as_secs(),
        now.subsec_millis()
    )))
}

fn host_random_int(_args: &[Value], _tracer: &mut TraceEmitter) -> Result<Value, HostError> {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| HostError::RuntimeError(format!("random_int: {}", e)))?
        .subsec_nanos();
    Ok(Value::Int((seed % 1000) as i64))
}
