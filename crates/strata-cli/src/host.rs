//! Host function registry and implementations for Strata extern fns.
//!
//! When the interpreter encounters a call to an extern fn, it dispatches
//! to a Rust implementation registered here. Phase 3 adds structured
//! trace emission: every host call records effect, operation, capability
//! access, inputs, output (with SHA-256 hashing), and duration.

use std::collections::{BTreeMap, HashMap};
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
    /// Trace write failure — execution must abort
    TraceWriteError(String),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::UnknownFunction(name) => write!(f, "unknown host function: {}", name),
            HostError::TypeError(msg) => write!(f, "type error: {}", msg),
            HostError::IoError(msg) => write!(f, "I/O error: {}", msg),
            HostError::RuntimeError(msg) => write!(f, "runtime error: {}", msg),
            HostError::TraceWriteError(msg) => {
                write!(f, "trace write error (execution aborted): {}", msg)
            }
        }
    }
}

impl std::error::Error for HostError {}

// ---------------------------------------------------------------------------
// Trace data types
// ---------------------------------------------------------------------------

/// Tagged trace value — preserves type information across serialization.
///
/// Unlike the previous untyped `serialize_value()` approach, this enum
/// round-trips cleanly: `Int(42)` stays `Int(42)`, not ambiguous `"42"`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum TraceValue {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Unit,
}

impl TraceValue {
    /// Convert a runtime Value to a TraceValue.
    ///
    /// Panics on non-data values (Cap, HostFn, etc.) — those should never
    /// appear in trace inputs or outputs.
    pub fn from_value(val: &Value) -> Self {
        match val {
            Value::Int(n) => TraceValue::Int(*n),
            Value::Float(f) => TraceValue::Float(*f),
            Value::Str(s) => TraceValue::Str(s.clone()),
            Value::Bool(b) => TraceValue::Bool(*b),
            Value::Unit => TraceValue::Unit,
            other => TraceValue::Str(format!("{}", other)),
        }
    }

    /// Convert a TraceValue back to a runtime Value.
    pub fn to_value(&self) -> Value {
        match self {
            TraceValue::Int(n) => Value::Int(*n),
            TraceValue::Float(f) => Value::Float(*f),
            TraceValue::Str(s) => Value::Str(s.clone()),
            TraceValue::Bool(b) => Value::Bool(*b),
            TraceValue::Unit => Value::Unit,
        }
    }

    /// Serialize to a string for hashing purposes.
    fn to_hash_string(&self) -> String {
        match self {
            TraceValue::Int(n) => n.to_string(),
            TraceValue::Float(f) => f.to_string(),
            TraceValue::Str(s) => s.clone(),
            TraceValue::Bool(b) => b.to_string(),
            TraceValue::Unit => "()".to_string(),
        }
    }
}

/// A single trace entry recording one host function call.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TraceEntry {
    pub seq: u64,
    pub timestamp: String,
    pub effect: String,
    pub operation: String,
    pub capability: CapRef,
    pub inputs: BTreeMap<String, TraceValue>,
    pub output: TraceOutput,
    pub duration_ms: u64,
    /// Whether all values are stored (true) or large values are hashed (false).
    /// Replay requires full_values=true.
    #[serde(default)]
    pub full_values: bool,
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
    pub value: Option<TraceValue>,
    pub value_hash: String,
    pub value_size: usize,
}

// ---------------------------------------------------------------------------
// TraceRecord — versioned trace envelope
// ---------------------------------------------------------------------------

/// Current trace schema version.
pub const TRACE_SCHEMA_VERSION: &str = "0.1";

/// A trace record in the JSONL stream. Tagged enum wrapping header, effect
/// entries, and footer for schema versioning and completeness detection.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "record")]
pub enum TraceRecord {
    /// First line: schema version and metadata.
    #[serde(rename = "header")]
    Header(TraceHeader),
    /// Effect entry: one host function call.
    #[serde(rename = "effect")]
    Effect(TraceEntry),
    /// Last line: summary and completion status.
    #[serde(rename = "footer")]
    Footer(TraceFooter),
}

/// Trace header — first line of the JSONL stream.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TraceHeader {
    pub schema_version: String,
    pub timestamp: String,
    pub full_values: bool,
}

/// Trace footer — last line of the JSONL stream.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TraceFooter {
    pub timestamp: String,
    pub effect_count: u64,
    /// "complete" if finalize() was called normally, "incomplete" otherwise.
    pub trace_status: String,
    /// "success" or "error".
    pub program_status: String,
}

// ---------------------------------------------------------------------------
// TraceEmitter
// ---------------------------------------------------------------------------

/// Emits structured JSONL trace entries for host function calls.
///
/// Each trace is a stream of `TraceRecord` entries:
/// 1. Header (schema version, start time)
/// 2. Effect entries (one per host fn call)
/// 3. Footer (effect count, completion status)
///
/// Call `finalize()` when the program completes to write the footer.
pub struct TraceEmitter {
    seq: u64,
    writer: Option<Box<dyn Write + Send>>,
    full_values: bool,
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
    /// When `full_values` is true, all output values are recorded regardless
    /// of size (for replay-capable traces). When false, values > 1KB are
    /// replaced with their SHA-256 hash.
    ///
    /// Emits a header record immediately. Returns error if header write fails.
    pub fn new(mut writer: Box<dyn Write + Send>, full_values: bool) -> Result<Self, HostError> {
        // Write header as first record
        let header = TraceRecord::Header(TraceHeader {
            schema_version: TRACE_SCHEMA_VERSION.to_string(),
            timestamp: now_iso8601(),
            full_values,
        });
        let json = serde_json::to_string(&header)
            .map_err(|e| HostError::TraceWriteError(format!("serialize header: {}", e)))?;
        writeln!(writer, "{}", json)
            .map_err(|e| HostError::TraceWriteError(format!("write header: {}", e)))?;
        Ok(Self {
            seq: 0,
            writer: Some(writer),
            full_values,
        })
    }

    /// Create a disabled trace emitter (no output).
    pub fn disabled() -> Self {
        Self {
            seq: 0,
            writer: None,
            full_values: false,
        }
    }

    /// Whether this tracer records full values (no size-based hashing).
    pub fn full_values(&self) -> bool {
        self.full_values
    }

    /// Return the next sequence number and advance the counter.
    pub fn next_seq(&mut self) -> u64 {
        let s = self.seq;
        self.seq += 1;
        s
    }

    /// Emit a trace entry as a JSONL line, wrapped in TraceRecord::Effect.
    ///
    /// Returns error if serialization or writing fails — caller must abort.
    pub fn emit(&mut self, entry: TraceEntry) -> Result<(), HostError> {
        if let Some(ref mut w) = self.writer {
            let record = TraceRecord::Effect(entry);
            let json = serde_json::to_string(&record)
                .map_err(|e| HostError::TraceWriteError(format!("serialize effect: {}", e)))?;
            writeln!(w, "{}", json)
                .map_err(|e| HostError::TraceWriteError(format!("write effect: {}", e)))?;
        }
        Ok(())
    }

    /// Write the footer record and flush. Call this when the program completes.
    ///
    /// `program_status` should be "success" or "error".
    /// Returns error if serialization, writing, or flushing fails.
    pub fn finalize(&mut self, program_status: &str) -> Result<(), HostError> {
        if let Some(ref mut w) = self.writer {
            let footer = TraceRecord::Footer(TraceFooter {
                timestamp: now_iso8601(),
                effect_count: self.seq,
                trace_status: "complete".to_string(),
                program_status: program_status.to_string(),
            });
            let json = serde_json::to_string(&footer)
                .map_err(|e| HostError::TraceWriteError(format!("serialize footer: {}", e)))?;
            writeln!(w, "{}", json)
                .map_err(|e| HostError::TraceWriteError(format!("write footer: {}", e)))?;
            w.flush()
                .map_err(|e| HostError::TraceWriteError(format!("flush trace: {}", e)))?;
        }
        Ok(())
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
        let mut inputs = BTreeMap::new();

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
                    cap_access = if *borrowed { "borrow" } else { "consume" }.to_string();
                    effect_str = format!("{:?}", kind.gates_effect());
                }
                ParamKind::Data { name } => {
                    if let Some(val) = all_args.get(i) {
                        inputs.insert(name.clone(), TraceValue::from_value(val));
                        data_args.push(val.clone());
                    }
                }
            }
        }

        let start = std::time::Instant::now();
        let result = self.call(name, &data_args, tracer);
        let duration = start.elapsed();

        let full = tracer.full_values();
        let (status, output_value, output_hash, output_size) = match &result {
            Ok(val) => {
                let tv = TraceValue::from_value(val);
                let hash_str = tv.to_hash_string();
                let hash = sha256_hex(&hash_str);
                let size = hash_str.len();
                let value = if full || size <= 1024 { Some(tv) } else { None };
                ("ok", value, hash, size)
            }
            Err(e) => {
                let err_str = e.to_string();
                let hash = sha256_hex(&err_str);
                let size = err_str.len();
                ("error", Some(TraceValue::Str(err_str)), hash, size)
            }
        };

        // In audit mode (not full_values), hash large input values
        if !full {
            for tv in inputs.values_mut() {
                let s = tv.to_hash_string();
                if s.len() > 1024 {
                    *tv = TraceValue::Str(sha256_hex(&s));
                }
            }
        }

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
            inputs,
            output: TraceOutput {
                status: status.to_string(),
                value: output_value,
                value_hash: output_hash,
                value_size: output_size,
            },
            duration_ms: duration.as_millis() as u64,
            full_values: full,
        })?;

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
    /// Trace was recorded in audit mode and cannot be replayed.
    NotReplayable { seq: u64, reason: String },
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
            ReplayError::NotReplayable { seq, reason } => {
                write!(
                    f,
                    "replay: trace entry at seq {} is not replayable: {}",
                    seq, reason
                )
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
    trace_complete: bool,
}

impl TraceReplayer {
    /// Load a trace from JSONL content (one JSON object per line).
    ///
    /// Parses the `TraceRecord` envelope, extracts effect entries, and
    /// validates the header for replay capability.
    pub fn from_jsonl(content: &str) -> Result<Self, ReplayError> {
        let mut entries = Vec::new();
        let mut saw_header = false;
        let mut saw_footer = false;

        for (i, line) in content.lines().filter(|l| !l.is_empty()).enumerate() {
            // Try to parse as TraceRecord first (versioned format)
            if let Ok(record) = serde_json::from_str::<TraceRecord>(line) {
                match record {
                    TraceRecord::Header(h) => {
                        saw_header = true;
                        // Reject unknown schema versions
                        if h.schema_version != TRACE_SCHEMA_VERSION {
                            return Err(ReplayError::NotReplayable {
                                seq: 0,
                                reason: format!(
                                    "unsupported trace schema version '{}' (expected '{}')",
                                    h.schema_version, TRACE_SCHEMA_VERSION
                                ),
                            });
                        }
                        if !h.full_values {
                            return Err(ReplayError::NotReplayable {
                                seq: 0,
                                reason: "trace was recorded with --trace (audit mode). \
                                         Re-run with --trace-full for replay-capable traces"
                                    .to_string(),
                            });
                        }
                    }
                    TraceRecord::Effect(entry) => {
                        entries.push(entry);
                    }
                    TraceRecord::Footer(_) => {
                        saw_footer = true;
                    }
                }
            } else {
                // Fallback: try parsing as a bare TraceEntry (pre-versioning format)
                let entry: TraceEntry = serde_json::from_str(line)
                    .map_err(|e| ReplayError::ParseError(i, e.to_string()))?;

                // Reject audit-mode entries
                if !entry.full_values {
                    return Err(ReplayError::NotReplayable {
                        seq: entry.seq,
                        reason: "trace was recorded with --trace (audit mode). \
                                 Re-run with --trace-full for replay-capable traces"
                            .to_string(),
                    });
                }
                entries.push(entry);
            }
        }

        // If we saw a header, this is a versioned trace — good
        // If not, it's a legacy trace (pre-versioning) — still works
        let _ = saw_header;

        Ok(Self {
            entries,
            cursor: 0,
            trace_complete: saw_footer,
        })
    }

    /// Replay the next extern call. Validates operation and inputs match
    /// the trace, then returns the recorded output.
    pub fn next(
        &mut self,
        operation: &str,
        inputs: &BTreeMap<String, TraceValue>,
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
                expected: serde_json::to_value(&entry.inputs).unwrap_or_default(),
                actual: serde_json::to_value(inputs).unwrap_or_default(),
            });
        }

        self.cursor += 1;

        match entry.output.status.as_str() {
            "ok" => {
                let tv = entry
                    .output
                    .value
                    .as_ref()
                    .ok_or_else(|| ReplayError::MissingValue {
                        operation: operation.to_string(),
                        seq: (self.cursor - 1) as u64,
                        value_size: entry.output.value_size,
                    })?;
                Ok(tv.to_value())
            }
            "error" => {
                let err_msg = entry
                    .output
                    .value
                    .as_ref()
                    .map(|tv| tv.to_hash_string())
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

    /// Whether the trace included a footer record (indicating clean completion).
    /// A missing footer means the trace may be truncated.
    pub fn is_trace_complete(&self) -> bool {
        self.trace_complete
    }
}

// Note: deserialize_value() removed in Fix 2 — replaced by TraceValue::to_value().

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

// Note: serialize_value() removed in Fix 2 — replaced by TraceValue::from_value().

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
        _ => {
            return Err(HostError::TypeError(
                "read_file: expected String path".into(),
            ))
        }
    };
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Value::Str(content)),
        Err(e) => Err(HostError::IoError(format!("read_file: {}", e))),
    }
}

fn host_write_file(args: &[Value], _tracer: &mut TraceEmitter) -> Result<Value, HostError> {
    let path = match args.first() {
        Some(Value::Str(s)) => s,
        _ => {
            return Err(HostError::TypeError(
                "write_file: expected String path".into(),
            ))
        }
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
