// crates/strata-types/src/checker.rs
// Type checker for Strata - validates programs and infers types

use super::infer::ty::{Scheme, Ty, TypeVarId};
use super::infer::{InferCtx, Solver};
use std::collections::HashMap;
use strata_ast::ast::{Item, LetDecl, Module, TypeExpr};
use strata_ast::span::Span;

/// Type errors that can occur during type checking
#[derive(Debug, Clone)]
pub enum TypeError {
    /// Type mismatch - expected one type but found another
    Mismatch { expected: Ty, found: Ty, span: Span },
    /// Reference to an unknown variable
    UnknownVariable { name: String, span: Span },
    /// Assignment to an immutable variable
    ImmutableAssignment { name: String, span: Span },
    /// Feature not yet implemented
    NotImplemented { msg: String, span: Span },
    /// Inference depth limit exceeded (pathological input)
    DepthLimitExceeded { span: Span },
    /// Occurs check failure (infinite type)
    OccursCheck { var: TypeVarId, ty: Ty, span: Span },
    /// Arity mismatch (different number of arguments)
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },
    /// Internal invariant violation (indicates a bug in the type checker)
    InvariantViolation { msg: String, span: Span },
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TypeError::Mismatch {
                expected,
                found,
                span,
            } => {
                write!(
                    f,
                    "Type mismatch at {:?}: expected {}, found {}",
                    span, expected, found
                )
            }
            TypeError::UnknownVariable { name, span } => {
                write!(f, "Unknown variable '{}' at {:?}", name, span)
            }
            TypeError::ImmutableAssignment { name, span } => {
                write!(
                    f,
                    "Cannot assign to immutable variable '{}' at {:?}",
                    name, span
                )
            }
            TypeError::NotImplemented { msg, span } => {
                write!(f, "{} at {:?}", msg, span)
            }
            TypeError::DepthLimitExceeded { span } => {
                write!(
                    f,
                    "Type inference depth limit exceeded at {:?} (pathological input)",
                    span
                )
            }
            TypeError::OccursCheck { var, ty, span } => {
                write!(f, "Infinite type at {:?}: {} occurs in {}", span, var, ty)
            }
            TypeError::ArityMismatch {
                expected,
                found,
                span,
            } => {
                write!(
                    f,
                    "Arity mismatch at {:?}: expected {} arguments, found {}",
                    span, expected, found
                )
            }
            TypeError::InvariantViolation { msg, span } => {
                write!(
                    f,
                    "Internal error at {:?}: {} (this is a bug in the type checker)",
                    span, msg
                )
            }
        }
    }
}

impl std::error::Error for TypeError {}

/// Type checker with environment for let bindings
pub struct TypeChecker {
    /// Maps variable names to their type schemes
    env: HashMap<String, Scheme>,
    /// Inference context for constraint generation
    infer_ctx: InferCtx,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    /// Create a new type checker with an empty environment
    pub fn new() -> Self {
        Self {
            env: HashMap::new(),
            infer_ctx: InferCtx::new(),
        }
    }

    /// Infer the type of an expression
    ///
    /// This is the main entry point for expression type checking.
    /// It creates a CheckContext, infers the expression type, solves constraints,
    /// and returns the final type.
    pub fn infer_expr(&mut self, expr: &strata_ast::ast::Expr) -> Result<Ty, TypeError> {
        use super::infer::constraint::CheckContext;

        // Create a CheckContext from the current environment
        let ctx = CheckContext::from_env(self.env.clone());

        // Infer the expression type
        let ty = self
            .infer_ctx
            .infer_expr_ctx(&ctx, expr)
            .map_err(infer_error_to_type_error)?;

        // Solve constraints
        let constraints = self.infer_ctx.take_constraints();
        let mut solver = Solver::new();
        let subst = solver
            .solve(constraints)
            .map_err(solve_error_to_type_error)?;

        // Apply substitution to get final type
        Ok(subst.apply(&ty))
    }

    /// Type check an entire module using two-pass approach
    ///
    /// Pass 1: Predeclare all functions with MONOMORPHIC signatures
    ///         (prevents polymorphic self-reference in recursive calls)
    /// Pass 2: Check let bindings and function bodies
    ///         After checking each function, generalize and update env
    pub fn check_module(&mut self, module: &Module) -> Result<(), TypeError> {
        // Pass 1: Predeclare all functions with MONOMORPHIC signatures
        // This ensures that recursive calls see the same type variables,
        // preventing unsound polymorphic self-reference.
        for item in &module.items {
            if let Item::Fn(decl) = item {
                // Extract function signature with fresh type vars
                let fn_ty = self.extract_fn_signature(decl)?;

                // Store MONOMORPHIC placeholder - do NOT generalize yet!
                // This is critical: recursive calls must see the same type vars.
                let fn_scheme = Scheme::mono(fn_ty);

                // Add to environment
                self.env.insert(decl.name.text.clone(), fn_scheme);
            }
        }

        // Pass 2: Check all items (let bindings and function bodies)
        for item in &module.items {
            self.check_item(item)?;
        }

        Ok(())
    }

    /// Type check a single top-level item
    fn check_item(&mut self, item: &Item) -> Result<(), TypeError> {
        match item {
            Item::Let(decl) => self.check_let(decl),
            Item::Fn(decl) => self.check_fn(decl),
        }
    }

    /// Type check a let declaration
    fn check_let(&mut self, decl: &LetDecl) -> Result<(), TypeError> {
        // Infer the type of the value expression
        let inferred_ty = self
            .infer_ctx
            .infer_expr(&self.env, &decl.value)
            .map_err(infer_error_to_type_error)?;

        // If there's a type annotation, add constraint
        if let Some(ann_ty) = &decl.ty {
            let expected = ty_from_type_expr(ann_ty)?;
            self.infer_ctx
                .add_constraint(super::infer::ty::Constraint::Equal(
                    inferred_ty.clone(),
                    expected,
                    decl.span,
                ));
        }

        // Solve constraints
        let constraints = self.infer_ctx.take_constraints();
        let mut solver = Solver::new();
        let subst = solver
            .solve(constraints)
            .map_err(solve_error_to_type_error)?;

        // Apply substitution to get final type
        let final_ty = subst.apply(&inferred_ty);

        // Generalize: free vars in type that aren't already in environment become ∀-bound
        use super::infer::ty::free_vars_env;
        let env_vars = free_vars_env(&self.env);
        let scheme = self.infer_ctx.generalize(final_ty, &env_vars);

        // Add to environment
        self.env.insert(decl.name.text.clone(), scheme);

        Ok(())
    }

    /// Type check a function declaration (Pass 2)
    ///
    /// The function's type has already been predeclared in Pass 1 as MONOMORPHIC.
    /// We now check the body, solve constraints, apply substitution, and THEN generalize.
    fn check_fn(&mut self, decl: &strata_ast::ast::FnDecl) -> Result<(), TypeError> {
        use super::infer::constraint::CheckContext;
        use super::infer::ty::{free_vars_env, Scheme};

        // Get the predeclared function type from environment (monomorphic)
        let predeclared_scheme = self
            .env
            .get(&decl.name.text)
            .ok_or_else(|| TypeError::InvariantViolation {
                msg: format!("function '{}' not predeclared in pass 1", decl.name.text),
                span: decl.name.span,
            })?
            .clone();

        // The scheme is monomorphic (no ∀-bound vars), so instantiate returns the same type.
        // This is critical: recursive calls within the body will see the SAME type vars.
        let predeclared_ty = predeclared_scheme.instantiate(|| self.infer_ctx.fresh_var());

        // Extract param and return types from the arrow
        let (param_tys, ret_ty) = match &predeclared_ty {
            Ty::Arrow(params, ret) => (params.clone(), ret.as_ref().clone()),
            _ => {
                return Err(TypeError::InvariantViolation {
                    msg: format!(
                        "expected function type to be an arrow, found {}",
                        predeclared_ty
                    ),
                    span: decl.name.span,
                });
            }
        };

        // Create a CheckContext for the function body
        let mut fn_ctx = CheckContext::from_env(self.env.clone());
        fn_ctx.expected_return = Some(ret_ty.clone());

        // Add parameters to function context (parameters are immutable)
        for (param, param_ty) in decl.params.iter().zip(param_tys.iter()) {
            fn_ctx.bind(
                param.name.text.clone(),
                Scheme::mono(param_ty.clone()),
                false,
            );
        }

        // Infer body type using full block inference
        let body_ty = self
            .infer_ctx
            .infer_block(&fn_ctx, &decl.body)
            .map_err(infer_error_to_type_error)?;

        // Constrain body type to match return type (unless body always diverges)
        // A diverging body (Never) satisfies any return type.
        if body_ty != Ty::Never {
            self.infer_ctx
                .add_constraint(super::infer::ty::Constraint::Equal(
                    body_ty,
                    ret_ty.clone(),
                    decl.span,
                ));
        }

        // Solve constraints
        let constraints = self.infer_ctx.take_constraints();
        let mut solver = Solver::new();
        let subst = solver
            .solve(constraints)
            .map_err(solve_error_to_type_error)?;

        // Apply substitution to get the final function type
        let final_fn_ty = subst.apply(&predeclared_ty);

        // NOW generalize: compute env vars excluding this function's own type vars
        // (since this function is still monomorphic in env, its vars are included in env_vars,
        // but we want to generalize those vars if they're not constrained by the environment)
        let mut env_for_generalize = self.env.clone();
        env_for_generalize.remove(&decl.name.text);
        let env_vars = free_vars_env(&env_for_generalize);
        let gen_scheme = self.infer_ctx.generalize(final_fn_ty, &env_vars);

        // Update environment with the generalized scheme
        self.env.insert(decl.name.text.clone(), gen_scheme);

        Ok(())
    }

    /// Extract a function's type signature without checking its body
    ///
    /// This is used in pass 1 to predeclare functions.
    fn extract_fn_signature(&mut self, decl: &strata_ast::ast::FnDecl) -> Result<Ty, TypeError> {
        // Extract parameter types
        let mut param_tys = Vec::new();
        for param in &decl.params {
            let param_ty = if let Some(ref ty_expr) = param.ty {
                // Parameter has type annotation
                ty_from_type_expr(ty_expr)?
            } else {
                // No annotation - create fresh type variable
                self.infer_ctx.fresh_var()
            };
            param_tys.push(param_ty);
        }

        // Extract return type
        let ret_ty = if let Some(ref ty_expr) = decl.ret_ty {
            ty_from_type_expr(ty_expr)?
        } else {
            self.infer_ctx.fresh_var()
        };

        // Create function type: (params) -> ret
        Ok(Ty::arrow(param_tys, ret_ty))
    }
}

/// Convert a TypeExpr from the AST to an inference type
#[allow(clippy::only_used_in_recursion)]
fn ty_from_type_expr(te: &TypeExpr) -> Result<Ty, TypeError> {
    match te {
        TypeExpr::Path(path, span) => {
            // For now, just handle simple type names
            let name = &path[0].text;
            match name.as_str() {
                "Unit" => Ok(Ty::unit()),
                "Bool" => Ok(Ty::bool_()),
                "Int" => Ok(Ty::int()),
                "Float" => Ok(Ty::float()),
                "String" => Ok(Ty::string()),
                _ => Err(TypeError::NotImplemented {
                    msg: format!("Unknown type: {}", name),
                    span: *span,
                }),
            }
        }
        TypeExpr::Arrow { params, ret, .. } => {
            let param_tys: Result<Vec<Ty>, TypeError> =
                params.iter().map(ty_from_type_expr).collect();
            let param_tys = param_tys?;
            let ret_ty = ty_from_type_expr(ret)?;
            Ok(Ty::arrow(param_tys, ret_ty))
        }
    }
}

/// Convert InferError to TypeError
fn infer_error_to_type_error(err: super::infer::constraint::InferError) -> TypeError {
    match err {
        super::infer::constraint::InferError::UnknownVariable { name, span } => {
            TypeError::UnknownVariable { name, span }
        }
        super::infer::constraint::InferError::ImmutableAssignment { name, span } => {
            TypeError::ImmutableAssignment { name, span }
        }
        super::infer::constraint::InferError::NotImplemented { msg, span } => {
            TypeError::NotImplemented { msg, span }
        }
        super::infer::constraint::InferError::DepthLimitExceeded { span } => {
            TypeError::DepthLimitExceeded { span }
        }
    }
}

/// Convert a SolveError (which includes span) to checker TypeError
fn solve_error_to_type_error(err: super::infer::solver::SolveError) -> TypeError {
    let span = err.span;
    match err.error {
        super::infer::unifier::TypeError::Mismatch(expected, found) => TypeError::Mismatch {
            expected,
            found,
            span,
        },
        super::infer::unifier::TypeError::Occurs { var, ty } => {
            TypeError::OccursCheck { var, ty, span }
        }
        super::infer::unifier::TypeError::Arity { left, right } => TypeError::ArityMismatch {
            expected: left,
            found: right,
            span,
        },
    }
}
