// crates/strata-types/src/checker.rs
// Type checker for Strata - validates programs and infers types

use super::infer::ty::{Scheme, Ty};
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
    /// Feature not yet implemented
    NotImplemented { msg: String, span: Span },
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
            TypeError::NotImplemented { msg, span } => {
                write!(f, "{} at {:?}", msg, span)
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

    /// Type check an entire module using two-pass approach
    ///
    /// Pass 1: Predeclare all functions (add signatures to environment)
    /// Pass 2: Check let bindings and function bodies
    pub fn check_module(&mut self, module: &Module) -> Result<(), TypeError> {
        use super::infer::ty::free_vars_env;

        // Pass 1: Predeclare all functions
        for item in &module.items {
            if let Item::Fn(decl) = item {
                // Extract function signature
                let fn_ty = self.extract_fn_signature(decl)?;

                // Generalize the signature
                let env_vars = free_vars_env(&self.env);
                let fn_scheme = self.infer_ctx.generalize(fn_ty, &env_vars);

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
            .map_err(|e| unifier_error_to_type_error(e, decl.span))?;

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
    /// The function's type has already been predeclared in Pass 1.
    /// We now check the body and verify it matches the declared type.
    fn check_fn(&mut self, decl: &strata_ast::ast::FnDecl) -> Result<(), TypeError> {
        // Get the predeclared function type from environment
        let predeclared_scheme = self
            .env
            .get(&decl.name.text)
            .expect("Function should be predeclared in pass 1")
            .clone();

        // Instantiate the predeclared scheme to get concrete types
        let predeclared_ty = predeclared_scheme.instantiate(|| self.infer_ctx.fresh_var());

        // Extract param and return types from the arrow
        let (param_tys, ret_ty) = match predeclared_ty {
            Ty::Arrow(params, ret) => (params, *ret),
            _ => panic!("Function type should be an arrow"),
        };

        // Create a new environment for the function body
        let mut fn_env = self.env.clone();

        // Add parameters to function environment
        for (param, param_ty) in decl.params.iter().zip(param_tys.iter()) {
            use super::infer::ty::Scheme;
            fn_env.insert(param.name.text.clone(), Scheme::mono(param_ty.clone()));
        }

        // Infer body type from the Block's tail expression
        // (Full block inference will be implemented in Phase 3)
        let body_ty = if let Some(ref tail_expr) = decl.body.tail {
            self.infer_ctx
                .infer_expr(&fn_env, tail_expr)
                .map_err(infer_error_to_type_error)?
        } else {
            // Empty block or no tail expression returns Unit
            Ty::unit()
        };

        // Constrain body type to match return type
        self.infer_ctx
            .add_constraint(super::infer::ty::Constraint::Equal(
                body_ty,
                ret_ty.clone(),
                decl.span,
            ));

        // Solve constraints
        let constraints = self.infer_ctx.take_constraints();
        let mut solver = Solver::new();
        let _subst = solver
            .solve(constraints)
            .map_err(|e| unifier_error_to_type_error(e, decl.span))?;

        // Note: We don't need to re-add the function to env - it's already there from pass 1

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
        super::infer::constraint::InferError::NotImplemented { msg, span } => {
            TypeError::NotImplemented { msg, span }
        }
    }
}

/// Convert unifier TypeError to checker TypeError with span context
fn unifier_error_to_type_error(err: super::infer::unifier::TypeError, span: Span) -> TypeError {
    match err {
        super::infer::unifier::TypeError::Mismatch(expected, found) => TypeError::Mismatch {
            expected,
            found,
            span,
        },
        super::infer::unifier::TypeError::Occurs { var, ty } => {
            // Occurs check failure - format as mismatch for now
            TypeError::Mismatch {
                expected: Ty::Var(var),
                found: ty,
                span,
            }
        }
        super::infer::unifier::TypeError::Arity { left, right } => {
            // Arity mismatch - format as NotImplemented for now
            TypeError::NotImplemented {
                msg: format!(
                    "Arity mismatch: expected {} arguments, found {}",
                    left, right
                ),
                span,
            }
        }
    }
}
