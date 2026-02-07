//! Host function registry and implementations for Strata extern fns.
//!
//! When the interpreter encounters a call to an extern fn, it dispatches
//! to a Rust implementation registered here. Capability args are stripped
//! at dispatch time — host functions only receive data args.

use std::collections::HashMap;

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

/// No-op trace emitter stub for Phase 2.
/// Phase 3 will replace this with real trace recording.
pub struct TraceEmitter;

impl Default for TraceEmitter {
    fn default() -> Self {
        TraceEmitter
    }
}

impl TraceEmitter {
    pub fn new() -> Self {
        TraceEmitter
    }
}

/// Type alias for host function signatures.
pub type HostFnImpl = fn(&[Value], &mut TraceEmitter) -> Result<Value, HostError>;

/// Registry mapping extern fn names to Rust implementations.
///
/// Debug is manually implemented because function pointers don't derive Debug.
pub struct HostRegistry {
    functions: HashMap<String, HostFnImpl>,
}

impl std::fmt::Debug for HostRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<_> = self.functions.keys().collect();
        f.debug_struct("HostRegistry")
            .field("functions", &names)
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

    /// Dispatch a host function call by name.
    pub fn call(
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
}

// --- Host function implementations ---

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
