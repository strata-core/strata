//! Evaluator for Strata programs
//!
//! Implements a tree-walking interpreter with proper scoping,
//! closures, and control flow (return, break, continue).

use anyhow::{bail, Result};
use std::cell::Cell;
use std::collections::HashMap;
use strata_ast::ast::{BinOp, Block, Expr, Lit, Module, Stmt, UnOp};

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
#[derive(Debug, Clone, Default)]
pub struct Env {
    scopes: Vec<HashMap<String, Binding>>,
}

impl Env {
    /// Create a new environment with a single empty scope
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
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

    // Pass 1: Define all function names as mutable placeholders
    // This allows forward references and self-references
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

        // Variable lookup
        Expr::Var(id) => match env.get(&id.text) {
            Some(v) => Ok(ControlFlow::Value(v.clone())),
            None => bail!("undefined variable `{}`", id.text),
        },

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
    env.push_scope();

    // Evaluate each statement
    for stmt in &block.stmts {
        let cf = eval_stmt(env, stmt)?;
        // Propagate returns early
        if cf.is_return() {
            env.pop_scope()?;
            return Ok(cf);
        }
    }

    // Evaluate tail expression if present
    let result = if let Some(ref tail) = block.tail {
        eval_expr(env, tail)?
    } else {
        ControlFlow::Value(Value::Unit)
    };

    env.pop_scope()?;
    Ok(result)
}

/// Evaluate a statement
fn eval_stmt(env: &mut Env, stmt: &Stmt) -> Result<ControlFlow> {
    match stmt {
        Stmt::Let {
            mutable,
            name,
            value,
            ..
        } => {
            let cf = eval_expr(env, value)?;
            if cf.is_return() {
                return Ok(cf);
            }
            let v = cf.into_value();
            env.define(name.text.clone(), v, *mutable);
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

    let closure = match cf.into_value() {
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
                name: ident("x"),
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
                name: ident("x"),
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
                    name: ident("x"),
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
                    name: ident("x"),
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
                    name: ident("x"),
                    ty: None,
                    value: Expr::Lit(Lit::Int(1), sp()),
                    span: sp(),
                },
                Stmt::Expr {
                    expr: Expr::Block(Block {
                        stmts: vec![Stmt::Let {
                            mutable: false,
                            name: ident("x"),
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
                    name: ident("sum"),
                    ty: None,
                    value: Expr::Lit(Lit::Int(0), sp()),
                    span: sp(),
                },
                Stmt::Let {
                    mutable: true,
                    name: ident("i"),
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
}
