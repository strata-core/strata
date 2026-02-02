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
use strata_ast::ast::{BinOp, Expr, Lit, UnOp};
use strata_ast::span::Span;

/// Errors that can occur during type inference
#[derive(Debug, Clone)]
pub enum InferError {
    /// Reference to an unknown variable
    UnknownVariable { name: String, span: Span },
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
    pub fn infer_expr(
        &mut self,
        env: &HashMap<String, Scheme>,
        expr: &Expr,
    ) -> Result<Ty, InferError> {
        match expr {
            // Literals have known types
            Expr::Lit(lit, _) => Ok(self.infer_lit(lit)),

            // Variables: look up scheme and instantiate
            Expr::Var(ident) => {
                if let Some(scheme) = env.get(&ident.text) {
                    Ok(scheme.instantiate(|| self.fresh_var()))
                } else {
                    Err(InferError::UnknownVariable {
                        name: ident.text.clone(),
                        span: ident.span,
                    })
                }
            }

            // Parentheses: just infer the inner expression
            Expr::Paren { inner, .. } => self.infer_expr(env, inner),

            // Unary operations
            Expr::Unary { op, expr, span } => self.infer_unary(env, *op, expr, *span),

            // Binary operations
            Expr::Binary { lhs, op, rhs, span } => self.infer_binary(env, *op, lhs, rhs, *span),

            // Function calls
            Expr::Call { callee, args, .. } => {
                // Infer function type
                let func_ty = self.infer_expr(env, callee)?;

                // Infer argument types
                let arg_tys: Result<Vec<Ty>, InferError> =
                    args.iter().map(|arg| self.infer_expr(env, arg)).collect();
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
        }
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

    /// Infer type of unary operation
    fn infer_unary(
        &mut self,
        env: &std::collections::HashMap<String, Scheme>,
        op: UnOp,
        expr: &Expr,
        _span: Span,
    ) -> Result<Ty, InferError> {
        let expr_ty = self.infer_expr(env, expr)?;

        match op {
            UnOp::Not => {
                // !e requires e : Bool, returns Bool
                self.add_constraint(Constraint::Equal(expr_ty, Ty::bool_(), _span));
                Ok(Ty::bool_())
            }
            UnOp::Neg => {
                // -e requires e : Int (or Float, but Int for now)
                self.add_constraint(Constraint::Equal(expr_ty, Ty::int(), _span));
                Ok(Ty::int())
            }
        }
    }

    /// Infer type of binary operation
    fn infer_binary(
        &mut self,
        env: &std::collections::HashMap<String, Scheme>,
        op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
        _span: Span,
    ) -> Result<Ty, InferError> {
        let lhs_ty = self.infer_expr(env, lhs)?;
        let rhs_ty = self.infer_expr(env, rhs)?;

        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                // Arithmetic requires numeric types
                // For now, constrain to Int (Float support can be added later)
                self.add_constraint(Constraint::Equal(lhs_ty, Ty::int(), _span));
                self.add_constraint(Constraint::Equal(rhs_ty, Ty::int(), _span));
                Ok(Ty::int())
            }

            // Comparison: both Int or both Float, returns Bool
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.add_constraint(Constraint::Equal(lhs_ty, rhs_ty, _span));
                Ok(Ty::bool_())
            }

            // Equality: both same type, returns Bool
            BinOp::Eq | BinOp::Ne => {
                self.add_constraint(Constraint::Equal(lhs_ty, rhs_ty, _span));
                Ok(Ty::bool_())
            }

            // Logical: both Bool, returns Bool
            BinOp::And | BinOp::Or => {
                self.add_constraint(Constraint::Equal(lhs_ty, Ty::bool_(), _span));
                self.add_constraint(Constraint::Equal(rhs_ty, Ty::bool_(), _span));
                Ok(Ty::bool_())
            }
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
