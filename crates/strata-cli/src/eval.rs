//! Evaluator for Strata programs
//!
//! Implements a tree-walking interpreter with proper scoping,
//! closures, and control flow (return, break, continue).

use anyhow::{bail, Result};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use strata_ast::ast::{
    BinOp, Block, Expr, FieldInit, Lit, MatchArm, Module, Pat, Path, Stmt, UnOp,
};
use strata_ast::span::Span;
use strata_types::CapKind;

use crate::host::{
    ExternFnMeta, HostRegistry, ParamKind, ReplayError, TraceEmitter, TraceReplayer, TraceValue,
};

/// Maximum call depth to prevent stack overflow from deep recursion
const MAX_CALL_DEPTH: u32 = 1000;

thread_local! {
    /// Current call depth (thread-local for safety)
    static CALL_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Runtime values in Strata
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
    /// Function closure capturing its environment
    Closure {
        params: Vec<String>,
        body: Block,
        env: Env,
    },
    /// Tuple value: (a, b, c)
    Tuple(Vec<Value>),
    /// Struct value: Point { x: 1, y: 2 }
    Struct {
        name: String,
        fields: HashMap<String, Value>,
    },
    /// Enum variant value: Some(42) or None
    Variant {
        enum_name: String,
        variant_name: String,
        fields: Vec<Value>,
    },
    /// Runtime capability token
    Cap(CapKind),
    /// Host function reference (extern fn name)
    HostFn(String),
    /// Tombstone: affine value already moved. Runtime defense-in-depth.
    Consumed {
        var_name: String,
        moved_at: Span,
    },
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::Str(s) => write!(f, "\"{s}\""),
            Value::Unit => write!(f, "()"),
            Value::Closure { params, .. } => write!(f, "<fn({})>", params.join(", ")),
            Value::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
            Value::Struct { name, fields } => {
                write!(f, "{} {{ ", name)?;
                let mut first = true;
                // Sort fields for deterministic output
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(k, _)| *k);
                for (field_name, value) in sorted_fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", field_name, value)?;
                    first = false;
                }
                write!(f, " }}")
            }
            Value::Variant {
                enum_name,
                variant_name,
                fields,
            } => {
                write!(f, "{}::{}", enum_name, variant_name)?;
                if !fields.is_empty() {
                    write!(f, "(")?;
                    for (i, field) in fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", field)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::Cap(kind) => write!(f, "<cap:{}>", kind.type_name()),
            Value::HostFn(name) => write!(f, "<host_fn:{}>", name),
            Value::Consumed { var_name, .. } => write!(f, "<consumed:{}>", var_name),
        }
    }
}

impl Value {
    /// Returns true if this value has affine semantics (single-use).
    ///
    /// A value is affine if it IS a capability or CONTAINS one.
    /// This mirrors Ty::kind() propagation at the type level — the same
    /// principle that caught the v0.0.10.1 kind propagation bypass now
    /// operates at the runtime value level as defense-in-depth.
    fn is_affine(&self) -> bool {
        match self {
            Value::Cap(_) => true,
            Value::Tuple(elems) => elems.iter().any(|v| v.is_affine()),
            Value::Struct { fields, .. } => fields.values().any(|v| v.is_affine()),
            Value::Variant { fields, .. } => fields.iter().any(|v| v.is_affine()),
            // Closures: NOT affine in v0.1. All closures are module-level function
            // definitions. The static move checker bans closure capture of affine
            // values. Making closures affine here would break every function call.
            // Revisit when user-defined closures/lambdas are added (v0.2+).
            _ => false,
        }
    }
}

/// Control flow for evaluation
///
/// Used to propagate returns through blocks and function calls.
#[derive(Debug, Clone)]
pub enum ControlFlow {
    /// Normal value result
    Value(Value),
    /// Return statement - bubbles up to function boundary
    Return(Value),
    /// Break statement - reserved for future loop control
    #[allow(dead_code)]
    Break,
    /// Continue statement - reserved for future loop control
    #[allow(dead_code)]
    Continue,
}

impl ControlFlow {
    /// Extract the value, treating Return as a normal value
    pub fn into_value(self) -> Value {
        match self {
            ControlFlow::Value(v) | ControlFlow::Return(v) => v,
            ControlFlow::Break | ControlFlow::Continue => Value::Unit,
        }
    }

    /// Check if this is a Return
    pub fn is_return(&self) -> bool {
        matches!(self, ControlFlow::Return(_))
    }
}

/// A variable binding with mutability tracking
#[derive(Debug, Clone)]
struct Binding {
    value: Value,
    mutable: bool,
}

/// Environment with lexical scoping
///
/// Uses a stack of scopes for proper variable shadowing and block scoping.
#[derive(Debug, Clone)]
pub struct Env {
    scopes: Vec<HashMap<String, Binding>>,
    host_registry: Option<Arc<HostRegistry>>,
    tracer: Option<Arc<Mutex<TraceEmitter>>>,
    replayer: Option<Arc<Mutex<TraceReplayer>>>,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            host_registry: None,
            tracer: None,
            replayer: None,
        }
    }
}

impl Env {
    /// Create a new environment with a single empty scope
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new environment with a host function registry
    pub fn with_host_registry(registry: Arc<HostRegistry>) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            host_registry: Some(registry),
            tracer: None,
            replayer: None,
        }
    }

    /// Attach a trace emitter to this environment.
    pub fn with_tracer(mut self, tracer: Arc<Mutex<TraceEmitter>>) -> Self {
        self.tracer = Some(tracer);
        self
    }

    /// Attach a trace replayer to this environment.
    pub fn with_replayer(mut self, replayer: Arc<Mutex<TraceReplayer>>) -> Self {
        self.replayer = Some(replayer);
        self
    }

    /// Push a new scope onto the stack
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope off the stack
    ///
    /// Returns an error if attempting to pop the global scope.
    pub fn pop_scope(&mut self) -> anyhow::Result<()> {
        if self.scopes.len() <= 1 {
            anyhow::bail!("internal error: attempted to pop global scope");
        }
        self.scopes.pop();
        Ok(())
    }

    /// Execute a closure with a new scope that is automatically popped on exit.
    /// This ensures the scope is popped even if the closure returns an error.
    pub fn with_scope<T>(&mut self, f: impl FnOnce(&mut Env) -> Result<T>) -> Result<T> {
        self.push_scope();
        let result = f(self);
        // Always pop scope, even on error - ignore pop_scope result since
        // we know we just pushed a scope so it can't fail
        let _ = self.pop_scope();
        result
    }

    /// Define a new variable in the current scope
    pub fn define(&mut self, name: String, value: Value, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, Binding { value, mutable });
        }
    }

    /// Look up a variable by name, searching from innermost to outermost scope
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .map(|b| &b.value)
    }

    /// Destructive read: take the value out and leave a Consumed tombstone.
    /// Traverses scopes in reverse order (like `get`) and tombstones at the
    /// ACTUAL binding depth — never inserts a shadow in the current scope.
    /// This prevents the scope-pop resurrection exploit.
    pub fn move_out(&mut self, name: &str, span: Span) -> Option<Value> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                let val = std::mem::replace(
                    &mut binding.value,
                    Value::Consumed {
                        var_name: name.to_string(),
                        moved_at: span,
                    },
                );
                return Some(val);
            }
        }
        None
    }

    /// Set a variable's value, respecting mutability
    pub fn set(&mut self, name: &str, value: Value) -> Result<()> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                if !binding.mutable {
                    bail!("cannot assign to immutable variable `{}`", name);
                }
                binding.value = value;
                return Ok(());
            }
        }
        bail!("undefined variable `{}`", name)
    }
}

/// Evaluate an entire module
pub fn eval_module(m: &Module) -> Result<()> {
    use strata_ast::ast::Item;

    let mut env = Env::new();

    // Collect function declarations
    let fn_decls: Vec<_> = m
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Fn(decl) = item {
                Some(decl)
            } else {
                None
            }
        })
        .collect();

    // Pass 0: Register extern fns as host function references
    for item in &m.items {
        if let Item::ExternFn(decl) = item {
            env.define(
                decl.name.text.clone(),
                Value::HostFn(decl.name.text.clone()),
                false,
            );
        }
    }

    // Pass 1: Define all function names as mutable placeholders
    // This allows forward references and self-references
    for decl in &fn_decls {
        env.define(decl.name.text.clone(), Value::Unit, true);
    }

    // Pass 2: Create closures that capture env with all names defined
    // NOTE (Issue 012): Closure capture currently clones the entire env.
    // In v0.2 (Issue 016), closure creation should hollow affine vars from
    // the parent scope. For v0.1, the static move checker prevents double-use
    // and runtime tombstoning in Expr::Var provides defense-in-depth.
    for decl in &fn_decls {
        let closure = Value::Closure {
            params: decl.params.iter().map(|p| p.name.text.clone()).collect(),
            body: decl.body.clone(),
            env: env.clone(),
        };
        env.set(&decl.name.text, closure).ok();
    }

    // Pass 3: Re-create closures to capture env with actual closures (not placeholders)
    // This enables recursion - each closure's captured env now contains all functions
    for decl in &fn_decls {
        let closure = Value::Closure {
            params: decl.params.iter().map(|p| p.name.text.clone()).collect(),
            body: decl.body.clone(),
            env: env.clone(),
        };
        env.set(&decl.name.text, closure).ok();
    }

    // Pass 4: Evaluate let bindings
    for item in &m.items {
        if let Item::Let(ld) = item {
            let cf = eval_expr(&mut env, &ld.value)?;
            let v = cf.into_value();
            println!("{} = {}", ld.name.text, v);
            env.define(ld.name.text.clone(), v, false);
        }
    }

    // Pass 5: Call main() if it exists and print result
    if let Some(main_val) = env.get("main") {
        if let Value::Closure {
            body,
            env: closure_env,
            ..
        } = main_val.clone()
        {
            let mut call_env = closure_env;
            let result = eval_block(&mut call_env, &body)?;
            let v = result.into_value();
            println!("main() = {}", v);
        }
    }

    Ok(())
}

/// Extract a capability type name from a TypeExpr.
///
/// Returns the type name for `FsCap` (from `TypeExpr::Path`) or `&FsCap`
/// (from `TypeExpr::Ref(TypeExpr::Path(...))`).
fn extract_cap_type_name(ty: &strata_ast::ast::TypeExpr) -> Option<String> {
    use strata_ast::ast::TypeExpr;
    match ty {
        TypeExpr::Path(segments, _) if segments.len() == 1 => Some(segments[0].text.clone()),
        TypeExpr::Ref(inner, _) => extract_cap_type_name(inner),
        _ => None,
    }
}

/// Run a module with host function dispatch and main() capability injection.
///
/// This is the primary entry point for programs that use capabilities.
/// No trace output is produced.
pub fn run_module(m: &Module) -> Result<Value> {
    run_module_inner(m, None, false)
}

/// Run a module with host function dispatch, capability injection, and
/// JSONL trace output written to the provided writer.
/// Values > 1KB are hashed (not suitable for replay).
pub fn run_module_traced(m: &Module, writer: Box<dyn std::io::Write + Send>) -> Result<Value> {
    run_module_inner(m, Some(writer), false)
}

/// Run a module with full trace output (all values recorded, no hashing).
/// The resulting trace is suitable for deterministic replay.
pub fn run_module_traced_full(m: &Module, writer: Box<dyn std::io::Write + Send>) -> Result<Value> {
    run_module_inner(m, Some(writer), true)
}

fn run_module_inner(
    m: &Module,
    trace_writer: Option<Box<dyn std::io::Write + Send>>,
    full_values: bool,
) -> Result<Value> {
    use strata_ast::ast::Item;

    let mut registry = HostRegistry::new();

    // Build ExternFnMeta from extern fn declarations and register host fn refs
    for item in &m.items {
        if let Item::ExternFn(decl) = item {
            let mut params = Vec::new();
            for param in &decl.params {
                if let Some(ty_expr) = &param.ty {
                    let (is_ref, cap_name) = extract_cap_info(ty_expr);
                    if let Some(name) = cap_name {
                        if let Some(kind) = CapKind::from_name(&name) {
                            params.push(ParamKind::Cap {
                                kind,
                                borrowed: is_ref,
                            });
                            continue;
                        }
                    }
                }
                params.push(ParamKind::Data {
                    name: param.name.text.clone(),
                });
            }
            registry.register_extern_meta(&decl.name.text, ExternFnMeta { params });
        }
    }

    let registry = Arc::new(registry);

    let tracer = trace_writer
        .map(|w| TraceEmitter::new(w, full_values))
        .transpose()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .map(|t| Arc::new(Mutex::new(t)));

    let mut env = Env::with_host_registry(registry);
    if let Some(t) = tracer {
        env = env.with_tracer(t);
    }

    // Register extern fns as host function references
    for item in &m.items {
        if let Item::ExternFn(decl) = item {
            env.define(
                decl.name.text.clone(),
                Value::HostFn(decl.name.text.clone()),
                false,
            );
        }
    }

    // Collect and register Strata function declarations
    let fn_decls: Vec<_> = m
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Fn(decl) = item {
                Some(decl)
            } else {
                None
            }
        })
        .collect();

    // Pass 1: Define all function names as mutable placeholders
    for decl in &fn_decls {
        env.define(decl.name.text.clone(), Value::Unit, true);
    }

    // Pass 2: Create closures that capture env with all names defined
    for decl in &fn_decls {
        let closure = Value::Closure {
            params: decl.params.iter().map(|p| p.name.text.clone()).collect(),
            body: decl.body.clone(),
            env: env.clone(),
        };
        env.set(&decl.name.text, closure).ok();
    }

    // Pass 3: Re-create closures for recursion support
    for decl in &fn_decls {
        let closure = Value::Closure {
            params: decl.params.iter().map(|p| p.name.text.clone()).collect(),
            body: decl.body.clone(),
            env: env.clone(),
        };
        env.set(&decl.name.text, closure).ok();
    }

    // Pass 4: Evaluate let bindings
    for item in &m.items {
        if let Item::Let(ld) = item {
            let cf = eval_expr(&mut env, &ld.value)?;
            let v = cf.into_value();
            env.define(ld.name.text.clone(), v, false);
        }
    }

    // Pass 5: Find main() and call with injected capabilities
    let main_decl = m.items.iter().find_map(|item| {
        if let Item::Fn(decl) = item {
            if decl.name.text == "main" {
                Some(decl)
            } else {
                None
            }
        } else {
            None
        }
    });

    let main_decl = match main_decl {
        Some(d) => d,
        None => return Ok(Value::Unit),
    };

    // Build capability args from main()'s param type annotations
    let mut cap_args: Vec<Value> = Vec::new();
    for param in &main_decl.params {
        if let Some(ty_expr) = &param.ty {
            if let Some(name) = extract_cap_type_name(ty_expr) {
                if let Some(kind) = CapKind::from_name(&name) {
                    cap_args.push(Value::Cap(kind));
                }
            }
        }
    }

    // Call main with cap args
    let main_val = env
        .get("main")
        .ok_or_else(|| anyhow::anyhow!("main function not found"))?
        .clone();

    if let Value::Closure {
        params,
        body,
        env: closure_env,
    } = main_val
    {
        let mut call_env = closure_env;
        call_env.push_scope();

        // Bind parameters to capability arguments
        for (param, value) in params.iter().zip(cap_args) {
            call_env.define(param.clone(), value, false);
        }

        let result = eval_block(&mut call_env, &body);
        call_env.pop_scope()?;

        // Finalize the trace (write footer) regardless of success/error.
        // If program succeeded but finalize fails, propagate the write error.
        // If program already errored, prioritize the program error.
        let program_status = if result.is_ok() { "success" } else { "error" };
        if let Some(tracer) = &env.tracer {
            let mut t = tracer.lock().unwrap();
            let fin = t.finalize(program_status);
            if result.is_ok() {
                fin.map_err(|e| anyhow::anyhow!("{}", e))?;
            }
        }

        Ok(result?.into_value())
    } else {
        bail!("main is not a function")
    }
}

/// Extract cap info from a TypeExpr: returns (is_ref, cap_type_name).
fn extract_cap_info(ty: &strata_ast::ast::TypeExpr) -> (bool, Option<String>) {
    use strata_ast::ast::TypeExpr;
    match ty {
        TypeExpr::Ref(inner, _) => {
            let (_, name) = extract_cap_info(inner);
            (true, name)
        }
        TypeExpr::Path(segments, _) if segments.len() == 1 => {
            (false, Some(segments[0].text.clone()))
        }
        _ => (false, None),
    }
}

/// Run a module in replay mode, substituting recorded trace outputs
/// instead of calling real host functions.
pub fn run_module_replay(m: &Module, trace_jsonl: &str) -> Result<Value> {
    use strata_ast::ast::Item;

    let replayer = TraceReplayer::from_jsonl(trace_jsonl).map_err(|e| anyhow::anyhow!("{}", e))?;
    let replayer = Arc::new(Mutex::new(replayer));

    // We still need a registry for ExternFnMeta (position-aware input building),
    // but we won't call any real host functions.
    let mut registry = HostRegistry::new();
    for item in &m.items {
        if let Item::ExternFn(decl) = item {
            let mut params = Vec::new();
            for param in &decl.params {
                if let Some(ty_expr) = &param.ty {
                    let (is_ref, cap_name) = extract_cap_info(ty_expr);
                    if let Some(name) = cap_name {
                        if let Some(kind) = CapKind::from_name(&name) {
                            params.push(ParamKind::Cap {
                                kind,
                                borrowed: is_ref,
                            });
                            continue;
                        }
                    }
                }
                params.push(ParamKind::Data {
                    name: param.name.text.clone(),
                });
            }
            registry.register_extern_meta(&decl.name.text, ExternFnMeta { params });
        }
    }
    let registry = Arc::new(registry);

    let mut env = Env::with_host_registry(registry).with_replayer(replayer.clone());

    // Register extern fns as host function references
    for item in &m.items {
        if let Item::ExternFn(decl) = item {
            env.define(
                decl.name.text.clone(),
                Value::HostFn(decl.name.text.clone()),
                false,
            );
        }
    }

    // Collect and register Strata function declarations
    let fn_decls: Vec<_> = m
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Fn(decl) = item {
                Some(decl)
            } else {
                None
            }
        })
        .collect();

    for decl in &fn_decls {
        env.define(decl.name.text.clone(), Value::Unit, true);
    }
    for decl in &fn_decls {
        let closure = Value::Closure {
            params: decl.params.iter().map(|p| p.name.text.clone()).collect(),
            body: decl.body.clone(),
            env: env.clone(),
        };
        env.set(&decl.name.text, closure).ok();
    }
    for decl in &fn_decls {
        let closure = Value::Closure {
            params: decl.params.iter().map(|p| p.name.text.clone()).collect(),
            body: decl.body.clone(),
            env: env.clone(),
        };
        env.set(&decl.name.text, closure).ok();
    }

    for item in &m.items {
        if let Item::Let(ld) = item {
            let cf = eval_expr(&mut env, &ld.value)?;
            let v = cf.into_value();
            env.define(ld.name.text.clone(), v, false);
        }
    }

    // Find and call main()
    let main_decl = m.items.iter().find_map(|item| {
        if let Item::Fn(decl) = item {
            if decl.name.text == "main" {
                Some(decl)
            } else {
                None
            }
        } else {
            None
        }
    });

    let main_decl = match main_decl {
        Some(d) => d,
        None => return Ok(Value::Unit),
    };

    let mut cap_args: Vec<Value> = Vec::new();
    for param in &main_decl.params {
        if let Some(ty_expr) = &param.ty {
            if let Some(name) = extract_cap_type_name(ty_expr) {
                if let Some(kind) = CapKind::from_name(&name) {
                    cap_args.push(Value::Cap(kind));
                }
            }
        }
    }

    let main_val = env
        .get("main")
        .ok_or_else(|| anyhow::anyhow!("main function not found"))?
        .clone();

    let result = if let Value::Closure {
        params,
        body,
        env: closure_env,
    } = main_val
    {
        let mut call_env = closure_env;
        call_env.push_scope();
        for (param, value) in params.iter().zip(cap_args) {
            call_env.define(param.clone(), value, false);
        }
        let result = eval_block(&mut call_env, &body)?;
        call_env.pop_scope()?;
        result.into_value()
    } else {
        bail!("main is not a function")
    };

    // Verify all trace entries were consumed
    let r = replayer.lock().unwrap();
    r.verify_complete().map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(result)
}

/// Build the inputs map for replay matching, using ExternFnMeta
/// to identify data params by position.
fn build_replay_inputs(
    env: &Env,
    name: &str,
    all_args: &[Value],
) -> std::collections::BTreeMap<String, TraceValue> {
    if let Some(registry) = &env.host_registry {
        if let Some(meta) = registry.get_extern_meta(name) {
            let mut inputs = std::collections::BTreeMap::new();
            for (i, param) in meta.params.iter().enumerate() {
                if let ParamKind::Data { name } = param {
                    if let Some(val) = all_args.get(i) {
                        inputs.insert(name.clone(), TraceValue::from_value(val));
                    }
                }
            }
            return inputs;
        }
    }
    std::collections::BTreeMap::new()
}

/// Runtime check: bail if the value is a consumed tombstone.
/// This is defense-in-depth — the static move checker should prevent this,
/// so hitting this at runtime indicates a move checker bug.
fn check_not_consumed(val: &Value, _var_name: &str, use_span: Span) -> Result<()> {
    if let Value::Consumed {
        var_name: orig_name,
        moved_at,
    } = val
    {
        bail!(
            "error[CAP-MOVE-RUNTIME]: capability '{}' has already been used\n  \
             -> used at: {}:{}\n  \
             -> previously transferred at: {}:{}\n\
             \n  \
             note: this should have been rejected at compile time. This is a Strata bug.\n  \
             Please report at: https://github.com/strata-lang/strata/issues\n  \
             Include your source file and `strata --version` output.",
            orig_name,
            use_span.start,
            use_span.end,
            moved_at.start,
            moved_at.end,
        );
    }
    Ok(())
}

/// Evaluate an expression
pub fn eval_expr(env: &mut Env, expr: &Expr) -> Result<ControlFlow> {
    match expr {
        // Literals
        Expr::Lit(Lit::Int(v), _) => Ok(ControlFlow::Value(Value::Int(*v))),
        Expr::Lit(Lit::Float(v), _) => Ok(ControlFlow::Value(Value::Float(*v))),
        Expr::Lit(Lit::Bool(b), _) => Ok(ControlFlow::Value(Value::Bool(*b))),
        Expr::Lit(Lit::Str(s), _) => Ok(ControlFlow::Value(Value::Str(s.clone()))),
        Expr::Lit(Lit::Nil, _) => Ok(ControlFlow::Value(Value::Unit)),

        // Variable lookup — affine values are destructively read (tombstoned)
        Expr::Var(id) => {
            // Peek first to check for consumed tombstone or affine value
            let is_affine = match env.get(&id.text) {
                Some(v) => {
                    check_not_consumed(v, &id.text, id.span)?;
                    v.is_affine()
                }
                None => bail!("undefined variable `{}`", id.text),
            };
            if is_affine {
                // Destructive read: take value out, leave tombstone
                let val = env.move_out(&id.text, id.span).ok_or_else(|| {
                    anyhow::anyhow!(
                        "internal error: move_out failed for `{}` — binding not found in any scope",
                        id.text
                    )
                })?;
                Ok(ControlFlow::Value(val))
            } else {
                Ok(ControlFlow::Value(env.get(&id.text).unwrap().clone()))
            }
        }

        // Parenthesized expression
        Expr::Paren { inner, .. } => eval_expr(env, inner),

        // Unary operations
        Expr::Unary { op, expr, .. } => {
            let cf = eval_expr(env, expr)?;
            if cf.is_return() {
                return Ok(cf);
            }
            let v = cf.into_value();
            match (op, v) {
                (UnOp::Not, Value::Bool(b)) => Ok(ControlFlow::Value(Value::Bool(!b))),
                (UnOp::Neg, Value::Int(i)) => Ok(ControlFlow::Value(Value::Int(-i))),
                (UnOp::Neg, Value::Float(f)) => Ok(ControlFlow::Value(Value::Float(-f))),
                (UnOp::Not, _) => bail!("`!` expects Bool"),
                (UnOp::Neg, _) => bail!("unary `-` expects Int or Float"),
            }
        }

        // Binary operations
        Expr::Binary { lhs, op, rhs, .. } => eval_binary(env, op, lhs, rhs),

        // Function call
        Expr::Call { callee, args, .. } => eval_call(env, callee, args),

        // Block expression
        Expr::Block(block) => eval_block(env, block),

        // If expression
        Expr::If {
            cond, then_, else_, ..
        } => eval_if(env, cond, then_, else_.as_deref()),

        // While loop
        Expr::While { cond, body, .. } => eval_while(env, cond, body),

        // Match expression
        Expr::Match {
            scrutinee, arms, ..
        } => eval_match(env, scrutinee, arms),

        // Tuple expression
        Expr::Tuple { elems, .. } => eval_tuple(env, elems),

        // Struct expression
        Expr::StructExpr { path, fields, .. } => eval_struct_expr(env, path, fields),

        // Path expression (enum constructor)
        Expr::PathExpr(path) => eval_path_expr(env, path),

        // Borrow expression: non-destructive read. When inner is a variable,
        // we read via env.get() (not move_out) so the capability is NOT consumed.
        // Borrows are second-class (extern fn params only, type-level enforcement).
        Expr::Borrow(inner, _) => match inner.as_ref() {
            Expr::Var(id) => match env.get(&id.text) {
                Some(v) => {
                    check_not_consumed(v, &id.text, id.span)?;
                    Ok(ControlFlow::Value(v.clone()))
                }
                None => bail!("undefined variable `{}`", id.text),
            },
            _ => eval_expr(env, inner),
        },
    }
}

/// Evaluate a binary operation
fn eval_binary(env: &mut Env, op: &BinOp, lhs: &Expr, rhs: &Expr) -> Result<ControlFlow> {
    use BinOp::*;

    // Short-circuit evaluation for logical operators
    match op {
        And => {
            let cf = eval_expr(env, lhs)?;
            if cf.is_return() {
                return Ok(cf);
            }
            match cf.into_value() {
                Value::Bool(false) => return Ok(ControlFlow::Value(Value::Bool(false))),
                Value::Bool(true) => {
                    let cf = eval_expr(env, rhs)?;
                    if cf.is_return() {
                        return Ok(cf);
                    }
                    match cf.into_value() {
                        Value::Bool(b) => return Ok(ControlFlow::Value(Value::Bool(b))),
                        _ => bail!("&& expects Bool"),
                    }
                }
                _ => bail!("&& expects Bool"),
            }
        }
        Or => {
            let cf = eval_expr(env, lhs)?;
            if cf.is_return() {
                return Ok(cf);
            }
            match cf.into_value() {
                Value::Bool(true) => return Ok(ControlFlow::Value(Value::Bool(true))),
                Value::Bool(false) => {
                    let cf = eval_expr(env, rhs)?;
                    if cf.is_return() {
                        return Ok(cf);
                    }
                    match cf.into_value() {
                        Value::Bool(b) => return Ok(ControlFlow::Value(Value::Bool(b))),
                        _ => bail!("|| expects Bool"),
                    }
                }
                _ => bail!("|| expects Bool"),
            }
        }
        _ => {}
    }

    // Evaluate both operands
    let cf_l = eval_expr(env, lhs)?;
    if cf_l.is_return() {
        return Ok(cf_l);
    }
    let l = cf_l.into_value();

    let cf_r = eval_expr(env, rhs)?;
    if cf_r.is_return() {
        return Ok(cf_r);
    }
    let r = cf_r.into_value();

    match op {
        Add | Sub | Mul | Div => {
            let result = match (l, r, op) {
                (Value::Int(a), Value::Int(b), Add) => Value::Int(a + b),
                (Value::Int(a), Value::Int(b), Sub) => Value::Int(a - b),
                (Value::Int(a), Value::Int(b), Mul) => Value::Int(a * b),
                (Value::Int(a), Value::Int(b), Div) => Value::Int(a / b),

                (Value::Int(a), Value::Float(b), Add) => Value::Float((a as f64) + b),
                (Value::Int(a), Value::Float(b), Sub) => Value::Float((a as f64) - b),
                (Value::Int(a), Value::Float(b), Mul) => Value::Float((a as f64) * b),
                (Value::Int(a), Value::Float(b), Div) => Value::Float((a as f64) / b),

                (Value::Float(a), Value::Int(b), Add) => Value::Float(a + (b as f64)),
                (Value::Float(a), Value::Int(b), Sub) => Value::Float(a - (b as f64)),
                (Value::Float(a), Value::Int(b), Mul) => Value::Float(a * (b as f64)),
                (Value::Float(a), Value::Int(b), Div) => Value::Float(a / (b as f64)),

                (Value::Float(a), Value::Float(b), Add) => Value::Float(a + b),
                (Value::Float(a), Value::Float(b), Sub) => Value::Float(a - b),
                (Value::Float(a), Value::Float(b), Mul) => Value::Float(a * b),
                (Value::Float(a), Value::Float(b), Div) => Value::Float(a / b),

                _ => bail!("arithmetic expects Int/Float"),
            };
            Ok(ControlFlow::Value(result))
        }

        Lt | Le | Gt | Ge => {
            let result = match (l, r, op) {
                (Value::Int(a), Value::Int(b), Lt) => a < b,
                (Value::Int(a), Value::Int(b), Le) => a <= b,
                (Value::Int(a), Value::Int(b), Gt) => a > b,
                (Value::Int(a), Value::Int(b), Ge) => a >= b,

                (Value::Float(a), Value::Float(b), Lt) => a < b,
                (Value::Float(a), Value::Float(b), Le) => a <= b,
                (Value::Float(a), Value::Float(b), Gt) => a > b,
                (Value::Float(a), Value::Float(b), Ge) => a >= b,

                (Value::Int(a), Value::Float(b), Lt) => (a as f64) < b,
                (Value::Int(a), Value::Float(b), Le) => (a as f64) <= b,
                (Value::Int(a), Value::Float(b), Gt) => (a as f64) > b,
                (Value::Int(a), Value::Float(b), Ge) => (a as f64) >= b,

                (Value::Float(a), Value::Int(b), Lt) => a < (b as f64),
                (Value::Float(a), Value::Int(b), Le) => a <= (b as f64),
                (Value::Float(a), Value::Int(b), Gt) => a > (b as f64),
                (Value::Float(a), Value::Int(b), Ge) => a >= (b as f64),

                _ => bail!("relational ops expect numbers"),
            };
            Ok(ControlFlow::Value(Value::Bool(result)))
        }

        Eq | Ne => {
            let eq = match (l, r) {
                (Value::Int(a), Value::Int(b)) => a == b,
                (Value::Float(a), Value::Float(b)) => a == b,
                (Value::Int(a), Value::Float(b)) => (a as f64) == b,
                (Value::Float(a), Value::Int(b)) => a == (b as f64),
                (Value::Bool(a), Value::Bool(b)) => a == b,
                (Value::Str(a), Value::Str(b)) => a == b,
                (Value::Unit, Value::Unit) => true,
                _ => false,
            };
            Ok(ControlFlow::Value(Value::Bool(if matches!(op, Eq) {
                eq
            } else {
                !eq
            })))
        }

        And | Or => unreachable!("handled above"),
    }
}

/// Evaluate a block expression
pub fn eval_block(env: &mut Env, block: &Block) -> Result<ControlFlow> {
    env.with_scope(|env| {
        // Evaluate each statement
        for stmt in &block.stmts {
            let cf = eval_stmt(env, stmt)?;
            // Propagate returns early
            if cf.is_return() {
                return Ok(cf);
            }
        }

        // Evaluate tail expression if present
        if let Some(ref tail) = block.tail {
            eval_expr(env, tail)
        } else {
            Ok(ControlFlow::Value(Value::Unit))
        }
    })
}

/// Evaluate a statement
fn eval_stmt(env: &mut Env, stmt: &Stmt) -> Result<ControlFlow> {
    match stmt {
        Stmt::Let {
            mutable,
            pat,
            value,
            ..
        } => {
            let cf = eval_expr(env, value)?;
            if cf.is_return() {
                return Ok(cf);
            }
            let v = cf.into_value();

            // Match pattern against value to get bindings
            // Pattern should always match (irrefutability checked by type checker)
            let bindings = match_pattern(pat, &v).ok_or_else(|| {
                anyhow::anyhow!("pattern match failed (should be caught by type checker)")
            })?;

            // Check for duplicate bindings (defensive - type checker should catch this)
            check_duplicate_bindings(&bindings)?;

            // Define all bindings with the same mutability
            for (name, val) in bindings {
                env.define(name, val, *mutable);
            }

            Ok(ControlFlow::Value(Value::Unit))
        }

        Stmt::Assign { target, value, .. } => {
            let cf = eval_expr(env, value)?;
            if cf.is_return() {
                return Ok(cf);
            }
            let v = cf.into_value();
            env.set(&target.text, v)?;
            Ok(ControlFlow::Value(Value::Unit))
        }

        Stmt::Expr { expr, .. } => {
            let cf = eval_expr(env, expr)?;
            // Propagate returns, but discard normal values
            if cf.is_return() {
                Ok(cf)
            } else {
                Ok(ControlFlow::Value(Value::Unit))
            }
        }

        Stmt::Return { value, .. } => {
            let v = if let Some(val_expr) = value {
                let cf = eval_expr(env, val_expr)?;
                if cf.is_return() {
                    return Ok(cf);
                }
                cf.into_value()
            } else {
                Value::Unit
            };
            Ok(ControlFlow::Return(v))
        }
    }
}

/// Evaluate an if expression
fn eval_if(env: &mut Env, cond: &Expr, then_: &Block, else_: Option<&Expr>) -> Result<ControlFlow> {
    // Evaluate condition
    let cf = eval_expr(env, cond)?;
    if cf.is_return() {
        return Ok(cf);
    }

    let cond_val = match cf.into_value() {
        Value::Bool(b) => b,
        _ => bail!("if condition must be Bool"),
    };

    if cond_val {
        eval_block(env, then_)
    } else if let Some(else_expr) = else_ {
        eval_expr(env, else_expr)
    } else {
        Ok(ControlFlow::Value(Value::Unit))
    }
}

/// Evaluate a while loop
fn eval_while(env: &mut Env, cond: &Expr, body: &Block) -> Result<ControlFlow> {
    loop {
        // Evaluate condition
        let cf = eval_expr(env, cond)?;
        if cf.is_return() {
            return Ok(cf);
        }

        let cond_val = match cf.into_value() {
            Value::Bool(b) => b,
            _ => bail!("while condition must be Bool"),
        };

        if !cond_val {
            break;
        }

        // Evaluate body
        let cf = eval_block(env, body)?;

        // Propagate returns
        if cf.is_return() {
            return Ok(cf);
        }

        // Handle break/continue (reserved for future)
        match cf {
            ControlFlow::Break => break,
            ControlFlow::Continue => continue,
            _ => {}
        }
    }

    Ok(ControlFlow::Value(Value::Unit))
}

/// Evaluate a function call
fn eval_call(env: &mut Env, callee: &Expr, args: &[Expr]) -> Result<ControlFlow> {
    // Security: Check call depth limit
    let depth = CALL_DEPTH.with(|d| {
        let current = d.get();
        d.set(current + 1);
        current + 1
    });

    if depth > MAX_CALL_DEPTH {
        CALL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        bail!(
            "maximum call depth exceeded (limit: {} calls)",
            MAX_CALL_DEPTH
        );
    }

    // Ensure we decrement depth even on error/return
    let result = eval_call_inner(env, callee, args);

    CALL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));

    result
}

/// Inner implementation of eval_call (without depth tracking)
fn eval_call_inner(env: &mut Env, callee: &Expr, args: &[Expr]) -> Result<ControlFlow> {
    // Evaluate callee
    let cf = eval_expr(env, callee)?;
    if cf.is_return() {
        return Ok(cf);
    }

    let callee_val = cf.into_value();

    // Handle variant constructor calls: Option::Some(42)
    if let Value::Variant {
        enum_name,
        variant_name,
        fields: existing_fields,
    } = &callee_val
    {
        if existing_fields.is_empty() {
            // This is a unit variant being called as a constructor
            let mut field_values = Vec::new();
            for arg in args {
                let cf = eval_expr(env, arg)?;
                if cf.is_return() {
                    return Ok(cf);
                }
                field_values.push(cf.into_value());
            }
            return Ok(ControlFlow::Value(Value::Variant {
                enum_name: enum_name.clone(),
                variant_name: variant_name.clone(),
                fields: field_values,
            }));
        }
    }

    // Handle host function dispatch for extern fns
    if let Value::HostFn(name) = &callee_val {
        let mut arg_values = Vec::new();
        for arg in args {
            let cf = eval_expr(env, arg)?;
            if cf.is_return() {
                return Ok(cf);
            }
            arg_values.push(cf.into_value());
        }

        // Replay mode: substitute outputs from recorded trace
        if let Some(replayer) = &env.replayer {
            let inputs = build_replay_inputs(env, name, &arg_values);
            let mut r = replayer.lock().unwrap();
            match r.next(name, &inputs) {
                Ok(val) => return Ok(ControlFlow::Value(val)),
                Err(ReplayError::ReplayedError(msg)) => {
                    bail!("host function '{}': {}", name, msg)
                }
                Err(e) => bail!("{}", e),
            }
        }

        // Live mode: dispatch to real host function
        let registry = env.host_registry.as_ref().ok_or_else(|| {
            anyhow::anyhow!("no host registry available for extern fn '{}'", name)
        })?;

        // Single dispatch path: always use position-aware dispatch_traced().
        // TraceEmitter::disabled() handles the no-output case.
        let result = if let Some(tracer) = &env.tracer {
            let mut t = tracer.lock().unwrap();
            registry.dispatch_traced(name, &arg_values, &mut t)
        } else {
            let mut t = TraceEmitter::disabled();
            registry.dispatch_traced(name, &arg_values, &mut t)
        };

        match result {
            Ok(val) => return Ok(ControlFlow::Value(val)),
            Err(e) => bail!("host function '{}': {}", name, e),
        }
    }

    let closure = match callee_val {
        Value::Closure { params, body, env } => (params, body, env),
        v => bail!("cannot call non-function value: {}", v),
    };

    let (params, body, mut closure_env) = closure;

    // For recursion and mutual recursion support: patch the closure's captured
    // environment with any closures from the calling environment that are
    // placeholders (Unit) or outdated versions in the captured env.
    // This handles self-recursion, forward references, and mutual recursion.
    if let Some(calling_scope) = env.scopes.first() {
        if let Some(closure_scope) = closure_env.scopes.first_mut() {
            for (name, binding) in calling_scope {
                // Only patch if it's a closure in the calling env
                if matches!(binding.value, Value::Closure { .. }) {
                    // Check if closure_env has Unit (placeholder) or a different closure
                    let needs_update = match closure_scope.get(name) {
                        Some(b) => matches!(b.value, Value::Unit),
                        None => true,
                    };
                    if needs_update {
                        closure_scope.insert(
                            name.clone(),
                            Binding {
                                value: binding.value.clone(),
                                mutable: false,
                            },
                        );
                    }
                }
            }
        }
    }

    // Check argument count
    if args.len() != params.len() {
        bail!(
            "function expects {} arguments, got {}",
            params.len(),
            args.len()
        );
    }

    // Evaluate arguments
    let mut arg_values = Vec::new();
    for arg in args {
        let cf = eval_expr(env, arg)?;
        if cf.is_return() {
            return Ok(cf);
        }
        arg_values.push(cf.into_value());
    }

    // Set up function environment with captured env
    closure_env.push_scope();

    // Bind parameters to arguments
    for (param, value) in params.iter().zip(arg_values) {
        closure_env.define(param.clone(), value, false);
    }

    // Evaluate body
    let result = eval_block(&mut closure_env, &body)?;

    closure_env.pop_scope()?;

    // Unwrap Return at function boundary
    Ok(ControlFlow::Value(result.into_value()))
}

/// Evaluate a tuple expression
fn eval_tuple(env: &mut Env, elems: &[Expr]) -> Result<ControlFlow> {
    // Empty tuple is unit
    if elems.is_empty() {
        return Ok(ControlFlow::Value(Value::Unit));
    }

    // Single element is unwrapped (not a tuple)
    if elems.len() == 1 {
        return eval_expr(env, &elems[0]);
    }

    // Evaluate each element
    let mut values = Vec::new();
    for elem in elems {
        let cf = eval_expr(env, elem)?;
        if cf.is_return() {
            return Ok(cf);
        }
        values.push(cf.into_value());
    }

    Ok(ControlFlow::Value(Value::Tuple(values)))
}

/// Evaluate a struct expression
fn eval_struct_expr(env: &mut Env, path: &Path, fields: &[FieldInit]) -> Result<ControlFlow> {
    let struct_name = path.as_str();

    let mut field_values = HashMap::new();
    for field in fields {
        let cf = eval_expr(env, &field.value)?;
        if cf.is_return() {
            return Ok(cf);
        }
        field_values.insert(field.name.text.clone(), cf.into_value());
    }

    Ok(ControlFlow::Value(Value::Struct {
        name: struct_name,
        fields: field_values,
    }))
}

/// Evaluate a path expression (enum constructor)
fn eval_path_expr(env: &mut Env, path: &Path) -> Result<ControlFlow> {
    let segments = &path.segments;

    if segments.len() == 2 {
        // Enum::Variant format - unit constructor
        let enum_name = segments[0].text.clone();
        let variant_name = segments[1].text.clone();
        return Ok(ControlFlow::Value(Value::Variant {
            enum_name,
            variant_name,
            fields: vec![],
        }));
    }

    // Single segment - look up in environment (might be a function or variable)
    // Same affine treatment as Expr::Var
    if segments.len() == 1 {
        let seg = &segments[0];
        let is_affine = match env.get(&seg.text) {
            Some(v) => {
                check_not_consumed(v, &seg.text, seg.span)?;
                v.is_affine()
            }
            None => bail!("undefined: {}", seg.text),
        };
        if is_affine {
            let val = env.move_out(&seg.text, seg.span).ok_or_else(|| {
                anyhow::anyhow!(
                    "internal error: move_out failed for `{}` — binding not found in any scope",
                    seg.text
                )
            })?;
            return Ok(ControlFlow::Value(val));
        } else {
            return Ok(ControlFlow::Value(env.get(&seg.text).unwrap().clone()));
        }
    }

    bail!("invalid path expression: {}", path.as_str())
}

/// Evaluate a match expression
fn eval_match(env: &mut Env, scrutinee: &Expr, arms: &[MatchArm]) -> Result<ControlFlow> {
    // Evaluate the scrutinee
    let cf = eval_expr(env, scrutinee)?;
    if cf.is_return() {
        return Ok(cf);
    }
    let value = cf.into_value();

    // Try each arm in order
    for arm in arms {
        if let Some(bindings) = match_pattern(&arm.pat, &value) {
            // Check for duplicate bindings (defensive - type checker should catch this)
            check_duplicate_bindings(&bindings)?;

            // Pattern matched - evaluate arm body with bindings in new scope
            return env.with_scope(|env| {
                for (name, val) in bindings {
                    env.define(name, val, false);
                }
                eval_expr(env, &arm.body)
            });
        }
    }

    // No arm matched (should be caught by exhaustiveness checking)
    bail!("non-exhaustive match: no pattern matched value {}", value)
}

/// Try to match a pattern against a value, returning bindings if successful
fn match_pattern(pat: &Pat, value: &Value) -> Option<Vec<(String, Value)>> {
    match pat {
        Pat::Wildcard(_) => Some(vec![]),

        Pat::Ident(ident) => Some(vec![(ident.text.clone(), value.clone())]),

        Pat::Literal(lit, _) => match (lit, value) {
            (Lit::Int(n), Value::Int(v)) if *n == *v => Some(vec![]),
            (Lit::Float(n), Value::Float(v)) if *n == *v => Some(vec![]),
            (Lit::Bool(b), Value::Bool(v)) if *b == *v => Some(vec![]),
            (Lit::Str(s), Value::Str(v)) if s == v => Some(vec![]),
            (Lit::Nil, Value::Unit) => Some(vec![]),
            _ => None,
        },

        Pat::Tuple(pats, _) => {
            // Special case: empty tuple pattern () matches Unit
            if pats.is_empty() {
                return if matches!(value, Value::Unit) {
                    Some(vec![])
                } else {
                    None
                };
            }
            if let Value::Tuple(values) = value {
                if pats.len() != values.len() {
                    return None;
                }
                let mut bindings = Vec::new();
                for (pat, val) in pats.iter().zip(values.iter()) {
                    if let Some(mut sub_bindings) = match_pattern(pat, val) {
                        bindings.append(&mut sub_bindings);
                    } else {
                        return None;
                    }
                }
                Some(bindings)
            } else {
                None
            }
        }

        Pat::Struct { path, fields, .. } => {
            if let Value::Struct {
                name,
                fields: value_fields,
            } = value
            {
                if path.as_str() != *name {
                    return None;
                }
                let mut bindings = Vec::new();
                for pat_field in fields {
                    let field_value = value_fields.get(&pat_field.name.text)?;
                    if let Some(mut sub_bindings) = match_pattern(&pat_field.pat, field_value) {
                        bindings.append(&mut sub_bindings);
                    } else {
                        return None;
                    }
                }
                Some(bindings)
            } else {
                None
            }
        }

        Pat::Variant { path, fields, .. } => {
            if let Value::Variant {
                enum_name,
                variant_name,
                fields: value_fields,
            } = value
            {
                // Check if the pattern path matches the variant
                let pattern_path = path.as_str();
                let value_path = format!("{}::{}", enum_name, variant_name);
                if pattern_path != value_path {
                    return None;
                }

                // Check field count
                if fields.len() != value_fields.len() {
                    return None;
                }

                // Match each field pattern
                let mut bindings = Vec::new();
                for (pat, val) in fields.iter().zip(value_fields.iter()) {
                    if let Some(mut sub_bindings) = match_pattern(pat, val) {
                        bindings.append(&mut sub_bindings);
                    } else {
                        return None;
                    }
                }
                Some(bindings)
            } else {
                None
            }
        }
    }
}

/// Check for duplicate bindings and return an error if found.
/// This is a defensive check - the type checker should catch duplicates.
fn check_duplicate_bindings(bindings: &[(String, Value)]) -> Result<()> {
    let mut seen = HashSet::new();
    for (name, _) in bindings {
        if !seen.insert(name) {
            bail!(
                "duplicate binding '{}' in pattern (should be caught by type checker)",
                name
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_ast::ast::Ident;
    use strata_ast::span::Span;

    fn sp() -> Span {
        Span { start: 0, end: 0 }
    }

    fn ident(name: &str) -> Ident {
        Ident {
            text: name.to_string(),
            span: sp(),
        }
    }

    #[test]
    fn test_eval_literal_int() {
        let mut env = Env::new();
        let expr = Expr::Lit(Lit::Int(42), sp());
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(42))));
    }

    #[test]
    fn test_eval_literal_bool() {
        let mut env = Env::new();
        let expr = Expr::Lit(Lit::Bool(true), sp());
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Bool(true))));
    }

    #[test]
    fn test_eval_block_tail() {
        // { let x = 1; x + 1 } evaluates to 2
        let mut env = Env::new();
        let block = Block {
            stmts: vec![Stmt::Let {
                mutable: false,
                pat: Pat::Ident(ident("x")),
                ty: None,
                value: Expr::Lit(Lit::Int(1), sp()),
                span: sp(),
            }],
            tail: Some(Box::new(Expr::Binary {
                lhs: Box::new(Expr::Var(ident("x"))),
                op: BinOp::Add,
                rhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
                span: sp(),
            })),
            span: sp(),
        };
        let cf = eval_block(&mut env, &block).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(2))));
    }

    #[test]
    fn test_eval_block_no_tail() {
        // { let x = 1; } evaluates to Unit
        let mut env = Env::new();
        let block = Block {
            stmts: vec![Stmt::Let {
                mutable: false,
                pat: Pat::Ident(ident("x")),
                ty: None,
                value: Expr::Lit(Lit::Int(1), sp()),
                span: sp(),
            }],
            tail: None,
            span: sp(),
        };
        let cf = eval_block(&mut env, &block).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Unit)));
    }

    #[test]
    fn test_eval_if_true() {
        // if true { 1 } else { 2 } evaluates to 1
        let mut env = Env::new();
        let expr = Expr::If {
            cond: Box::new(Expr::Lit(Lit::Bool(true), sp())),
            then_: Block {
                stmts: vec![],
                tail: Some(Box::new(Expr::Lit(Lit::Int(1), sp()))),
                span: sp(),
            },
            else_: Some(Box::new(Expr::Block(Block {
                stmts: vec![],
                tail: Some(Box::new(Expr::Lit(Lit::Int(2), sp()))),
                span: sp(),
            }))),
            span: sp(),
        };
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(1))));
    }

    #[test]
    fn test_eval_if_false() {
        // if false { 1 } else { 2 } evaluates to 2
        let mut env = Env::new();
        let expr = Expr::If {
            cond: Box::new(Expr::Lit(Lit::Bool(false), sp())),
            then_: Block {
                stmts: vec![],
                tail: Some(Box::new(Expr::Lit(Lit::Int(1), sp()))),
                span: sp(),
            },
            else_: Some(Box::new(Expr::Block(Block {
                stmts: vec![],
                tail: Some(Box::new(Expr::Lit(Lit::Int(2), sp()))),
                span: sp(),
            }))),
            span: sp(),
        };
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(2))));
    }

    #[test]
    fn test_eval_mutable_assign() {
        // let mut x = 1; x = 2; x evaluates to 2
        let mut env = Env::new();
        let block = Block {
            stmts: vec![
                Stmt::Let {
                    mutable: true,
                    pat: Pat::Ident(ident("x")),
                    ty: None,
                    value: Expr::Lit(Lit::Int(1), sp()),
                    span: sp(),
                },
                Stmt::Assign {
                    target: ident("x"),
                    value: Expr::Lit(Lit::Int(2), sp()),
                    span: sp(),
                },
            ],
            tail: Some(Box::new(Expr::Var(ident("x")))),
            span: sp(),
        };
        let cf = eval_block(&mut env, &block).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(2))));
    }

    #[test]
    fn test_eval_immutable_assign_error() {
        // let x = 1; x = 2; should fail
        let mut env = Env::new();
        let block = Block {
            stmts: vec![
                Stmt::Let {
                    mutable: false,
                    pat: Pat::Ident(ident("x")),
                    ty: None,
                    value: Expr::Lit(Lit::Int(1), sp()),
                    span: sp(),
                },
                Stmt::Assign {
                    target: ident("x"),
                    value: Expr::Lit(Lit::Int(2), sp()),
                    span: sp(),
                },
            ],
            tail: None,
            span: sp(),
        };
        let result = eval_block(&mut env, &block);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_nested_scopes() {
        // { let x = 1; { let x = 2; x }; x } - inner returns 2, outer returns 1
        let mut env = Env::new();
        let block = Block {
            stmts: vec![
                Stmt::Let {
                    mutable: false,
                    pat: Pat::Ident(ident("x")),
                    ty: None,
                    value: Expr::Lit(Lit::Int(1), sp()),
                    span: sp(),
                },
                Stmt::Expr {
                    expr: Expr::Block(Block {
                        stmts: vec![Stmt::Let {
                            mutable: false,
                            pat: Pat::Ident(ident("x")),
                            ty: None,
                            value: Expr::Lit(Lit::Int(2), sp()),
                            span: sp(),
                        }],
                        tail: Some(Box::new(Expr::Var(ident("x")))),
                        span: sp(),
                    }),
                    span: sp(),
                },
            ],
            tail: Some(Box::new(Expr::Var(ident("x")))),
            span: sp(),
        };
        let cf = eval_block(&mut env, &block).unwrap();
        // Outer x is still 1
        assert!(matches!(cf, ControlFlow::Value(Value::Int(1))));
    }

    #[test]
    fn test_eval_while_sum() {
        // let mut sum = 0; let mut i = 0; while i < 5 { sum = sum + i; i = i + 1; }; sum
        // Sum of 0..5 = 0+1+2+3+4 = 10
        let mut env = Env::new();
        let block = Block {
            stmts: vec![
                Stmt::Let {
                    mutable: true,
                    pat: Pat::Ident(ident("sum")),
                    ty: None,
                    value: Expr::Lit(Lit::Int(0), sp()),
                    span: sp(),
                },
                Stmt::Let {
                    mutable: true,
                    pat: Pat::Ident(ident("i")),
                    ty: None,
                    value: Expr::Lit(Lit::Int(0), sp()),
                    span: sp(),
                },
                Stmt::Expr {
                    expr: Expr::While {
                        cond: Box::new(Expr::Binary {
                            lhs: Box::new(Expr::Var(ident("i"))),
                            op: BinOp::Lt,
                            rhs: Box::new(Expr::Lit(Lit::Int(5), sp())),
                            span: sp(),
                        }),
                        body: Block {
                            stmts: vec![
                                Stmt::Assign {
                                    target: ident("sum"),
                                    value: Expr::Binary {
                                        lhs: Box::new(Expr::Var(ident("sum"))),
                                        op: BinOp::Add,
                                        rhs: Box::new(Expr::Var(ident("i"))),
                                        span: sp(),
                                    },
                                    span: sp(),
                                },
                                Stmt::Assign {
                                    target: ident("i"),
                                    value: Expr::Binary {
                                        lhs: Box::new(Expr::Var(ident("i"))),
                                        op: BinOp::Add,
                                        rhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
                                        span: sp(),
                                    },
                                    span: sp(),
                                },
                            ],
                            tail: None,
                            span: sp(),
                        },
                        span: sp(),
                    },
                    span: sp(),
                },
            ],
            tail: Some(Box::new(Expr::Var(ident("sum")))),
            span: sp(),
        };
        let cf = eval_block(&mut env, &block).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(10))));
    }

    #[test]
    fn test_eval_return_early() {
        // { return 42; 100 } should return 42, not evaluate 100
        let mut env = Env::new();
        let block = Block {
            stmts: vec![Stmt::Return {
                value: Some(Expr::Lit(Lit::Int(42), sp())),
                span: sp(),
            }],
            tail: Some(Box::new(Expr::Lit(Lit::Int(100), sp()))),
            span: sp(),
        };
        let cf = eval_block(&mut env, &block).unwrap();
        assert!(matches!(cf, ControlFlow::Return(Value::Int(42))));
    }

    #[test]
    fn test_eval_function_call() {
        // Define fn add(x, y) { x + y } and call add(1, 2)
        let mut env = Env::new();

        // Create closure
        let add_closure = Value::Closure {
            params: vec!["x".to_string(), "y".to_string()],
            body: Block {
                stmts: vec![],
                tail: Some(Box::new(Expr::Binary {
                    lhs: Box::new(Expr::Var(ident("x"))),
                    op: BinOp::Add,
                    rhs: Box::new(Expr::Var(ident("y"))),
                    span: sp(),
                })),
                span: sp(),
            },
            env: Env::new(),
        };
        env.define("add".to_string(), add_closure, false);

        // Call add(1, 2)
        let call_expr = Expr::Call {
            callee: Box::new(Expr::Var(ident("add"))),
            args: vec![Expr::Lit(Lit::Int(1), sp()), Expr::Lit(Lit::Int(2), sp())],
            span: sp(),
        };
        let cf = eval_expr(&mut env, &call_expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(3))));
    }

    #[test]
    fn test_eval_recursive_function() {
        // Factorial: fn fact(n) { if n <= 1 { 1 } else { n * fact(n - 1) } }
        // To enable recursion, we need:
        // 1. Define function name as placeholder
        // 2. Create closure that captures env with placeholder
        // 3. Update env with closure
        // 4. Re-create closure that captures env with actual closure
        let mut env = Env::new();

        // Step 1: Define placeholder
        env.define("fact".to_string(), Value::Unit, true);

        let fact_body = Block {
            stmts: vec![],
            tail: Some(Box::new(Expr::If {
                cond: Box::new(Expr::Binary {
                    lhs: Box::new(Expr::Var(ident("n"))),
                    op: BinOp::Le,
                    rhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
                    span: sp(),
                }),
                then_: Block {
                    stmts: vec![],
                    tail: Some(Box::new(Expr::Lit(Lit::Int(1), sp()))),
                    span: sp(),
                },
                else_: Some(Box::new(Expr::Block(Block {
                    stmts: vec![],
                    tail: Some(Box::new(Expr::Binary {
                        lhs: Box::new(Expr::Var(ident("n"))),
                        op: BinOp::Mul,
                        rhs: Box::new(Expr::Call {
                            callee: Box::new(Expr::Var(ident("fact"))),
                            args: vec![Expr::Binary {
                                lhs: Box::new(Expr::Var(ident("n"))),
                                op: BinOp::Sub,
                                rhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
                                span: sp(),
                            }],
                            span: sp(),
                        }),
                        span: sp(),
                    })),
                    span: sp(),
                }))),
                span: sp(),
            })),
            span: sp(),
        };

        // Step 2 & 3: Create closure and update env
        let fact_closure = Value::Closure {
            params: vec!["n".to_string()],
            body: fact_body.clone(),
            env: env.clone(),
        };
        env.set("fact", fact_closure).unwrap();

        // Step 4: Re-create closure with updated env (now contains actual closure)
        let fact_closure = Value::Closure {
            params: vec!["n".to_string()],
            body: fact_body,
            env: env.clone(),
        };
        env.set("fact", fact_closure).unwrap();

        // Call fact(5) = 120
        let call_expr = Expr::Call {
            callee: Box::new(Expr::Var(ident("fact"))),
            args: vec![Expr::Lit(Lit::Int(5), sp())],
            span: sp(),
        };
        let cf = eval_expr(&mut env, &call_expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(120))));
    }

    // =========================================================================
    // Phase 6: Tuple, Struct, Variant, Match tests
    // =========================================================================

    #[test]
    fn test_eval_tuple() {
        // (1, 2, 3) evaluates to a tuple
        let mut env = Env::new();
        let expr = Expr::Tuple {
            elems: vec![
                Expr::Lit(Lit::Int(1), sp()),
                Expr::Lit(Lit::Int(2), sp()),
                Expr::Lit(Lit::Int(3), sp()),
            ],
            span: sp(),
        };
        let cf = eval_expr(&mut env, &expr).unwrap();
        if let ControlFlow::Value(Value::Tuple(elems)) = cf {
            assert_eq!(elems.len(), 3);
            assert!(matches!(elems[0], Value::Int(1)));
            assert!(matches!(elems[1], Value::Int(2)));
            assert!(matches!(elems[2], Value::Int(3)));
        } else {
            panic!("expected Tuple value");
        }
    }

    #[test]
    fn test_eval_empty_tuple() {
        // () evaluates to Unit
        let mut env = Env::new();
        let expr = Expr::Tuple {
            elems: vec![],
            span: sp(),
        };
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Unit)));
    }

    #[test]
    fn test_eval_single_elem_tuple() {
        // (1) evaluates to Int (not a tuple)
        let mut env = Env::new();
        let expr = Expr::Tuple {
            elems: vec![Expr::Lit(Lit::Int(42), sp())],
            span: sp(),
        };
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(42))));
    }

    #[test]
    fn test_eval_match_literal() {
        // match 1 { 1 => true, _ => false }
        use strata_ast::ast::MatchArm;
        let mut env = Env::new();
        let expr = Expr::Match {
            scrutinee: Box::new(Expr::Lit(Lit::Int(1), sp())),
            arms: vec![
                MatchArm {
                    pat: Pat::Literal(Lit::Int(1), sp()),
                    body: Expr::Lit(Lit::Bool(true), sp()),
                    span: sp(),
                },
                MatchArm {
                    pat: Pat::Wildcard(sp()),
                    body: Expr::Lit(Lit::Bool(false), sp()),
                    span: sp(),
                },
            ],
            span: sp(),
        };
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Bool(true))));
    }

    #[test]
    fn test_eval_match_wildcard() {
        // match 99 { 1 => false, _ => true }
        use strata_ast::ast::MatchArm;
        let mut env = Env::new();
        let expr = Expr::Match {
            scrutinee: Box::new(Expr::Lit(Lit::Int(99), sp())),
            arms: vec![
                MatchArm {
                    pat: Pat::Literal(Lit::Int(1), sp()),
                    body: Expr::Lit(Lit::Bool(false), sp()),
                    span: sp(),
                },
                MatchArm {
                    pat: Pat::Wildcard(sp()),
                    body: Expr::Lit(Lit::Bool(true), sp()),
                    span: sp(),
                },
            ],
            span: sp(),
        };
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Bool(true))));
    }

    #[test]
    fn test_eval_match_binding() {
        // match 42 { x => x + 1 }
        use strata_ast::ast::MatchArm;
        let mut env = Env::new();
        let expr = Expr::Match {
            scrutinee: Box::new(Expr::Lit(Lit::Int(42), sp())),
            arms: vec![MatchArm {
                pat: Pat::Ident(ident("x")),
                body: Expr::Binary {
                    lhs: Box::new(Expr::Var(ident("x"))),
                    op: BinOp::Add,
                    rhs: Box::new(Expr::Lit(Lit::Int(1), sp())),
                    span: sp(),
                },
                span: sp(),
            }],
            span: sp(),
        };
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(43))));
    }

    #[test]
    fn test_eval_match_tuple() {
        // match (1, 2) { (a, b) => a + b }
        use strata_ast::ast::MatchArm;
        let mut env = Env::new();
        let expr = Expr::Match {
            scrutinee: Box::new(Expr::Tuple {
                elems: vec![Expr::Lit(Lit::Int(1), sp()), Expr::Lit(Lit::Int(2), sp())],
                span: sp(),
            }),
            arms: vec![MatchArm {
                pat: Pat::Tuple(vec![Pat::Ident(ident("a")), Pat::Ident(ident("b"))], sp()),
                body: Expr::Binary {
                    lhs: Box::new(Expr::Var(ident("a"))),
                    op: BinOp::Add,
                    rhs: Box::new(Expr::Var(ident("b"))),
                    span: sp(),
                },
                span: sp(),
            }],
            span: sp(),
        };
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(3))));
    }

    #[test]
    fn test_eval_variant_construction() {
        // Option::Some(42)
        use strata_ast::ast::Path;
        let mut env = Env::new();

        // First construct the path expression for Option::Some
        let path_expr = Expr::PathExpr(Path {
            segments: vec![ident("Option"), ident("Some")],
            span: sp(),
        });

        // Call it with argument 42
        let expr = Expr::Call {
            callee: Box::new(path_expr),
            args: vec![Expr::Lit(Lit::Int(42), sp())],
            span: sp(),
        };

        let cf = eval_expr(&mut env, &expr).unwrap();
        if let ControlFlow::Value(Value::Variant {
            enum_name,
            variant_name,
            fields,
        }) = cf
        {
            assert_eq!(enum_name, "Option");
            assert_eq!(variant_name, "Some");
            assert_eq!(fields.len(), 1);
            assert!(matches!(fields[0], Value::Int(42)));
        } else {
            panic!("expected Variant value");
        }
    }

    #[test]
    fn test_eval_unit_variant() {
        // Option::None
        use strata_ast::ast::Path;
        let mut env = Env::new();
        let expr = Expr::PathExpr(Path {
            segments: vec![ident("Option"), ident("None")],
            span: sp(),
        });

        let cf = eval_expr(&mut env, &expr).unwrap();
        if let ControlFlow::Value(Value::Variant {
            enum_name,
            variant_name,
            fields,
        }) = cf
        {
            assert_eq!(enum_name, "Option");
            assert_eq!(variant_name, "None");
            assert!(fields.is_empty());
        } else {
            panic!("expected Variant value");
        }
    }

    #[test]
    fn test_eval_match_variant() {
        // match Option::Some(42) { Option::Some(x) => x, Option::None => 0 }
        use strata_ast::ast::{MatchArm, Path};
        let mut env = Env::new();

        // Build Option::Some(42)
        let scrutinee = Expr::Call {
            callee: Box::new(Expr::PathExpr(Path {
                segments: vec![ident("Option"), ident("Some")],
                span: sp(),
            })),
            args: vec![Expr::Lit(Lit::Int(42), sp())],
            span: sp(),
        };

        let expr = Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![
                MatchArm {
                    pat: Pat::Variant {
                        path: Path {
                            segments: vec![ident("Option"), ident("Some")],
                            span: sp(),
                        },
                        fields: vec![Pat::Ident(ident("x"))],
                        span: sp(),
                    },
                    body: Expr::Var(ident("x")),
                    span: sp(),
                },
                MatchArm {
                    pat: Pat::Variant {
                        path: Path {
                            segments: vec![ident("Option"), ident("None")],
                            span: sp(),
                        },
                        fields: vec![],
                        span: sp(),
                    },
                    body: Expr::Lit(Lit::Int(0), sp()),
                    span: sp(),
                },
            ],
            span: sp(),
        };

        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(42))));
    }

    #[test]
    fn test_eval_struct_construction() {
        // Point { x: 10, y: 20 }
        use strata_ast::ast::{FieldInit, Path};
        let mut env = Env::new();

        let expr = Expr::StructExpr {
            path: Path {
                segments: vec![ident("Point")],
                span: sp(),
            },
            fields: vec![
                FieldInit {
                    name: ident("x"),
                    value: Expr::Lit(Lit::Int(10), sp()),
                    span: sp(),
                },
                FieldInit {
                    name: ident("y"),
                    value: Expr::Lit(Lit::Int(20), sp()),
                    span: sp(),
                },
            ],
            span: sp(),
        };

        let cf = eval_expr(&mut env, &expr).unwrap();
        match cf {
            ControlFlow::Value(Value::Struct { name, fields }) => {
                assert_eq!(name, "Point");
                assert!(matches!(fields.get("x"), Some(Value::Int(10))));
                assert!(matches!(fields.get("y"), Some(Value::Int(20))));
            }
            _ => panic!("expected Struct value"),
        }
    }

    #[test]
    fn test_eval_match_struct_pattern() {
        // match Point { x: 3, y: 4 } { Point { x, y } => x + y }
        use strata_ast::ast::{MatchArm, PatField, Path};
        let mut env = Env::new();

        // Build the struct value
        let scrutinee = Expr::StructExpr {
            path: Path {
                segments: vec![ident("Point")],
                span: sp(),
            },
            fields: vec![
                FieldInit {
                    name: ident("x"),
                    value: Expr::Lit(Lit::Int(3), sp()),
                    span: sp(),
                },
                FieldInit {
                    name: ident("y"),
                    value: Expr::Lit(Lit::Int(4), sp()),
                    span: sp(),
                },
            ],
            span: sp(),
        };

        let expr = Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![MatchArm {
                pat: Pat::Struct {
                    path: Path {
                        segments: vec![ident("Point")],
                        span: sp(),
                    },
                    fields: vec![
                        PatField {
                            name: ident("x"),
                            pat: Pat::Ident(ident("x")),
                            span: sp(),
                        },
                        PatField {
                            name: ident("y"),
                            pat: Pat::Ident(ident("y")),
                            span: sp(),
                        },
                    ],
                    span: sp(),
                },
                body: Expr::Binary {
                    lhs: Box::new(Expr::Var(ident("x"))),
                    op: BinOp::Add,
                    rhs: Box::new(Expr::Var(ident("y"))),
                    span: sp(),
                },
                span: sp(),
            }],
            span: sp(),
        };

        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(7))));
    }

    #[test]
    fn test_eval_nested_tuple_pattern() {
        // match ((1, 2), 3) { ((a, b), c) => a + b + c }
        use strata_ast::ast::MatchArm;
        let mut env = Env::new();

        let scrutinee = Expr::Tuple {
            elems: vec![
                Expr::Tuple {
                    elems: vec![Expr::Lit(Lit::Int(1), sp()), Expr::Lit(Lit::Int(2), sp())],
                    span: sp(),
                },
                Expr::Lit(Lit::Int(3), sp()),
            ],
            span: sp(),
        };

        let expr = Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![MatchArm {
                pat: Pat::Tuple(
                    vec![
                        Pat::Tuple(vec![Pat::Ident(ident("a")), Pat::Ident(ident("b"))], sp()),
                        Pat::Ident(ident("c")),
                    ],
                    sp(),
                ),
                body: Expr::Binary {
                    lhs: Box::new(Expr::Binary {
                        lhs: Box::new(Expr::Var(ident("a"))),
                        op: BinOp::Add,
                        rhs: Box::new(Expr::Var(ident("b"))),
                        span: sp(),
                    }),
                    op: BinOp::Add,
                    rhs: Box::new(Expr::Var(ident("c"))),
                    span: sp(),
                },
                span: sp(),
            }],
            span: sp(),
        };

        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Int(6))));
    }

    // ================================================================
    // Issue 012: Affine Integrity — Runtime tombstone tests
    // ================================================================

    #[test]
    fn test_affine_cap_tombstoned_after_use() {
        let mut env = Env::new();
        env.define("fs".to_string(), Value::Cap(CapKind::Fs), false);

        // First access should succeed and return the cap
        let expr = Expr::Var(ident("fs"));
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Cap(CapKind::Fs))));

        // Value in env should now be Consumed
        let val = env.get("fs").unwrap();
        assert!(matches!(val, Value::Consumed { .. }));
    }

    #[test]
    fn test_consumed_cap_gives_runtime_error() {
        let mut env = Env::new();
        env.define("fs".to_string(), Value::Cap(CapKind::Fs), false);

        // First use succeeds
        let expr = Expr::Var(ident("fs"));
        eval_expr(&mut env, &expr).unwrap();

        // Second use hits the tombstone
        let err = eval_expr(&mut env, &expr).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("CAP-MOVE-RUNTIME"),
            "expected CAP-MOVE-RUNTIME, got: {msg}"
        );
        assert!(
            msg.contains("fs"),
            "error should mention var name, got: {msg}"
        );
    }

    #[test]
    fn test_consumed_error_message_includes_both_spans() {
        let mut env = Env::new();
        env.define("net".to_string(), Value::Cap(CapKind::Net), false);

        // First use with identifiable span (becomes the "transferred at" span)
        let id1 = Ident {
            text: "net".to_string(),
            span: Span { start: 42, end: 45 },
        };
        eval_expr(&mut env, &Expr::Var(id1)).unwrap();

        // Second use with different span (becomes the "used at" span)
        let id2 = Ident {
            text: "net".to_string(),
            span: Span {
                start: 100,
                end: 103,
            },
        };
        let err = eval_expr(&mut env, &Expr::Var(id2)).unwrap_err();
        let msg = err.to_string();
        // Error should include both spans
        assert!(
            msg.contains("previously transferred at: 42:45"),
            "should include moved_at span, got: {msg}"
        );
        assert!(
            msg.contains("used at: 100:103"),
            "should include use_span, got: {msg}"
        );
        assert!(
            msg.contains("CAP-MOVE-RUNTIME"),
            "should include error code, got: {msg}"
        );
        assert!(
            msg.contains("Strata bug"),
            "should identify as compiler bug, got: {msg}"
        );
    }

    #[test]
    fn test_borrow_does_not_tombstone() {
        let mut env = Env::new();
        env.define("fs".to_string(), Value::Cap(CapKind::Fs), false);

        // Borrow should NOT consume
        let borrow_expr = Expr::Borrow(Box::new(Expr::Var(ident("fs"))), sp());
        let cf = eval_expr(&mut env, &borrow_expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Cap(CapKind::Fs))));

        // Cap should still be alive in env (not consumed)
        let val = env.get("fs").unwrap();
        assert!(matches!(val, Value::Cap(CapKind::Fs)));

        // Can still borrow again
        let cf2 = eval_expr(&mut env, &borrow_expr).unwrap();
        assert!(matches!(cf2, ControlFlow::Value(Value::Cap(CapKind::Fs))));
    }

    #[test]
    fn test_non_affine_values_unaffected() {
        let mut env = Env::new();
        env.define("x".to_string(), Value::Int(42), false);
        env.define("s".to_string(), Value::Str("hello".to_string()), false);

        // Non-affine values can be read multiple times without tombstoning
        let expr_x = Expr::Var(ident("x"));
        let expr_s = Expr::Var(ident("s"));

        eval_expr(&mut env, &expr_x).unwrap();
        eval_expr(&mut env, &expr_x).unwrap(); // second read fine

        eval_expr(&mut env, &expr_s).unwrap();
        eval_expr(&mut env, &expr_s).unwrap(); // second read fine

        // Values still intact
        assert!(matches!(env.get("x"), Some(Value::Int(42))));
    }

    #[test]
    fn test_scope_tombstone_at_actual_depth() {
        // Regression test for the scoping exploit found in adversarial review.
        // If move_out inserts Consumed as a shadow in the CURRENT scope
        // (instead of at the actual binding depth), popping the inner scope
        // would resurrect the capability.
        let mut env = Env::new();
        env.define("fs".to_string(), Value::Cap(CapKind::Fs), false);

        // Push inner scope and consume there
        env.push_scope();
        let expr = Expr::Var(ident("fs"));
        eval_expr(&mut env, &expr).unwrap();

        // Pop inner scope
        env.pop_scope().unwrap();

        // Outer scope should have Consumed — NOT the original Cap
        let val = env.get("fs").unwrap();
        assert!(
            matches!(val, Value::Consumed { .. }),
            "expected Consumed after inner scope pop, got: {:?}",
            val
        );
    }

    #[test]
    fn test_nested_scope_tombstone_prevents_reuse() {
        // Deep nesting: cap defined in scope 0, consumed in scope 2
        let mut env = Env::new();
        env.define("cap".to_string(), Value::Cap(CapKind::Time), false);

        env.push_scope(); // scope 1
        env.push_scope(); // scope 2

        // Consume in scope 2
        let expr = Expr::Var(ident("cap"));
        eval_expr(&mut env, &expr).unwrap();

        env.pop_scope().unwrap(); // back to scope 1
        env.pop_scope().unwrap(); // back to scope 0

        // Should be Consumed in scope 0, not resurrected
        let err = eval_expr(&mut env, &expr).unwrap_err();
        assert!(err.to_string().contains("CAP-MOVE-RUNTIME"));
    }

    #[test]
    fn test_move_out_returns_none_for_undefined() {
        let mut env = Env::new();
        let result = env.move_out("nonexistent", sp());
        assert!(result.is_none());
    }

    #[test]
    fn test_borrow_of_consumed_cap_gives_error() {
        let mut env = Env::new();
        env.define("fs".to_string(), Value::Cap(CapKind::Fs), false);

        // Consume via Var
        let expr = Expr::Var(ident("fs"));
        eval_expr(&mut env, &expr).unwrap();

        // Borrow after consume should also fail
        let borrow_expr = Expr::Borrow(Box::new(Expr::Var(ident("fs"))), sp());
        let err = eval_expr(&mut env, &borrow_expr).unwrap_err();
        assert!(err.to_string().contains("CAP-MOVE-RUNTIME"));
    }

    #[test]
    fn test_is_affine() {
        assert!(Value::Cap(CapKind::Fs).is_affine());
        assert!(Value::Cap(CapKind::Net).is_affine());
        assert!(!Value::Int(42).is_affine());
        assert!(!Value::Str("hello".to_string()).is_affine());
        assert!(!Value::Bool(true).is_affine());
        assert!(!Value::Unit.is_affine());
        assert!(!Value::HostFn("read_file".to_string()).is_affine());
    }

    #[test]
    fn test_compound_type_containing_cap_is_tombstoned() {
        // A tuple containing a Cap should be treated as affine.
        let mut env = Env::new();
        env.define(
            "t".to_string(),
            Value::Tuple(vec![Value::Cap(CapKind::Fs), Value::Int(42)]),
            false,
        );

        // First access succeeds and tombstones
        let expr = Expr::Var(ident("t"));
        let cf = eval_expr(&mut env, &expr).unwrap();
        assert!(matches!(cf, ControlFlow::Value(Value::Tuple(_))));

        // Second access hits the tombstone
        let err = eval_expr(&mut env, &expr).unwrap_err();
        assert!(err.to_string().contains("CAP-MOVE-RUNTIME"));
    }

    #[test]
    fn test_nested_compound_affine() {
        // A struct containing a tuple containing a Cap should also be affine.
        let mut env = Env::new();
        let mut fields = HashMap::new();
        fields.insert(
            "inner".to_string(),
            Value::Tuple(vec![Value::Cap(CapKind::Net)]),
        );
        env.define(
            "s".to_string(),
            Value::Struct {
                name: "Wrapper".to_string(),
                fields,
            },
            false,
        );

        // First access succeeds
        let expr = Expr::Var(ident("s"));
        eval_expr(&mut env, &expr).unwrap();

        // Second access hits the tombstone
        let err = eval_expr(&mut env, &expr).unwrap_err();
        assert!(err.to_string().contains("CAP-MOVE-RUNTIME"));
    }

    #[test]
    fn test_compound_without_cap_is_not_affine() {
        // A tuple of (Int, String) should NOT be tombstoned — freely copyable.
        let mut env = Env::new();
        env.define(
            "t".to_string(),
            Value::Tuple(vec![Value::Int(1), Value::Str("hello".to_string())]),
            false,
        );

        let expr = Expr::Var(ident("t"));
        eval_expr(&mut env, &expr).unwrap();
        eval_expr(&mut env, &expr).unwrap(); // second read fine

        // Value still intact
        assert!(env.get("t").is_some());
        assert!(!matches!(env.get("t"), Some(Value::Consumed { .. })));
    }

    #[test]
    fn test_is_affine_compound() {
        // Tuple with cap
        assert!(Value::Tuple(vec![Value::Cap(CapKind::Fs), Value::Int(1)]).is_affine());
        // Tuple without cap
        assert!(!Value::Tuple(vec![Value::Int(1), Value::Bool(true)]).is_affine());
        // Empty tuple
        assert!(!Value::Tuple(vec![]).is_affine());

        // Struct with cap in field
        let mut fields = HashMap::new();
        fields.insert("cap".to_string(), Value::Cap(CapKind::Net));
        assert!(Value::Struct {
            name: "S".to_string(),
            fields
        }
        .is_affine());

        // Struct without cap
        let mut fields2 = HashMap::new();
        fields2.insert("x".to_string(), Value::Int(42));
        assert!(!Value::Struct {
            name: "S".to_string(),
            fields: fields2
        }
        .is_affine());

        // Variant with cap
        assert!(Value::Variant {
            enum_name: "E".to_string(),
            variant_name: "V".to_string(),
            fields: vec![Value::Cap(CapKind::Time)],
        }
        .is_affine());

        // Variant without cap
        assert!(!Value::Variant {
            enum_name: "E".to_string(),
            variant_name: "V".to_string(),
            fields: vec![Value::Int(1)],
        }
        .is_affine());
    }
}
