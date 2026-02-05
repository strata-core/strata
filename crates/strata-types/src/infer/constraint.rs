//! Constraint-based type inference
//!
//! Uses a two-phase approach:
//! 1. Generate constraints from AST (this module)
//! 2. Solve constraints via unification (solver module)
//!
//! This is more extensible than Algorithm W and better suited
//! for adding effects and other extensions later.

use super::ty::{free_vars, Constraint, Scheme, Ty, TypeVarId};
use crate::adt::AdtRegistry;
use crate::exhaustive::{self, ExhaustivenessError};
use std::collections::{HashMap, HashSet};
use strata_ast::ast::{BinOp, Block, Expr, FieldInit, Lit, MatchArm, Pat, Path, Stmt, UnOp};
use strata_ast::span::Span;

/// Maximum inference depth to prevent stack overflow from pathological input
const MAX_INFER_DEPTH: u32 = 512;

/// Errors that can occur during type inference
#[derive(Debug, Clone)]
pub enum InferError {
    /// Reference to an unknown variable
    UnknownVariable { name: String, span: Span },
    /// Assignment to an immutable variable
    ImmutableAssignment { name: String, span: Span },
    /// Feature not yet implemented
    NotImplemented { msg: String, span: Span },
    /// Inference depth limit exceeded (pathological input)
    DepthLimitExceeded { span: Span },
    /// Duplicate binding in pattern
    DuplicateBinding { name: String, span: Span },
    /// Unknown type in pattern
    UnknownType { name: String, span: Span },
    /// Unknown variant in pattern
    UnknownVariant {
        type_name: String,
        variant: String,
        span: Span,
    },
    /// Wrong number of fields in pattern
    PatternArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },
    /// Missing field in struct pattern/expression
    MissingField {
        struct_name: String,
        field: String,
        span: Span,
    },
    /// Unknown field in struct pattern/expression
    UnknownField {
        struct_name: String,
        field: String,
        span: Span,
    },
    /// Duplicate field in struct expression
    DuplicateField { field: String, span: Span },
    /// Tuple arity exceeds limit
    TupleArityLimit {
        max: usize,
        found: usize,
        span: Span,
    },
    /// Match is not exhaustive
    NonExhaustiveMatch { witness: String, span: Span },
    /// Pattern arm is unreachable
    UnreachablePattern { arm_index: usize, span: Span },
    /// Exhaustiveness checking limit exceeded (DoS protection)
    ExhaustivenessLimitExceeded { msg: String, span: Span },
    /// Refutable pattern in let binding (should use match instead)
    RefutablePattern { pat_desc: String, span: Span },
    /// Capability type found in a binding
    CapabilityInBinding { cap_type: String, span: Span },
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
    /// ADT registry for looking up struct/enum definitions (for pattern checking)
    pub adt_registry: Option<AdtRegistry>,
}

impl CheckContext {
    /// Create a new empty check context
    pub fn new() -> Self {
        CheckContext {
            env: HashMap::new(),
            mutability: HashMap::new(),
            expected_return: None,
            adt_registry: None,
        }
    }

    /// Create a context from an existing environment
    pub fn from_env(env: HashMap<String, Scheme>) -> Self {
        CheckContext {
            env,
            mutability: HashMap::new(),
            expected_return: None,
            adt_registry: None,
        }
    }

    /// Create a context from an existing environment and ADT registry
    pub fn from_env_with_registry(env: HashMap<String, Scheme>, registry: AdtRegistry) -> Self {
        CheckContext {
            env,
            mutability: HashMap::new(),
            expected_return: None,
            adt_registry: Some(registry),
        }
    }

    /// Create a child context with the same expected_return and registry
    pub fn child(&self) -> Self {
        CheckContext {
            env: self.env.clone(),
            mutability: self.mutability.clone(),
            expected_return: self.expected_return.clone(),
            adt_registry: self.adt_registry.clone(),
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
    /// Current inference depth (for recursion limit)
    depth: u32,
}

impl InferCtx {
    /// Create a new inference context
    pub fn new() -> Self {
        InferCtx {
            fresh_counter: 0,
            constraints: vec![],
            depth: 0,
        }
    }

    /// Enter a new level of inference depth
    fn enter_depth(&mut self, span: Span) -> Result<(), InferError> {
        self.depth += 1;
        if self.depth > MAX_INFER_DEPTH {
            Err(InferError::DepthLimitExceeded { span })
        } else {
            Ok(())
        }
    }

    /// Exit a level of inference depth
    fn exit_depth(&mut self) {
        self.depth = self.depth.saturating_sub(1);
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
        // Track depth to prevent stack overflow from deeply nested expressions
        let span = expr.span();
        self.enter_depth(span)?;
        let result = self.infer_expr_inner(ctx, expr);
        self.exit_depth();
        result
    }

    /// Inner implementation of expression inference
    fn infer_expr_inner(&mut self, ctx: &CheckContext, expr: &Expr) -> Result<Ty, InferError> {
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
            Expr::Call { callee, args, span } => {
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
                    Expr::Paren {
                        span: paren_span, ..
                    } => *paren_span,
                    Expr::PathExpr(path) => path.span,
                    _ => *span, // Use the call expression span as fallback
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

            // Match expression
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => self.infer_match(ctx, scrutinee, arms, *span),

            // Tuple expression
            Expr::Tuple { elems, span } => self.infer_tuple(ctx, elems, *span),

            // Struct expression
            Expr::StructExpr { path, fields, span } => {
                self.infer_struct_expr(ctx, path, fields, *span)
            }

            // Path expression (enum constructor)
            Expr::PathExpr(path) => self.infer_path_expr(ctx, path),
        }
    }

    /// Infer the type of a block expression
    ///
    /// Block type = tail expression type, or Unit if no tail (or Never if it diverges)
    pub fn infer_block(&mut self, ctx: &CheckContext, block: &Block) -> Result<Ty, InferError> {
        // Create a child context for this block scope
        let mut block_ctx = ctx.child();

        // Process each statement in order
        for stmt in &block.stmts {
            self.infer_stmt(&mut block_ctx, stmt)?;
        }

        // Block type = tail expression type, or Unit if no tail
        // Special case: if the last statement is a return, the block type is Never
        if let Some(ref tail) = block.tail {
            self.infer_expr_ctx(&block_ctx, tail)
        } else if block
            .stmts
            .last()
            .is_some_and(|s| matches!(s, Stmt::Return { .. }))
        {
            // Block ends with return statement - it always diverges
            Ok(Ty::Never)
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
                pat,
                ty,
                value,
                span,
            } => {
                // Infer the type of the value
                let value_ty = self.infer_expr_ctx(ctx, value)?;

                // If there's a type annotation, add constraint
                // (only allowed for simple identifier patterns)
                if let Some(ann_ty) = ty {
                    let expected = ty_from_type_expr(ann_ty)?;
                    self.add_constraint(Constraint::Equal(value_ty.clone(), expected, *span));
                }

                // Check that the pattern is irrefutable
                if !is_irrefutable(ctx, pat) {
                    return Err(InferError::RefutablePattern {
                        pat_desc: Self::refutable_pattern_desc(pat),
                        span: pat.span(),
                    });
                }

                // Check for capability types in the binding type.
                // This is a best-effort check: value_ty may still contain
                // unresolved type variables, which will be caught post-solving
                // when the effect system lands (Issue 008).
                if crate::adt::contains_capability(&value_ty) {
                    let cap_name = crate::adt::find_capability_name(&value_ty)
                        .unwrap_or("capability".to_string());
                    return Err(InferError::CapabilityInBinding {
                        cap_type: cap_name,
                        span: *span,
                    });
                }

                // Check pattern against value type, get bindings
                let bindings = self.check_pattern(ctx, pat, &value_ty)?;

                // Check for duplicate bindings
                self.check_duplicate_bindings(&bindings)?;

                // Add bindings to context as MONOMORPHIC.
                //
                // Design decision: Block-level let bindings are not generalized because:
                // 1. Simplicity: Generalizing requires solving constraints first, which
                //    complicates the single-pass inference within blocks.
                // 2. Safety: Premature generalization can lead to unsoundness when the
                //    value contains type variables that escape to outer scope.
                // 3. Practicality: Most local bindings don't need polymorphism.
                //
                // For polymorphic local values, users can define a nested function:
                //   let id = fn(x) { x }; // id is polymorphic as a fn decl
                //   let x = id(1);        // x: Int
                //   let y = id(true);     // y: Bool
                for binding in bindings {
                    ctx.bind(binding.name, Scheme::mono(binding.ty), *mutable);
                }

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

            // Handle Never (diverging branches) specially:
            // - If then diverges, result type is else type
            // - If else diverges, result type is then type
            // - If both diverge, result is Never
            // - Otherwise, both must unify
            match (&then_ty, &else_ty) {
                (Ty::Never, Ty::Never) => Ok(Ty::Never),
                (Ty::Never, _) => Ok(else_ty), // then diverges, use else type
                (_, Ty::Never) => Ok(then_ty), // else diverges, use then type
                _ => {
                    // Neither diverges: must unify
                    self.add_constraint(Constraint::Equal(then_ty.clone(), else_ty, span));
                    Ok(then_ty)
                }
            }
        } else {
            // No else: then-branch must be Unit (unless it diverges)
            if then_ty != Ty::Never {
                self.add_constraint(Constraint::Equal(then_ty, Ty::unit(), span));
            }
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

    // ============ Phase 4: Match, Tuple, Struct, Path Inference ============

    /// Infer the type of a match expression
    fn infer_match(
        &mut self,
        ctx: &CheckContext,
        scrutinee: &Expr,
        arms: &[MatchArm],
        span: Span,
    ) -> Result<Ty, InferError> {
        // Infer scrutinee type
        let scrutinee_ty = self.infer_expr_ctx(ctx, scrutinee)?;

        // Special case: matching on Never - empty match is valid, returns Never
        if scrutinee_ty == Ty::Never {
            return Ok(Ty::Never);
        }

        let mut result_ty: Option<Ty> = None;

        for arm in arms {
            // Check pattern against scrutinee type, get bindings
            let bindings = self.check_pattern(ctx, &arm.pat, &scrutinee_ty)?;

            // Check for duplicate bindings within this pattern
            self.check_duplicate_bindings(&bindings)?;

            // Create child context with pattern bindings
            let mut arm_ctx = ctx.child();
            for binding in &bindings {
                arm_ctx.bind(
                    binding.name.clone(),
                    Scheme::mono(binding.ty.clone()),
                    false,
                );
            }

            // Infer arm body type
            let arm_ty = self.infer_expr_ctx(&arm_ctx, &arm.body)?;

            // Join arm types (Never arms don't contribute)
            if arm_ty != Ty::Never {
                match &result_ty {
                    None => result_ty = Some(arm_ty),
                    Some(existing) => {
                        self.add_constraint(Constraint::Equal(existing.clone(), arm_ty, arm.span));
                    }
                }
            }
        }

        // Exhaustiveness and redundancy checking
        // Note: We only skip exhaustiveness checking if the scrutinee type ITSELF is an
        // unresolved inference variable. If it's an ADT that contains type variables as
        // arguments (e.g., Option<T> in a generic function), we still run exhaustiveness
        // because the constructor set is known (Some/None) even if the payload type isn't.
        if let Some(registry) = &ctx.adt_registry {
            // Only skip if the scrutinee is a bare type variable (not yet resolved)
            let is_unresolved_var = matches!(&scrutinee_ty, Ty::Var(_));
            if is_unresolved_var {
                // Skip exhaustiveness check - scrutinee type not yet resolved
                return Ok(result_ty.unwrap_or(Ty::Never));
            }
            match exhaustive::check_match(arms, &scrutinee_ty, registry, span) {
                Ok((witness_opt, redundant)) => {
                    // Check for non-exhaustive match
                    if let Some(witness) = witness_opt {
                        return Err(InferError::NonExhaustiveMatch {
                            witness: format!("{}", witness),
                            span,
                        });
                    }
                    // Report first unreachable pattern (if any)
                    if let Some(&arm_idx) = redundant.first() {
                        // Find the span for this arm
                        let arm_span = arms.get(arm_idx).map(|a| a.span).unwrap_or(span);
                        return Err(InferError::UnreachablePattern {
                            arm_index: arm_idx,
                            span: arm_span,
                        });
                    }
                }
                Err(ExhaustivenessError::MatrixTooLarge { size, span }) => {
                    return Err(InferError::ExhaustivenessLimitExceeded {
                        msg: format!("pattern matrix too large: {} elements", size),
                        span,
                    });
                }
                Err(ExhaustivenessError::DepthExceeded { span }) => {
                    return Err(InferError::ExhaustivenessLimitExceeded {
                        msg: "exhaustiveness check depth limit exceeded".to_string(),
                        span,
                    });
                }
                Err(ExhaustivenessError::NonExhaustive { witness, span }) => {
                    return Err(InferError::NonExhaustiveMatch {
                        witness: format!("{}", witness),
                        span,
                    });
                }
            }
        }

        // Return joined type or Never if all arms diverge
        Ok(result_ty.unwrap_or(Ty::Never))
    }

    /// Check a pattern against an expected type, returning bindings
    fn check_pattern(
        &mut self,
        ctx: &CheckContext,
        pat: &Pat,
        expected: &Ty,
    ) -> Result<Vec<PatternBinding>, InferError> {
        match pat {
            Pat::Wildcard(_) => Ok(vec![]),

            Pat::Ident(ident) => Ok(vec![PatternBinding {
                name: ident.text.clone(),
                ty: expected.clone(),
                span: ident.span,
            }]),

            Pat::Literal(lit, span) => {
                let lit_ty = self.infer_lit(lit);
                self.add_constraint(Constraint::Equal(lit_ty, expected.clone(), *span));
                Ok(vec![])
            }

            Pat::Tuple(pats, span) => {
                // Expected must be Tuple of same arity
                match expected {
                    Ty::Tuple(tys) => {
                        if tys.len() != pats.len() {
                            return Err(InferError::PatternArityMismatch {
                                expected: tys.len(),
                                found: pats.len(),
                                span: *span,
                            });
                        }
                        let mut bindings = vec![];
                        for (pat, ty) in pats.iter().zip(tys.iter()) {
                            bindings.extend(self.check_pattern(ctx, pat, ty)?);
                        }
                        Ok(bindings)
                    }
                    _ => {
                        // Expected is a type variable - create fresh vars for elements
                        let elem_tys: Vec<Ty> = pats.iter().map(|_| self.fresh_var()).collect();
                        self.add_constraint(Constraint::Equal(
                            Ty::Tuple(elem_tys.clone()),
                            expected.clone(),
                            *span,
                        ));
                        let mut bindings = vec![];
                        for (pat, ty) in pats.iter().zip(elem_tys.iter()) {
                            bindings.extend(self.check_pattern(ctx, pat, ty)?);
                        }
                        Ok(bindings)
                    }
                }
            }

            Pat::Variant { path, fields, span } => {
                // Look up constructor in environment
                let ctor_name = path_to_string(path);

                let scheme = ctx.env.get(&ctor_name).ok_or_else(|| {
                    // Try to provide a better error message
                    let parts: Vec<&str> = ctor_name.split("::").collect();
                    if parts.len() >= 2 {
                        InferError::UnknownVariant {
                            type_name: parts[..parts.len() - 1].join("::"),
                            variant: parts[parts.len() - 1].to_string(),
                            span: *span,
                        }
                    } else {
                        InferError::UnknownVariable {
                            name: ctor_name.clone(),
                            span: *span,
                        }
                    }
                })?;

                // Instantiate constructor scheme
                let ctor_ty = scheme.instantiate(|| self.fresh_var());

                // Constructor type should be Arrow(params, result)
                let (param_tys, result_ty) = match &ctor_ty {
                    Ty::Arrow(params, ret) => (params.clone(), ret.as_ref().clone()),
                    _ => {
                        // Not a function - might be a unit constructor
                        (vec![], ctor_ty.clone())
                    }
                };

                // Unify result type with expected type
                self.add_constraint(Constraint::Equal(result_ty, expected.clone(), *span));

                // Check arity
                if param_tys.len() != fields.len() {
                    return Err(InferError::PatternArityMismatch {
                        expected: param_tys.len(),
                        found: fields.len(),
                        span: *span,
                    });
                }

                // Check each field pattern
                let mut bindings = vec![];
                for (pat, ty) in fields.iter().zip(param_tys.iter()) {
                    bindings.extend(self.check_pattern(ctx, pat, ty)?);
                }
                Ok(bindings)
            }

            Pat::Struct { path, fields, span } => {
                // Look up struct in registry
                let struct_name = path_to_string(path);

                let registry =
                    ctx.adt_registry
                        .as_ref()
                        .ok_or_else(|| InferError::NotImplemented {
                            msg: "Struct patterns require ADT registry".to_string(),
                            span: *span,
                        })?;

                let adt_def =
                    registry
                        .get(&struct_name)
                        .ok_or_else(|| InferError::UnknownType {
                            name: struct_name.clone(),
                            span: *span,
                        })?;

                // Get struct fields
                let struct_fields = adt_def.fields().ok_or_else(|| InferError::NotImplemented {
                    msg: format!("'{}' is not a struct", struct_name),
                    span: *span,
                })?;

                // Instantiate type parameters with fresh vars
                let type_args: Vec<Ty> = (0..adt_def.arity()).map(|_| self.fresh_var()).collect();
                let struct_ty = Ty::adt(&struct_name, type_args.clone());

                // Unify with expected type
                self.add_constraint(Constraint::Equal(struct_ty, expected.clone(), *span));

                // Create substitution for type parameters
                let type_subst: HashMap<TypeVarId, Ty> = (0..adt_def.arity())
                    .map(|i| (TypeVarId(i as u32), type_args[i].clone()))
                    .collect();

                // Check each field pattern
                let mut bindings = vec![];
                let mut seen_fields = HashSet::new();

                for pat_field in fields {
                    let field_name = &pat_field.name.text;

                    // Check for duplicate fields
                    if !seen_fields.insert(field_name.clone()) {
                        return Err(InferError::DuplicateField {
                            field: field_name.clone(),
                            span: pat_field.span,
                        });
                    }

                    // Find field in struct definition
                    let field_def = struct_fields
                        .iter()
                        .find(|f| f.name == *field_name)
                        .ok_or_else(|| InferError::UnknownField {
                            struct_name: struct_name.clone(),
                            field: field_name.clone(),
                            span: pat_field.span,
                        })?;

                    // Substitute type parameters in field type
                    let field_ty = substitute_type_vars(&field_def.ty, &type_subst);

                    // Check pattern
                    bindings.extend(self.check_pattern(ctx, &pat_field.pat, &field_ty)?);
                }

                // Note: We don't require all fields to be present in patterns
                // (partial matching is allowed)

                Ok(bindings)
            }
        }
    }

    /// Check for duplicate bindings in a pattern
    fn check_duplicate_bindings(&self, bindings: &[PatternBinding]) -> Result<(), InferError> {
        let mut seen = HashSet::new();
        for b in bindings {
            if !seen.insert(&b.name) {
                return Err(InferError::DuplicateBinding {
                    name: b.name.clone(),
                    span: b.span,
                });
            }
        }
        Ok(())
    }

    /// Get a description of why a pattern is refutable
    fn refutable_pattern_desc(pat: &Pat) -> String {
        match pat {
            Pat::Literal(lit, _) => format!("literal pattern `{:?}`", lit),
            Pat::Variant { path, .. } => {
                let name = path
                    .segments
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                format!("enum variant `{}`", name)
            }
            Pat::Tuple(pats, _) => {
                // Find the first refutable sub-pattern
                for (i, p) in pats.iter().enumerate() {
                    if !matches!(p, Pat::Wildcard(_) | Pat::Ident(_)) {
                        return format!("tuple element {} is refutable", i);
                    }
                }
                "tuple pattern".to_string()
            }
            Pat::Struct { path, .. } => {
                format!("struct `{}`", path.as_str())
            }
            _ => "pattern".to_string(),
        }
    }

    /// Infer type of tuple expression
    fn infer_tuple(
        &mut self,
        ctx: &CheckContext,
        elems: &[Expr],
        span: Span,
    ) -> Result<Ty, InferError> {
        const MAX_TUPLE_ARITY: usize = 8;

        if elems.len() > MAX_TUPLE_ARITY {
            return Err(InferError::TupleArityLimit {
                max: MAX_TUPLE_ARITY,
                found: elems.len(),
                span,
            });
        }

        // Empty tuple is Unit
        if elems.is_empty() {
            return Ok(Ty::unit());
        }

        // Single element - just return that type (not a tuple)
        if elems.len() == 1 {
            return self.infer_expr_ctx(ctx, &elems[0]);
        }

        // Infer each element type
        let tys: Vec<Ty> = elems
            .iter()
            .map(|e| self.infer_expr_ctx(ctx, e))
            .collect::<Result<_, _>>()?;

        Ok(Ty::Tuple(tys))
    }

    /// Infer type of struct construction expression
    fn infer_struct_expr(
        &mut self,
        ctx: &CheckContext,
        path: &Path,
        fields: &[FieldInit],
        span: Span,
    ) -> Result<Ty, InferError> {
        let struct_name = path_to_string(path);

        let registry = ctx
            .adt_registry
            .as_ref()
            .ok_or_else(|| InferError::NotImplemented {
                msg: "Struct expressions require ADT registry".to_string(),
                span,
            })?;

        let adt_def = registry
            .get(&struct_name)
            .ok_or_else(|| InferError::UnknownType {
                name: struct_name.clone(),
                span,
            })?;

        // Must be a struct
        let struct_fields = adt_def.fields().ok_or_else(|| InferError::NotImplemented {
            msg: format!("'{}' is not a struct", struct_name),
            span,
        })?;

        // Instantiate type parameters with fresh vars
        let type_args: Vec<Ty> = (0..adt_def.arity()).map(|_| self.fresh_var()).collect();
        let struct_ty = Ty::adt(&struct_name, type_args.clone());

        // Create substitution for type parameters
        let type_subst: HashMap<TypeVarId, Ty> = (0..adt_def.arity())
            .map(|i| (TypeVarId(i as u32), type_args[i].clone()))
            .collect();

        // Track which fields are provided
        let mut seen_fields = HashSet::new();

        for field in fields {
            let field_name = &field.name.text;

            // Check for duplicate fields
            if !seen_fields.insert(field_name.clone()) {
                return Err(InferError::DuplicateField {
                    field: field_name.clone(),
                    span: field.span,
                });
            }

            // Find field in struct definition
            let field_def = struct_fields
                .iter()
                .find(|f| f.name == *field_name)
                .ok_or_else(|| InferError::UnknownField {
                    struct_name: struct_name.clone(),
                    field: field_name.clone(),
                    span: field.span,
                })?;

            // Substitute type parameters in field type
            let expected_ty = substitute_type_vars(&field_def.ty, &type_subst);

            // Infer field value type and constrain
            let value_ty = self.infer_expr_ctx(ctx, &field.value)?;
            self.add_constraint(Constraint::Equal(value_ty, expected_ty, field.span));
        }

        // Check all required fields are provided
        for field_def in struct_fields {
            if !seen_fields.contains(&field_def.name) {
                return Err(InferError::MissingField {
                    struct_name: struct_name.clone(),
                    field: field_def.name.clone(),
                    span,
                });
            }
        }

        Ok(struct_ty)
    }

    /// Infer type of path expression (enum constructor)
    fn infer_path_expr(&mut self, ctx: &CheckContext, path: &Path) -> Result<Ty, InferError> {
        let name = path_to_string(path);

        // Look up in environment - could be an enum constructor
        if let Some(scheme) = ctx.env.get(&name) {
            return Ok(scheme.instantiate(|| self.fresh_var()));
        }

        // Not found
        Err(InferError::UnknownVariable {
            name,
            span: path.span,
        })
    }
}

/// A binding created by a pattern
#[derive(Clone, Debug)]
pub struct PatternBinding {
    pub name: String,
    pub ty: Ty,
    pub span: Span,
}

/// Check if a pattern is irrefutable (always matches).
/// Irrefutable patterns are required for let bindings.
fn is_irrefutable(ctx: &CheckContext, pat: &Pat) -> bool {
    match pat {
        // Wildcard and identifier patterns always match
        Pat::Wildcard(_) | Pat::Ident(_) => true,

        // Literals are always refutable (match specific values)
        Pat::Literal(_, _) => false,

        // Tuple patterns are irrefutable if all sub-patterns are irrefutable
        Pat::Tuple(pats, _) => pats.iter().all(|p| is_irrefutable(ctx, p)),

        // Struct patterns are irrefutable (single constructor)
        // Sub-patterns must also be irrefutable
        Pat::Struct { fields, .. } => fields.iter().all(|f| is_irrefutable(ctx, &f.pat)),

        // Variant patterns are only irrefutable if the enum has exactly one variant
        Pat::Variant { path, fields, .. } => {
            // Extract enum name from path (first segment)
            if let Some(enum_name) = path.segments.first() {
                if let Some(registry) = &ctx.adt_registry {
                    if let Some(adt) = registry.get(&enum_name.text) {
                        if let Some(variants) = adt.variants() {
                            // Single-variant enum is irrefutable
                            if variants.len() == 1 {
                                // All sub-patterns must also be irrefutable
                                return fields.iter().all(|p| is_irrefutable(ctx, p));
                            }
                        }
                    }
                }
            }
            false
        }
    }
}

/// Convert a Path to a string (e.g., "Option::Some")
fn path_to_string(path: &Path) -> String {
    path.segments
        .iter()
        .map(|i| i.text.as_str())
        .collect::<Vec<_>>()
        .join("::")
}

/// Substitute type variables in a type
fn substitute_type_vars(ty: &Ty, subst: &HashMap<TypeVarId, Ty>) -> Ty {
    match ty {
        Ty::Var(v) => subst.get(v).cloned().unwrap_or_else(|| ty.clone()),
        Ty::Const(_) | Ty::Never => ty.clone(),
        Ty::Arrow(params, ret) => Ty::arrow(
            params
                .iter()
                .map(|t| substitute_type_vars(t, subst))
                .collect(),
            substitute_type_vars(ret, subst),
        ),
        Ty::Tuple(tys) => Ty::Tuple(tys.iter().map(|t| substitute_type_vars(t, subst)).collect()),
        Ty::List(t) => Ty::List(Box::new(substitute_type_vars(t, subst))),
        Ty::Adt { name, args } => Ty::Adt {
            name: name.clone(),
            args: args
                .iter()
                .map(|t| substitute_type_vars(t, subst))
                .collect(),
        },
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
        // Generic type annotations in block-level let bindings not yet supported.
        // Workaround: rely on type inference. See Known Limitations in IMPLEMENTED.md.
        TypeExpr::App { base, span, .. } => {
            let name = base
                .iter()
                .map(|i| i.text.as_str())
                .collect::<Vec<_>>()
                .join("::");
            Err(InferError::NotImplemented {
                msg: format!("Generic types not yet implemented: {}", name),
                span: *span,
            })
        }
        // Tuple type annotations in block-level let bindings not yet supported.
        TypeExpr::Tuple(_, span) => Err(InferError::NotImplemented {
            msg: "Tuple types not yet implemented".to_string(),
            span: *span,
        }),
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

    #[test]
    fn depth_limit_triggers_on_deeply_nested_expr() {
        let mut ctx = InferCtx::new();
        let env = HashMap::new();

        // Create a deeply nested unary expression: !!!!!...!true (600 levels)
        let mut expr = Expr::Lit(Lit::Bool(true), Span { start: 0, end: 4 });
        for i in 0..600 {
            expr = Expr::Unary {
                op: strata_ast::ast::UnOp::Not,
                expr: Box::new(expr),
                span: Span {
                    start: i,
                    end: i + 1,
                },
            };
        }

        let result = ctx.infer_expr(&env, &expr);
        assert!(matches!(result, Err(InferError::DepthLimitExceeded { .. })));
    }

    #[test]
    fn normal_nesting_works() {
        let mut ctx = InferCtx::new();
        let env = HashMap::new();

        // Create moderately nested unary expression: !!!!!...!true (100 levels)
        let mut expr = Expr::Lit(Lit::Bool(true), Span { start: 0, end: 4 });
        for i in 0..100 {
            expr = Expr::Unary {
                op: strata_ast::ast::UnOp::Not,
                expr: Box::new(expr),
                span: Span {
                    start: i,
                    end: i + 1,
                },
            };
        }

        let result = ctx.infer_expr(&env, &expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ty::bool_());
    }
}
