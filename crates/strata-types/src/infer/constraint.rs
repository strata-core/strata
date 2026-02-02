//! Constraint-based type inference
//!
//! Uses a two-phase approach:
//! 1. Generate constraints from AST (this module)
//! 2. Solve constraints via unification (solver module)
//!
//! This is more extensible than Algorithm W and better suited
//! for adding effects and other extensions later.

use super::ty::{free_vars, Constraint, Scheme, Ty, TypeVarId};
use std::collections::{HashMap, HashSet};
use strata_ast::ast::{BinOp, Block, Expr, Lit, Stmt, UnOp};
use strata_ast::span::Span;

/// Errors that can occur during type inference
#[derive(Debug, Clone)]
pub enum InferError {
    /// Reference to an unknown variable
    UnknownVariable { name: String, span: Span },
    /// Assignment to an immutable variable
    ImmutableAssignment { name: String, span: Span },
    /// Feature not yet implemented
    NotImplemented { msg: String, span: Span },
}

/// Context for type checking within a scope
///
/// Tracks the type environment, mutability of variables, and expected return type
/// for return statement checking.
#[derive(Clone)]
pub struct CheckContext {
    /// Maps variable names to their type schemes
    pub env: HashMap<String, Scheme>,
    /// Tracks which variables are mutable
    pub mutability: HashMap<String, bool>,
    /// Expected return type for `return` statements (None if not in a function)
    pub expected_return: Option<Ty>,
}

impl CheckContext {
    /// Create a new empty check context
    pub fn new() -> Self {
        CheckContext {
            env: HashMap::new(),
            mutability: HashMap::new(),
            expected_return: None,
        }
    }

    /// Create a context from an existing environment
    pub fn from_env(env: HashMap<String, Scheme>) -> Self {
        CheckContext {
            env,
            mutability: HashMap::new(),
            expected_return: None,
        }
    }

    /// Create a child context with the same expected_return
    pub fn child(&self) -> Self {
        CheckContext {
            env: self.env.clone(),
            mutability: self.mutability.clone(),
            expected_return: self.expected_return.clone(),
        }
    }

    /// Add a binding to the context
    pub fn bind(&mut self, name: String, scheme: Scheme, mutable: bool) {
        self.env.insert(name.clone(), scheme);
        self.mutability.insert(name, mutable);
    }

    /// Check if a variable is mutable
    pub fn is_mutable(&self, name: &str) -> Option<bool> {
        self.mutability.get(name).copied()
    }
}

impl Default for CheckContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Inference context for constraint generation
pub struct InferCtx {
    /// Counter for generating fresh type variables
    fresh_counter: u32,
    /// Collected constraints
    constraints: Vec<Constraint>,
}

impl InferCtx {
    /// Create a new inference context
    pub fn new() -> Self {
        InferCtx {
            fresh_counter: 0,
            constraints: vec![],
        }
    }

    /// Generate a fresh type variable
    pub fn fresh_var(&mut self) -> Ty {
        let id = TypeVarId(self.fresh_counter);
        self.fresh_counter += 1;
        Ty::Var(id)
    }

    /// Add a constraint to the collection
    pub fn add_constraint(&mut self, c: Constraint) {
        self.constraints.push(c);
    }

    /// Take all collected constraints
    pub fn take_constraints(&mut self) -> Vec<Constraint> {
        std::mem::take(&mut self.constraints)
    }

    /// Generalize a type into a scheme
    ///
    /// Free variables in `ty` that are NOT in `env_vars` become ∀-bound.
    ///
    /// Example:
    /// - ty = t0 -> t0
    /// - env_vars = {} (empty environment)
    /// - Result: ∀t0. t0 -> t0 (polymorphic identity)
    pub fn generalize(&self, ty: Ty, env_vars: &HashSet<TypeVarId>) -> Scheme {
        let ty_vars = free_vars(&ty);

        // Variables to generalize = those in type but not in environment
        let mut gen_vars: Vec<TypeVarId> = ty_vars.difference(env_vars).copied().collect();

        // Sort for determinism (makes debugging easier)
        gen_vars.sort_by_key(|v| v.0);

        Scheme { vars: gen_vars, ty }
    }

    /// Infer the type of an expression, generating constraints
    ///
    /// `env` maps variable names to their type schemes
    /// This is the simple version without full CheckContext (for backwards compatibility)
    pub fn infer_expr(
        &mut self,
        env: &HashMap<String, Scheme>,
        expr: &Expr,
    ) -> Result<Ty, InferError> {
        let ctx = CheckContext::from_env(env.clone());
        self.infer_expr_ctx(&ctx, expr)
    }

    /// Infer the type of an expression with full context
    ///
    /// `ctx` provides the type environment, mutability tracking, and expected return type
    pub fn infer_expr_ctx(&mut self, ctx: &CheckContext, expr: &Expr) -> Result<Ty, InferError> {
        match expr {
            // Literals have known types
            Expr::Lit(lit, _) => Ok(self.infer_lit(lit)),

            // Variables: look up scheme and instantiate
            Expr::Var(ident) => {
                if let Some(scheme) = ctx.env.get(&ident.text) {
                    Ok(scheme.instantiate(|| self.fresh_var()))
                } else {
                    Err(InferError::UnknownVariable {
                        name: ident.text.clone(),
                        span: ident.span,
                    })
                }
            }

            // Parentheses: just infer the inner expression
            Expr::Paren { inner, .. } => self.infer_expr_ctx(ctx, inner),

            // Unary operations
            Expr::Unary { op, expr, span } => self.infer_unary_ctx(ctx, *op, expr, *span),

            // Binary operations
            Expr::Binary { lhs, op, rhs, span } => self.infer_binary_ctx(ctx, *op, lhs, rhs, *span),

            // Function calls
            Expr::Call { callee, args, .. } => {
                // Infer function type
                let func_ty = self.infer_expr_ctx(ctx, callee)?;

                // Infer argument types
                let arg_tys: Result<Vec<Ty>, InferError> = args
                    .iter()
                    .map(|arg| self.infer_expr_ctx(ctx, arg))
                    .collect();
                let arg_tys = arg_tys?;

                // Create fresh var for result
                let result_ty = self.fresh_var();

                // Add constraint: func_ty = (arg_tys) -> result_ty
                let expected_fn_ty = Ty::arrow(arg_tys, result_ty.clone());
                let call_span = match callee.as_ref() {
                    Expr::Var(id) => id.span,
                    Expr::Paren { span, .. } => *span,
                    _ => Span { start: 0, end: 0 }, // Fallback
                };
                self.add_constraint(Constraint::Equal(func_ty, expected_fn_ty, call_span));

                Ok(result_ty)
            }

            // Block expression
            Expr::Block(block) => self.infer_block(ctx, block),

            // If expression
            Expr::If {
                cond,
                then_,
                else_,
                span,
            } => self.infer_if(ctx, cond, then_, else_.as_deref(), *span),

            // While loop
            Expr::While { cond, body, span } => self.infer_while(ctx, cond, body, *span),
        }
    }

    /// Infer the type of a block expression
    ///
    /// Block type = tail expression type, or Unit if no tail
    pub fn infer_block(&mut self, ctx: &CheckContext, block: &Block) -> Result<Ty, InferError> {
        // Create a child context for this block scope
        let mut block_ctx = ctx.child();

        // Process each statement in order
        for stmt in &block.stmts {
            self.infer_stmt(&mut block_ctx, stmt)?;
        }

        // Block type = tail expression type, or Unit if no tail
        if let Some(ref tail) = block.tail {
            self.infer_expr_ctx(&block_ctx, tail)
        } else {
            Ok(Ty::unit())
        }
    }

    /// Infer the type of a statement
    ///
    /// Statements don't have a type per se, but they can:
    /// - Add bindings to the context (Let)
    /// - Generate constraints (Expr, Assign, Return)
    fn infer_stmt(&mut self, ctx: &mut CheckContext, stmt: &Stmt) -> Result<(), InferError> {
        match stmt {
            Stmt::Let {
                mutable,
                name,
                ty,
                value,
                span,
            } => {
                // Infer the type of the value
                let value_ty = self.infer_expr_ctx(ctx, value)?;

                // If there's a type annotation, add constraint
                if let Some(ann_ty) = ty {
                    if let Ok(expected) = ty_from_type_expr(ann_ty) {
                        self.add_constraint(Constraint::Equal(value_ty.clone(), expected, *span));
                    }
                }

                // Add binding to context (monomorphic for now)
                ctx.bind(name.text.clone(), Scheme::mono(value_ty), *mutable);

                Ok(())
            }

            Stmt::Assign {
                target,
                value,
                span,
            } => {
                // Check that target exists and is mutable
                let target_scheme =
                    ctx.env
                        .get(&target.text)
                        .ok_or_else(|| InferError::UnknownVariable {
                            name: target.text.clone(),
                            span: target.span,
                        })?;

                // Check mutability
                let is_mutable = ctx.is_mutable(&target.text).unwrap_or(false);
                if !is_mutable {
                    return Err(InferError::ImmutableAssignment {
                        name: target.text.clone(),
                        span: *span,
                    });
                }

                // Infer value type
                let value_ty = self.infer_expr_ctx(ctx, value)?;

                // Constrain value type to match target type
                let target_ty = target_scheme.instantiate(|| self.fresh_var());
                self.add_constraint(Constraint::Equal(value_ty, target_ty, *span));

                Ok(())
            }

            Stmt::Expr { expr, .. } => {
                // Infer type but discard it
                let _ = self.infer_expr_ctx(ctx, expr)?;
                Ok(())
            }

            Stmt::Return { value, span } => {
                // Get expected return type
                let expected_ret = ctx.expected_return.clone().unwrap_or_else(Ty::unit);

                if let Some(val_expr) = value {
                    // return expr; - infer expr type and constrain to expected return
                    let val_ty = self.infer_expr_ctx(ctx, val_expr)?;
                    self.add_constraint(Constraint::Equal(val_ty, expected_ret, *span));
                } else {
                    // return; - constrain Unit to expected return
                    self.add_constraint(Constraint::Equal(Ty::unit(), expected_ret, *span));
                }

                Ok(())
            }
        }
    }

    /// Infer the type of an if expression
    fn infer_if(
        &mut self,
        ctx: &CheckContext,
        cond: &Expr,
        then_: &Block,
        else_: Option<&Expr>,
        span: Span,
    ) -> Result<Ty, InferError> {
        // Condition must be Bool
        let cond_ty = self.infer_expr_ctx(ctx, cond)?;
        self.add_constraint(Constraint::Equal(cond_ty, Ty::bool_(), span));

        // Infer then-branch type
        let then_ty = self.infer_block(ctx, then_)?;

        if let Some(else_expr) = else_ {
            // Infer else-branch type
            let else_ty = self.infer_expr_ctx(ctx, else_expr)?;

            // Both branches must unify
            self.add_constraint(Constraint::Equal(then_ty.clone(), else_ty, span));

            Ok(then_ty)
        } else {
            // No else: then-branch must be Unit
            self.add_constraint(Constraint::Equal(then_ty, Ty::unit(), span));

            Ok(Ty::unit())
        }
    }

    /// Infer the type of a while loop
    fn infer_while(
        &mut self,
        ctx: &CheckContext,
        cond: &Expr,
        body: &Block,
        span: Span,
    ) -> Result<Ty, InferError> {
        // Condition must be Bool
        let cond_ty = self.infer_expr_ctx(ctx, cond)?;
        self.add_constraint(Constraint::Equal(cond_ty, Ty::bool_(), span));

        // Infer body type (discarded)
        let _ = self.infer_block(ctx, body)?;

        // While always returns Unit
        Ok(Ty::unit())
    }

    /// Infer type of a literal
    fn infer_lit(&self, lit: &Lit) -> Ty {
        match lit {
            Lit::Int(_) => Ty::int(),
            Lit::Float(_) => Ty::float(),
            Lit::Bool(_) => Ty::bool_(),
            Lit::Str(_) => Ty::string(),
            Lit::Nil => Ty::unit(),
        }
    }

    /// Infer type of unary operation (with context)
    fn infer_unary_ctx(
        &mut self,
        ctx: &CheckContext,
        op: UnOp,
        expr: &Expr,
        span: Span,
    ) -> Result<Ty, InferError> {
        let expr_ty = self.infer_expr_ctx(ctx, expr)?;

        match op {
            UnOp::Not => {
                // !e requires e : Bool, returns Bool
                self.add_constraint(Constraint::Equal(expr_ty, Ty::bool_(), span));
                Ok(Ty::bool_())
            }
            UnOp::Neg => {
                // -e requires e : Int (or Float, but Int for now)
                self.add_constraint(Constraint::Equal(expr_ty, Ty::int(), span));
                Ok(Ty::int())
            }
        }
    }

    /// Infer type of binary operation (with context)
    fn infer_binary_ctx(
        &mut self,
        ctx: &CheckContext,
        op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
        span: Span,
    ) -> Result<Ty, InferError> {
        let lhs_ty = self.infer_expr_ctx(ctx, lhs)?;
        let rhs_ty = self.infer_expr_ctx(ctx, rhs)?;

        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                // Arithmetic requires numeric types
                // For now, constrain to Int (Float support can be added later)
                self.add_constraint(Constraint::Equal(lhs_ty, Ty::int(), span));
                self.add_constraint(Constraint::Equal(rhs_ty, Ty::int(), span));
                Ok(Ty::int())
            }

            // Comparison: both Int or both Float, returns Bool
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.add_constraint(Constraint::Equal(lhs_ty, rhs_ty, span));
                Ok(Ty::bool_())
            }

            // Equality: both same type, returns Bool
            BinOp::Eq | BinOp::Ne => {
                self.add_constraint(Constraint::Equal(lhs_ty, rhs_ty, span));
                Ok(Ty::bool_())
            }

            // Logical: both Bool, returns Bool
            BinOp::And | BinOp::Or => {
                self.add_constraint(Constraint::Equal(lhs_ty, Ty::bool_(), span));
                self.add_constraint(Constraint::Equal(rhs_ty, Ty::bool_(), span));
                Ok(Ty::bool_())
            }
        }
    }
}

/// Convert a TypeExpr from the AST to an inference type
fn ty_from_type_expr(te: &strata_ast::ast::TypeExpr) -> Result<Ty, InferError> {
    use strata_ast::ast::TypeExpr;
    match te {
        TypeExpr::Path(path, span) => {
            let name = &path[0].text;
            match name.as_str() {
                "Unit" => Ok(Ty::unit()),
                "Bool" => Ok(Ty::bool_()),
                "Int" => Ok(Ty::int()),
                "Float" => Ok(Ty::float()),
                "String" => Ok(Ty::string()),
                _ => Err(InferError::NotImplemented {
                    msg: format!("Unknown type: {}", name),
                    span: *span,
                }),
            }
        }
        TypeExpr::Arrow { params, ret, .. } => {
            let param_tys: Result<Vec<Ty>, InferError> =
                params.iter().map(ty_from_type_expr).collect();
            let param_tys = param_tys?;
            let ret_ty = ty_from_type_expr(ret)?;
            Ok(Ty::arrow(param_tys, ret_ty))
        }
    }
}

impl Default for InferCtx {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use strata_ast::ast::{BinOp, Expr, Lit};
    use strata_ast::span::Span;

    #[test]
    fn fresh_vars_are_unique() {
        let mut ctx = InferCtx::new();
        let v1 = ctx.fresh_var();
        let v2 = ctx.fresh_var();
        let v3 = ctx.fresh_var();

        assert_eq!(v1, Ty::Var(TypeVarId(0)));
        assert_eq!(v2, Ty::Var(TypeVarId(1)));
        assert_eq!(v3, Ty::Var(TypeVarId(2)));
    }

    #[test]
    fn can_collect_constraints() {
        let mut ctx = InferCtx::new();
        let t1 = Ty::int();
        let t2 = Ty::bool_();
        let span = Span { start: 0, end: 0 };

        ctx.add_constraint(Constraint::Equal(t1.clone(), t2.clone(), span));
        ctx.add_constraint(Constraint::Equal(t2.clone(), t1.clone(), span));

        let constraints = ctx.take_constraints();
        assert_eq!(constraints.len(), 2);
    }

    #[test]
    fn generalize_no_free_vars() {
        let ctx = InferCtx::new();
        let ty = Ty::int();
        let env_vars = HashSet::new();

        let scheme = ctx.generalize(ty.clone(), &env_vars);
        assert_eq!(scheme.vars.len(), 0);
        assert_eq!(scheme.ty, ty);
    }

    #[test]
    fn generalize_all_free() {
        let ctx = InferCtx::new();
        // Type: t0 -> t0
        let ty = Ty::arrow1(Ty::Var(TypeVarId(0)), Ty::Var(TypeVarId(0)));
        let env_vars = HashSet::new(); // Empty environment

        let scheme = ctx.generalize(ty.clone(), &env_vars);
        assert_eq!(scheme.vars, vec![TypeVarId(0)]);
        assert_eq!(scheme.ty, ty);
    }

    #[test]
    fn generalize_some_in_env() {
        let ctx = InferCtx::new();
        // Type: t0 -> t1
        let ty = Ty::arrow1(Ty::Var(TypeVarId(0)), Ty::Var(TypeVarId(1)));

        // t0 is in environment, so don't generalize it
        let mut env_vars = HashSet::new();
        env_vars.insert(TypeVarId(0));

        let scheme = ctx.generalize(ty.clone(), &env_vars);
        // Only t1 should be generalized
        assert_eq!(scheme.vars, vec![TypeVarId(1)]);
        assert_eq!(scheme.ty, ty);
    }

    #[test]
    fn infer_literal() {
        let mut ctx = InferCtx::new();
        let env = HashMap::new();

        let expr = Expr::Lit(Lit::Int(42), Span { start: 0, end: 2 });
        let ty = ctx.infer_expr(&env, &expr).unwrap();

        assert_eq!(ty, Ty::int());
    }

    #[test]
    fn infer_binary_generates_constraints() {
        let mut ctx = InferCtx::new();
        let env = HashMap::new();

        // 1 + 2
        let expr = Expr::Binary {
            lhs: Box::new(Expr::Lit(Lit::Int(1), Span { start: 0, end: 1 })),
            op: BinOp::Add,
            rhs: Box::new(Expr::Lit(Lit::Int(2), Span { start: 4, end: 5 })),
            span: Span { start: 0, end: 5 },
        };

        let ty = ctx.infer_expr(&env, &expr).unwrap();
        assert_eq!(ty, Ty::int());

        let constraints = ctx.take_constraints();
        assert_eq!(constraints.len(), 2); // Two constraints: lhs = Int, rhs = Int
    }
}
