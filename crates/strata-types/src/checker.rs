// crates/strata-types/src/checker.rs
// Type checker for Strata - validates programs and infers types

use super::adt::{
    contains_capability, find_capability_name, AdtDef, AdtRegistry, FieldDef, VariantDef,
    VariantFields,
};
use super::effects::{CapKind, Effect, EffectRow};
use super::infer::ty::{free_effect_vars_env, Scheme, Ty, TypeVarId};
use super::infer::{InferCtx, Solver};
use std::collections::HashMap;
use strata_ast::ast::{EnumDef, Ident, Item, LetDecl, Module, StructDef, TypeExpr};
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
    /// Duplicate type definition
    DuplicateType { name: String, span: Span },
    /// Unknown type referenced
    UnknownType { name: String, span: Span },
    /// Unknown variant referenced
    UnknownVariant {
        type_name: String,
        variant: String,
        span: Span,
    },
    /// Capability stored in ADT (forbidden until linear types)
    CapabilityInAdt {
        field: String,
        cap_type: String,
        span: Span,
    },
    /// Missing field in struct expression
    MissingField {
        struct_name: String,
        field: String,
        span: Span,
    },
    /// Unknown field in struct expression
    UnknownField {
        struct_name: String,
        field: String,
        span: Span,
    },
    /// Duplicate field in struct expression
    DuplicateField { field: String, span: Span },
    /// Wrong number of type arguments
    WrongTypeArgCount {
        type_name: String,
        expected: usize,
        found: usize,
        span: Span,
    },
    /// Non-exhaustive match - pattern matching doesn't cover all cases
    NonExhaustiveMatch { witness: String, span: Span },
    /// Unreachable pattern - arm will never match
    UnreachablePattern { arm_index: usize, span: Span },
    /// Exhaustiveness check exceeded limits (DoS protection)
    ExhaustivenessLimitExceeded { msg: String, span: Span },
    /// Refutable pattern in let binding
    RefutablePattern { pat_desc: String, span: Span },
    /// Effect row mismatch
    EffectMismatch {
        expected: crate::effects::EffectRow,
        found: crate::effects::EffectRow,
        span: Span,
    },
    /// Effect variable limit exceeded (DoS protection)
    EffectVarLimitExceeded { limit: u32 },
    /// Cyclic effect variable substitution
    EffectCycle {
        var: crate::effects::EffectVarId,
        span: Span,
    },
    /// Effect substitution chain too deep
    EffectChainTooDeep { depth: usize, span: Span },
    /// Unknown effect name
    UnknownEffect { name: String, span: Span },
    /// Extern function missing required effect annotation
    MissingExternEffects { fn_name: String, span: Span },
    /// Undeclared effect: function uses effects not in its annotation
    UndeclaredEffect {
        effect: String,
        fn_name: String,
        declared: crate::effects::EffectRow,
        actual: crate::effects::EffectRow,
        span: Span,
    },
    /// Function performs effect but lacks the required capability parameter
    MissingCapability {
        effect: String,
        cap_type: String,
        fn_name: String,
        span: Span,
    },
    /// Extern function declares effect without matching capability parameter
    ExternMissingCapability {
        effect: String,
        cap_type: String,
        fn_name: String,
        span: Span,
    },
    /// ADT name shadows a built-in capability type
    ReservedCapabilityName { name: String, span: Span },
    /// Single-use capability has already been used
    CapabilityAlreadyUsed {
        name: String,
        used_at: Span,
        previous_use: Span,
    },
    /// Capability used inside a loop (would be used multiple times)
    CapabilityUsedInLoop { name: String, used_at: Span },
    /// Reference type (&T) escaped its allowed position (extern fn params only)
    RefEscape { ty: Ty, context: String, span: Span },
    /// Reference type (&T) found in ADT field definition
    RefInAdtField { field: String, ty: Ty, span: Span },
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
            TypeError::DuplicateType { name, span } => {
                write!(f, "Duplicate type definition '{}' at {:?}", name, span)
            }
            TypeError::UnknownType { name, span } => {
                write!(f, "Unknown type '{}' at {:?}", name, span)?;
                // Ergonomic hint: user may have written an effect name where a cap type goes
                match name.as_str() {
                    "Fs" | "Net" | "Time" | "Rand" | "Ai" => {
                        write!(
                            f,
                            ". Did you mean {}Cap? Effects like {} go in & {{...}}, capability types are parameter types",
                            name, name
                        )?;
                    }
                    _ => {}
                }
                Ok(())
            }
            TypeError::UnknownVariant {
                type_name,
                variant,
                span,
            } => {
                write!(
                    f,
                    "Unknown variant '{}::{}' at {:?}",
                    type_name, variant, span
                )
            }
            TypeError::CapabilityInAdt {
                field,
                cap_type,
                span,
            } => {
                write!(
                    f,
                    "Capability '{}' cannot be stored in ADT field '{}' at {:?}. \
                     Storing capabilities requires linear types (planned for Issue 011). \
                     Pass capabilities as function parameters instead.",
                    cap_type, field, span
                )
            }
            TypeError::MissingField {
                struct_name,
                field,
                span,
            } => {
                write!(
                    f,
                    "Missing field '{}' in struct '{}' at {:?}",
                    field, struct_name, span
                )
            }
            TypeError::UnknownField {
                struct_name,
                field,
                span,
            } => {
                write!(
                    f,
                    "Unknown field '{}' in struct '{}' at {:?}",
                    field, struct_name, span
                )
            }
            TypeError::DuplicateField { field, span } => {
                write!(
                    f,
                    "Duplicate field '{}' in struct expression at {:?}",
                    field, span
                )
            }
            TypeError::WrongTypeArgCount {
                type_name,
                expected,
                found,
                span,
            } => {
                write!(
                    f,
                    "Type '{}' expects {} type argument(s), but {} provided at {:?}",
                    type_name, expected, found, span
                )
            }
            TypeError::NonExhaustiveMatch { witness, span } => {
                write!(
                    f,
                    "Non-exhaustive match at {:?}: pattern '{}' not covered",
                    span, witness
                )
            }
            TypeError::UnreachablePattern { arm_index, span } => {
                write!(
                    f,
                    "Unreachable pattern at {:?}: arm {} will never match",
                    span, arm_index
                )
            }
            TypeError::ExhaustivenessLimitExceeded { msg, span } => {
                write!(
                    f,
                    "Exhaustiveness check limit exceeded at {:?}: {}",
                    span, msg
                )
            }
            TypeError::RefutablePattern { pat_desc, span } => {
                write!(
                    f,
                    "Refutable pattern in let binding at {:?}: {} may not match all values. \
                     Use `match` instead.",
                    span, pat_desc
                )
            }
            TypeError::EffectMismatch {
                expected,
                found,
                span,
            } => {
                write!(
                    f,
                    "Effect mismatch at {:?}: expected {}, found {}",
                    span, expected, found
                )
            }
            TypeError::EffectVarLimitExceeded { limit } => {
                write!(
                    f,
                    "Effect variable limit ({}) exceeded (pathological input)",
                    limit
                )
            }
            TypeError::EffectCycle { var, span } => {
                write!(
                    f,
                    "Cyclic effect variable {} at {:?}: effect variable refers to itself",
                    var, span
                )
            }
            TypeError::EffectChainTooDeep { depth, span } => {
                write!(
                    f,
                    "Effect substitution chain too deep ({} steps) at {:?}; possible cycle",
                    depth, span
                )
            }
            TypeError::UnknownEffect { name, span } => {
                write!(
                    f,
                    "Unknown effect '{}' at {:?}; known effects are Fs, Net, Time, Rand, Ai",
                    name, span
                )?;
                // Ergonomic hint: user may have written FsCap in an effect annotation
                if name.ends_with("Cap") {
                    if let Some(effect_name) = name.strip_suffix("Cap") {
                        let normalized = match effect_name {
                            "Fs" | "Net" | "Time" | "Rand" | "Ai" => Some(effect_name),
                            _ => None,
                        };
                        if let Some(eff) = normalized {
                            write!(
                                f,
                                ". Did you mean {{{}}}? Capability types like {} go in parameter types, not effect annotations",
                                eff, name
                            )?;
                        }
                    }
                }
                Ok(())
            }
            TypeError::MissingExternEffects { fn_name, span } => {
                write!(
                    f,
                    "Extern function '{}' at {:?} must declare its effects. \
                     Use `& {{}}` for pure or `& {{Fs, Net, ...}}` for effectful.",
                    fn_name, span
                )
            }
            TypeError::UndeclaredEffect {
                effect,
                fn_name,
                declared,
                actual,
                span,
            } => {
                write!(
                    f,
                    "Function '{}' uses {} but only declares {}; add {} to the effect annotation at {:?}",
                    fn_name, actual, declared, effect, span
                )
            }
            TypeError::MissingCapability {
                effect,
                cap_type,
                fn_name,
                span,
            } => {
                write!(
                    f,
                    "Function '{}' requires capability {} because its effect row includes {{{}}} at {:?}. \
                     Add a `{}: {}` parameter to this function.",
                    fn_name,
                    cap_type,
                    effect,
                    span,
                    cap_type.to_lowercase().replace("cap", ""),
                    cap_type
                )
            }
            TypeError::ExternMissingCapability {
                effect,
                cap_type,
                fn_name,
                span,
            } => {
                write!(
                    f,
                    "Extern function '{}' declares {{{}}} effect but lacks the required '{}' capability parameter at {:?}. \
                     Add a `{}: {}` parameter. Alternatively, remove {{{}}} from the effect annotation if this extern is actually pure.",
                    fn_name,
                    effect,
                    cap_type,
                    span,
                    cap_type.to_lowercase().replace("cap", ""),
                    cap_type,
                    effect
                )
            }
            TypeError::ReservedCapabilityName { name, span } => {
                write!(
                    f,
                    "Type name '{}' is reserved for the built-in {} capability type at {:?}",
                    name,
                    name.to_lowercase().replace("cap", " ").trim(),
                    span
                )
            }
            TypeError::CapabilityAlreadyUsed {
                name,
                used_at,
                previous_use,
            } => {
                write!(
                    f,
                    "capability '{}' has already been used; \
                     permission was transferred at {:?}; \
                     '{}' is no longer available at {:?}",
                    name, previous_use, name, used_at
                )
            }
            TypeError::CapabilityUsedInLoop { name, used_at } => {
                write!(
                    f,
                    "cannot use single-use capability '{}' inside loop at {:?}; \
                     '{}' would be used on every iteration",
                    name, used_at, name
                )
            }
            TypeError::RefEscape { ty, context, span } => {
                write!(
                    f,
                    "reference type '{}' cannot escape to {} at {:?}; \
                     &T is only allowed in extern function parameters",
                    ty, context, span
                )
            }
            TypeError::RefInAdtField { field, ty, span } => {
                write!(
                    f,
                    "reference type '{}' cannot be stored in ADT field '{}' at {:?}; \
                     &T is only allowed in extern function parameters",
                    ty, field, span
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
    /// Registry of ADT (struct/enum) definitions
    adt_registry: AdtRegistry,
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
            adt_registry: AdtRegistry::with_builtins(),
        }
    }

    /// Get a reference to the ADT registry
    pub fn adt_registry(&self) -> &AdtRegistry {
        &self.adt_registry
    }

    /// Infer the type of an expression
    ///
    /// This is the main entry point for expression type checking.
    /// It creates a CheckContext, infers the expression type, solves constraints,
    /// and returns the final type.
    pub fn infer_expr(&mut self, expr: &strata_ast::ast::Expr) -> Result<Ty, TypeError> {
        use super::infer::constraint::CheckContext;

        // Create a CheckContext from the current environment with ADT registry
        let ctx = CheckContext::from_env_with_registry(self.env.clone(), self.adt_registry.clone());

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
        let final_ty = subst
            .apply(&ty)
            .map_err(|e| subst_error_to_type_error(e, expr.span()))?;
        Ok(final_ty)
    }

    /// Type check an entire module using two-pass approach
    ///
    /// Pass 1a: Register all ADT definitions (struct/enum)
    /// Pass 1b: Predeclare all functions with MONOMORPHIC signatures
    ///          Add enum constructors to environment as polymorphic functions
    /// Pass 2: Check let bindings and function bodies
    ///         After checking each function, generalize and update env
    pub fn check_module(&mut self, module: &Module) -> Result<(), TypeError> {
        // Pass 1a: Register all ADT definitions
        for item in &module.items {
            match item {
                Item::Struct(def) => self.register_struct(def)?,
                Item::Enum(def) => self.register_enum(def)?,
                _ => {}
            }
        }

        // Pass 1b: Add enum constructors to environment
        // (Must happen after all ADTs are registered so types can reference each other)
        for item in &module.items {
            if let Item::Enum(def) = item {
                self.register_enum_constructors(def)?;
            }
        }

        // Pass 1c: Predeclare all functions with MONOMORPHIC signatures
        // This ensures that recursive calls see the same type variables,
        // preventing unsound polymorphic self-reference.
        for item in &module.items {
            match item {
                Item::Fn(decl) => {
                    // Extract function signature with fresh type vars
                    let fn_ty = self.extract_fn_signature(decl)?;

                    // Store MONOMORPHIC placeholder - do NOT generalize yet!
                    // This is critical: recursive calls must see the same type vars.
                    let fn_scheme = Scheme::mono(fn_ty);

                    // Add to environment
                    self.env.insert(decl.name.text.clone(), fn_scheme);
                }
                Item::ExternFn(decl) => {
                    // Register extern fn with its type signature (no body to check)
                    let fn_ty = self.extract_extern_fn_signature(decl)?;

                    // Validate: extern fn must have capability parameters matching
                    // all its declared concrete effects. No exceptions.
                    if let Ty::Arrow(ref params, _, ref eff) = fn_ty {
                        let param_caps: Vec<CapKind> = params
                            .iter()
                            .filter_map(|ty| match ty {
                                Ty::Cap(kind) => Some(*kind),
                                Ty::Ref(inner) => match inner.as_ref() {
                                    Ty::Cap(kind) => Some(*kind),
                                    _ => None,
                                },
                                _ => None,
                            })
                            .collect();

                        validate_caps_against_effects(
                            &decl.name.text,
                            &param_caps,
                            eff,
                            decl.span,
                            true,
                        )?;
                    }

                    let fn_scheme = Scheme::mono(fn_ty);
                    self.env.insert(decl.name.text.clone(), fn_scheme);
                }
                _ => {}
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
            // ADT registration happens in pass 1 (register_struct/register_enum)
            Item::Struct(_) => Ok(()),
            Item::Enum(_) => Ok(()),
            // Extern fn has no body; type was registered in pass 1c
            Item::ExternFn(_) => Ok(()),
        }
    }

    /// Type check a let declaration
    fn check_let(&mut self, decl: &LetDecl) -> Result<(), TypeError> {
        // Create a CheckContext with ADT registry so struct/enum expressions work
        use super::infer::constraint::CheckContext;
        let ctx = CheckContext::from_env_with_registry(self.env.clone(), self.adt_registry.clone());

        // Infer the type of the value expression
        let inferred_ty = self
            .infer_ctx
            .infer_expr_ctx(&ctx, &decl.value)
            .map_err(infer_error_to_type_error)?;

        // If there's a type annotation, add constraint
        if let Some(ann_ty) = &decl.ty {
            let expected = self.ty_from_type_expr(ann_ty)?;
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
        let final_ty = subst
            .apply(&inferred_ty)
            .map_err(|e| subst_error_to_type_error(e, decl.span))?;

        // ---- Settle point: reject &T escaping to let bindings ----
        // After solving, the type may have resolved to contain Ty::Ref through
        // inference (e.g., `let r = &fs;`). Refs are second-class — they cannot
        // appear in let binding types.
        if !final_ty.is_first_class() {
            return Err(TypeError::RefEscape {
                ty: final_ty,
                context: format!("let binding '{}'", decl.name.text),
                span: decl.span,
            });
        }

        // Generalize: free vars in type that aren't already in environment become ∀-bound
        use super::infer::ty::free_vars_env;
        let env_vars = free_vars_env(&self.env);
        let env_eff_vars = free_effect_vars_env(&self.env);
        let scheme = self
            .infer_ctx
            .generalize(final_ty, &env_vars, &env_eff_vars);

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
        let predeclared_ty = self
            .infer_ctx
            .instantiate_scheme(&predeclared_scheme)
            .map_err(infer_error_to_type_error)?;

        // Extract param, return types, and declared effect row from the arrow
        let (param_tys, ret_ty, declared_eff) = match &predeclared_ty {
            Ty::Arrow(params, ret, eff) => (params.clone(), ret.as_ref().clone(), *eff),
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

        // Create a fresh effect var for the function body.
        // Call site constraints will accumulate effects into this var.
        let body_eff = self
            .infer_ctx
            .fresh_effect_var()
            .map_err(infer_error_to_type_error)?;

        // Create a CheckContext for the function body with ADT registry
        let mut fn_ctx =
            CheckContext::from_env_with_registry(self.env.clone(), self.adt_registry.clone());
        fn_ctx.expected_return = Some(ret_ty.clone());
        fn_ctx.body_effects = Some(body_eff);

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

        // Constrain body effects to fit within declared effect row.
        // body_eff ⊆ declared_eff ensures the function only uses effects it declares.
        self.infer_ctx
            .add_constraint(super::infer::ty::Constraint::EffectSubset(
                body_eff,
                declared_eff,
                decl.span,
            ));

        // Solve constraints
        let constraints = self.infer_ctx.take_constraints();
        let mut solver = Solver::new();
        let subst = solver
            .solve(constraints)
            .map_err(solve_error_to_type_error)?;

        // Apply substitution to get the final function type
        let final_fn_ty = subst
            .apply(&predeclared_ty)
            .map_err(|e| subst_error_to_type_error(e, decl.span))?;

        // ---- Settle point: reject &T in resolved return type ----
        // After solving, check that the return type doesn't contain Ty::Ref.
        // This catches cases where &T propagates through inference into a return.
        if let Ty::Arrow(_, ref resolved_ret, _) = final_fn_ty {
            if !resolved_ret.is_first_class() {
                return Err(TypeError::RefEscape {
                    ty: resolved_ret.as_ref().clone(),
                    context: format!("return type of '{}'", decl.name.text),
                    span: decl.ret_ty.as_ref().map(|t| t.span()).unwrap_or(decl.span),
                });
            }
        }

        // ---- Capability validation ----
        // After solving, check that the function has capability parameters for each
        // concrete effect in its resolved effect row. Every function with concrete
        // effects must have matching capabilities — no exceptions.
        if let Ty::Arrow(ref resolved_params, _, ref resolved_eff) = final_fn_ty {
            let param_caps: Vec<CapKind> = resolved_params
                .iter()
                .filter_map(|ty| match ty {
                    Ty::Cap(kind) => Some(*kind),
                    Ty::Ref(inner) => match inner.as_ref() {
                        Ty::Cap(kind) => Some(*kind),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();

            // Resolve the effect row through the substitution
            let resolved_eff = subst
                .apply_effect_row(resolved_eff)
                .map_err(|e| subst_error_to_type_error(e, decl.span))?;

            validate_caps_against_effects(
                &decl.name.text,
                &param_caps,
                &resolved_eff,
                decl.name.span,
                false,
            )?;
        }

        // ---- Move checker (affine/single-use enforcement) ----
        // After type checking succeeds, validate that affine bindings (capabilities)
        // are used at most once. This is a separate validation pass.
        if let Ty::Arrow(ref resolved_params, _, _) = final_fn_ty {
            let param_info: Vec<(String, Ty, Span)> = decl
                .params
                .iter()
                .zip(resolved_params.iter())
                .map(|(param, ty)| {
                    let resolved = subst
                        .apply(ty)
                        .map_err(|e| subst_error_to_type_error(e, param.name.span))?;
                    Ok((param.name.text.clone(), resolved, param.name.span))
                })
                .collect::<Result<Vec<_>, TypeError>>()?;

            crate::move_check::check_function_body(
                &param_info,
                &decl.body,
                &self.env,
                &self.adt_registry,
            )
            .map_err(move_error_to_type_error)?;
        }

        // NOW generalize: compute env vars excluding this function's own type vars
        // (since this function is still monomorphic in env, its vars are included in env_vars,
        // but we want to generalize those vars if they're not constrained by the environment)
        let mut env_for_generalize = self.env.clone();
        env_for_generalize.remove(&decl.name.text);
        let env_vars = free_vars_env(&env_for_generalize);
        let env_eff_vars = free_effect_vars_env(&env_for_generalize);
        let gen_scheme = self
            .infer_ctx
            .generalize(final_fn_ty, &env_vars, &env_eff_vars);

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
                self.ty_from_type_expr(ty_expr)?
            } else {
                // No annotation - create fresh type variable
                self.infer_ctx.fresh_var()
            };
            param_tys.push(param_ty);
        }

        // Reject &T in regular fn params
        for (param, param_ty) in decl.params.iter().zip(param_tys.iter()) {
            if matches!(param_ty, Ty::Ref(_)) {
                return Err(TypeError::NotImplemented {
                    msg: "Reference types (&T) are only allowed in extern function parameters"
                        .to_string(),
                    span: param.span,
                });
            }
        }

        // Extract return type
        let ret_ty = if let Some(ref ty_expr) = decl.ret_ty {
            self.ty_from_type_expr(ty_expr)?
        } else {
            self.infer_ctx.fresh_var()
        };

        // Reject &T in return type
        if contains_ref(&ret_ty) {
            return Err(TypeError::NotImplemented {
                msg: "Reference types (&T) are not allowed in return types".to_string(),
                span: decl.ret_ty.as_ref().map(|t| t.span()).unwrap_or(decl.span),
            });
        }

        // Convert effect annotation to EffectRow
        let eff = if let Some(ref effects) = decl.effects {
            self.resolve_effect_annotation(effects)?
        } else {
            // No annotation: use fresh effect var (inferred/open)
            self.infer_ctx
                .fresh_effect_var()
                .map_err(infer_error_to_type_error)?
        };

        Ok(Ty::arrow_eff(param_tys, ret_ty, eff))
    }

    /// Extract an extern function's type signature (no body).
    fn extract_extern_fn_signature(
        &mut self,
        decl: &strata_ast::ast::ExternFnDecl,
    ) -> Result<Ty, TypeError> {
        let mut param_tys = Vec::new();
        for param in &decl.params {
            let param_ty = if let Some(ref ty_expr) = param.ty {
                self.ty_from_type_expr(ty_expr)?
            } else {
                self.infer_ctx.fresh_var()
            };
            param_tys.push(param_ty);
        }

        let ret_ty = if let Some(ref ty_expr) = decl.ret_ty {
            self.ty_from_type_expr(ty_expr)?
        } else {
            self.infer_ctx.fresh_var()
        };

        // Reject &T in return type
        if contains_ref(&ret_ty) {
            return Err(TypeError::NotImplemented {
                msg: "Reference types (&T) are not allowed in return types".to_string(),
                span: decl.ret_ty.as_ref().map(|t| t.span()).unwrap_or(decl.span),
            });
        }

        // Validate: &T in extern params must only wrap capability types
        for (param, param_ty) in decl.params.iter().zip(param_tys.iter()) {
            if let Ty::Ref(inner) = param_ty {
                if !matches!(inner.as_ref(), Ty::Cap(_)) {
                    return Err(TypeError::NotImplemented {
                        msg: "Reference types (&T) are only allowed on capability types in extern function parameters".to_string(),
                        span: param.span,
                    });
                }
            }
        }

        // Convert effect annotation (required for extern fns)
        let eff = if let Some(ref effects) = decl.effects {
            self.resolve_effect_annotation(effects)?
        } else {
            return Err(TypeError::MissingExternEffects {
                fn_name: decl.name.text.clone(),
                span: decl.span,
            });
        };

        Ok(Ty::arrow_eff(param_tys, ret_ty, eff))
    }

    /// Resolve an effect name to an Effect enum variant.
    fn resolve_effect_name(name: &str, span: Span) -> Result<Effect, TypeError> {
        match name {
            "Fs" => Ok(Effect::Fs),
            "Net" => Ok(Effect::Net),
            "Time" => Ok(Effect::Time),
            "Rand" => Ok(Effect::Rand),
            "Ai" => Ok(Effect::Ai),
            _ => Err(TypeError::UnknownEffect {
                name: name.to_string(),
                span,
            }),
        }
    }

    /// Convert a list of AST effect identifiers to a closed EffectRow.
    fn resolve_effect_annotation(&self, effects: &[Ident]) -> Result<EffectRow, TypeError> {
        let mut row = EffectRow::pure();
        for ident in effects {
            let effect = Self::resolve_effect_name(&ident.text, ident.span)?;
            row.insert(effect);
        }
        Ok(row)
    }

    // ============ ADT Registration (Phase 3) ============

    /// Register a struct definition in the ADT registry.
    ///
    /// Validates:
    /// - No duplicate type definitions
    /// - All field types are valid
    /// - No capabilities stored in fields (until linear types)
    fn register_struct(&mut self, def: &StructDef) -> Result<(), TypeError> {
        // Check for reserved capability type names
        if CapKind::from_name(&def.name.text).is_some() {
            return Err(TypeError::ReservedCapabilityName {
                name: def.name.text.clone(),
                span: def.span,
            });
        }

        // Check for duplicate type definition
        if self.adt_registry.contains(&def.name.text) {
            return Err(TypeError::DuplicateType {
                name: def.name.text.clone(),
                span: def.span,
            });
        }

        // Create mapping from type param names to TypeVarIds
        let type_param_map: HashMap<String, TypeVarId> = def
            .type_params
            .iter()
            .enumerate()
            .map(|(i, param)| (param.text.clone(), TypeVarId(i as u32)))
            .collect();

        // Convert fields, checking for references and capabilities
        let mut fields = Vec::new();
        for field in &def.fields {
            let ty = self.ty_from_type_expr_with_params(&field.ty, &type_param_map)?;

            // Check for reference types in fields (check before capabilities since
            // &FsCap would match both — Ref is the more specific/relevant error)
            if contains_ref(&ty) {
                return Err(TypeError::RefInAdtField {
                    field: field.name.text.clone(),
                    ty,
                    span: field.span,
                });
            }

            // Check for capability types in fields
            if contains_capability(&ty) {
                // Find which capability type for better error message
                let cap_name = find_capability_name(&ty).unwrap_or("capability".to_string());
                return Err(TypeError::CapabilityInAdt {
                    field: field.name.text.clone(),
                    cap_type: cap_name,
                    span: field.span,
                });
            }

            fields.push(FieldDef {
                name: field.name.text.clone(),
                ty,
            });
        }

        // Create and register the ADT definition
        let type_params = def.type_params.iter().map(|p| p.text.clone()).collect();
        let adt_def = AdtDef::new_struct(&def.name.text, type_params, fields);
        self.adt_registry
            .register(adt_def)
            .map_err(|msg| TypeError::DuplicateType {
                name: msg,
                span: def.span,
            })
    }

    /// Register an enum definition in the ADT registry.
    ///
    /// Validates:
    /// - No duplicate type definitions
    /// - All variant types are valid
    /// - No capabilities stored in variant payloads (until linear types)
    fn register_enum(&mut self, def: &EnumDef) -> Result<(), TypeError> {
        use strata_ast::ast::VariantFields as AstVariantFields;

        // Check for reserved capability type names
        if CapKind::from_name(&def.name.text).is_some() {
            return Err(TypeError::ReservedCapabilityName {
                name: def.name.text.clone(),
                span: def.span,
            });
        }

        // Check for duplicate type definition
        if self.adt_registry.contains(&def.name.text) {
            return Err(TypeError::DuplicateType {
                name: def.name.text.clone(),
                span: def.span,
            });
        }

        // Create mapping from type param names to TypeVarIds
        let type_param_map: HashMap<String, TypeVarId> = def
            .type_params
            .iter()
            .enumerate()
            .map(|(i, param)| (param.text.clone(), TypeVarId(i as u32)))
            .collect();

        // Convert variants, checking for capabilities
        let mut variants = Vec::new();
        for variant in &def.variants {
            let variant_def = match &variant.fields {
                AstVariantFields::Unit => VariantDef::unit(&variant.name.text),
                AstVariantFields::Tuple(type_exprs) => {
                    let mut field_tys = Vec::new();
                    for (i, te) in type_exprs.iter().enumerate() {
                        let ty = self.ty_from_type_expr_with_params(te, &type_param_map)?;

                        // Check for reference types in variant payload (check before
                        // capabilities since &FsCap matches both)
                        if contains_ref(&ty) {
                            return Err(TypeError::RefInAdtField {
                                field: format!("{}::{}.{}", def.name.text, variant.name.text, i),
                                ty,
                                span: variant.span,
                            });
                        }

                        // Check for capability types in variant payload
                        if contains_capability(&ty) {
                            let cap_name =
                                find_capability_name(&ty).unwrap_or("capability".to_string());
                            return Err(TypeError::CapabilityInAdt {
                                field: format!("{}::{}.{}", def.name.text, variant.name.text, i),
                                cap_type: cap_name,
                                span: variant.span,
                            });
                        }
                        field_tys.push(ty);
                    }
                    VariantDef::tuple(&variant.name.text, field_tys)
                }
            };
            variants.push(variant_def);
        }

        // Create and register the ADT definition
        let type_params = def.type_params.iter().map(|p| p.text.clone()).collect();
        let adt_def = AdtDef::new_enum(&def.name.text, type_params, variants);
        self.adt_registry
            .register(adt_def)
            .map_err(|msg| TypeError::DuplicateType {
                name: msg,
                span: def.span,
            })
    }

    /// Register enum variant constructors as polymorphic functions in the environment.
    ///
    /// For `enum Option<T> { Some(T), None }`:
    /// - `Option::None : ∀T. () -> Option<T>`
    /// - `Option::Some : ∀T. (T) -> Option<T>`
    fn register_enum_constructors(&mut self, def: &EnumDef) -> Result<(), TypeError> {
        // Get the registered ADT to access the converted types
        let adt_def = self
            .adt_registry
            .get(&def.name.text)
            .cloned()
            .ok_or_else(|| TypeError::InvariantViolation {
                msg: format!("enum '{}' not registered", def.name.text),
                span: def.span,
            })?;

        // Allocate fresh type variables for the scheme using InferCtx.
        // IMPORTANT: We must use fresh vars from InferCtx to avoid collision with
        // vars that will be allocated later during type checking. The scheme's bound
        // vars and the fresh vars allocated during instantiation must be DIFFERENT.
        let type_vars: Vec<TypeVarId> = (0..adt_def.arity())
            .map(|_| self.infer_ctx.fresh_var_id())
            .collect();

        // The result type for all constructors: EnumName<T0, T1, ...>
        let result_ty = Ty::adt(
            &def.name.text,
            type_vars.iter().map(|v| Ty::Var(*v)).collect(),
        );

        // Build a substitution from the ADT's TypeVarIds (0, 1, 2...) to our fresh vars
        // The ADT registry stores field types using TypeVarId(0), TypeVarId(1), etc.
        // We need to remap those to our fresh vars.
        let var_remap: std::collections::HashMap<TypeVarId, Ty> = (0..adt_def.arity())
            .map(|i| (TypeVarId(i as u32), Ty::Var(type_vars[i])))
            .collect();

        // Get variants from the ADT def
        let variants = adt_def
            .variants()
            .ok_or_else(|| TypeError::InvariantViolation {
                msg: format!("'{}' is not an enum", def.name.text),
                span: def.span,
            })?;

        for variant in variants {
            // Build constructor function type, remapping the field types
            let fn_ty = match &variant.fields {
                VariantFields::Unit => {
                    // Unit variant is a value of type EnumName<T0, T1, ...>
                    // Not a constructor function - used directly without calling
                    result_ty.clone()
                }
                VariantFields::Tuple(field_tys) => {
                    // Tuple variant: (T0, T1, ...) -> EnumName<T0, T1, ...>
                    // Remap the field types from ADT's vars to our fresh vars
                    let remapped_fields: Vec<Ty> = field_tys
                        .iter()
                        .map(|ty| remap_type_vars(ty, &var_remap))
                        .collect();
                    Ty::arrow(remapped_fields, result_ty.clone())
                }
            };

            // Create polymorphic scheme with our fresh vars
            let scheme = Scheme {
                type_vars: type_vars.clone(),
                effect_vars: vec![],
                ty: fn_ty,
            };

            // Register with qualified name: EnumName::VariantName
            let qualified_name = format!("{}::{}", def.name.text, variant.name);
            self.env.insert(qualified_name, scheme);
        }

        Ok(())
    }
}

/// Remap type variables in a type according to a substitution map
fn remap_type_vars(ty: &Ty, remap: &std::collections::HashMap<TypeVarId, Ty>) -> Ty {
    match ty {
        Ty::Var(v) => remap.get(v).cloned().unwrap_or_else(|| ty.clone()),
        Ty::Const(_) | Ty::Never | Ty::Cap(_) => ty.clone(),
        Ty::Arrow(params, ret, eff) => Ty::arrow_eff(
            params.iter().map(|t| remap_type_vars(t, remap)).collect(),
            remap_type_vars(ret, remap),
            *eff,
        ),
        Ty::Tuple(tys) => Ty::Tuple(tys.iter().map(|t| remap_type_vars(t, remap)).collect()),
        Ty::List(t) => Ty::List(Box::new(remap_type_vars(t, remap))),
        Ty::Adt { name, args } => Ty::Adt {
            name: name.clone(),
            args: args.iter().map(|t| remap_type_vars(t, remap)).collect(),
        },
        Ty::Ref(inner) => Ty::Ref(Box::new(remap_type_vars(inner, remap))),
    }
}

impl TypeChecker {
    // ============ Type Expression Conversion ============

    /// Convert a TypeExpr to a Ty, using the ADT registry for user-defined types.
    pub fn ty_from_type_expr(&self, te: &TypeExpr) -> Result<Ty, TypeError> {
        self.ty_from_type_expr_with_params(te, &HashMap::new())
    }

    /// Convert a TypeExpr to a Ty, with a mapping from type parameter names to TypeVarIds.
    ///
    /// This is used during ADT registration where type params like `T` need to become
    /// type variables like `t0`.
    fn ty_from_type_expr_with_params(
        &self,
        te: &TypeExpr,
        type_params: &HashMap<String, TypeVarId>,
    ) -> Result<Ty, TypeError> {
        match te {
            TypeExpr::Path(path, span) => {
                // Single identifier: could be builtin, type param, or user ADT
                if path.len() == 1 {
                    let name = &path[0].text;

                    // Check for builtin types first
                    if let Some(ty) = self.builtin_type(name) {
                        return Ok(ty);
                    }

                    // Check for type parameter
                    if let Some(&var_id) = type_params.get(name.as_str()) {
                        return Ok(Ty::Var(var_id));
                    }

                    // Check for capability types (first-class in the type system)
                    if let Some(kind) = CapKind::from_name(name) {
                        return Ok(Ty::Cap(kind));
                    }

                    // Check for user-defined ADT (no type args)
                    if let Some(adt_def) = self.adt_registry.get(name) {
                        // ADT must have 0 type params if used without args
                        if adt_def.arity() > 0 {
                            return Err(TypeError::WrongTypeArgCount {
                                type_name: name.clone(),
                                expected: adt_def.arity(),
                                found: 0,
                                span: *span,
                            });
                        }
                        return Ok(Ty::adt0(name));
                    }

                    // Unknown type
                    Err(TypeError::UnknownType {
                        name: name.clone(),
                        span: *span,
                    })
                } else {
                    // Qualified path (e.g., module::Type) - not yet supported
                    let full_name = path
                        .iter()
                        .map(|i| i.text.as_str())
                        .collect::<Vec<_>>()
                        .join("::");
                    Err(TypeError::NotImplemented {
                        msg: format!("Qualified type paths not yet supported: {}", full_name),
                        span: *span,
                    })
                }
            }
            TypeExpr::Arrow { params, ret, .. } => {
                let param_tys: Result<Vec<Ty>, TypeError> = params
                    .iter()
                    .map(|p| self.ty_from_type_expr_with_params(p, type_params))
                    .collect();
                let param_tys = param_tys?;
                let ret_ty = self.ty_from_type_expr_with_params(ret, type_params)?;
                Ok(Ty::arrow(param_tys, ret_ty))
            }
            TypeExpr::App { base, args, span } => {
                // Generic type application: Option<Int>, Result<T, E>
                let name = base
                    .iter()
                    .map(|i| i.text.as_str())
                    .collect::<Vec<_>>()
                    .join("::");

                // Look up the ADT
                let adt_def =
                    self.adt_registry
                        .get(&name)
                        .ok_or_else(|| TypeError::UnknownType {
                            name: name.clone(),
                            span: *span,
                        })?;

                // Check arity
                if adt_def.arity() != args.len() {
                    return Err(TypeError::WrongTypeArgCount {
                        type_name: name,
                        expected: adt_def.arity(),
                        found: args.len(),
                        span: *span,
                    });
                }

                // Convert type arguments
                let type_args: Result<Vec<Ty>, TypeError> = args
                    .iter()
                    .map(|a| self.ty_from_type_expr_with_params(a, type_params))
                    .collect();

                Ok(Ty::adt(name, type_args?))
            }
            TypeExpr::Tuple(elems, _span) => {
                if elems.is_empty() {
                    // Empty tuple is Unit
                    return Ok(Ty::unit());
                }
                if elems.len() == 1 {
                    // Single-element "tuple" is just the element type
                    return self.ty_from_type_expr_with_params(&elems[0], type_params);
                }

                // Multi-element tuple
                let elem_tys: Result<Vec<Ty>, TypeError> = elems
                    .iter()
                    .map(|e| self.ty_from_type_expr_with_params(e, type_params))
                    .collect();

                Ok(Ty::tuple(elem_tys?))
            }
            TypeExpr::Ref(inner, _span) => {
                let inner_ty = self.ty_from_type_expr_with_params(inner, type_params)?;
                Ok(Ty::Ref(Box::new(inner_ty)))
            }
        }
    }

    /// Return builtin type for a name, or None if not a builtin
    fn builtin_type(&self, name: &str) -> Option<Ty> {
        match name {
            "Unit" => Some(Ty::unit()),
            "Bool" => Some(Ty::bool_()),
            "Int" => Some(Ty::int()),
            "Float" => Some(Ty::float()),
            "String" => Some(Ty::string()),
            _ => None,
        }
    }
}

/// Convert InferError to TypeError
fn infer_error_to_type_error(err: super::infer::constraint::InferError) -> TypeError {
    use super::infer::constraint::InferError;
    match err {
        InferError::UnknownVariable { name, span } => TypeError::UnknownVariable { name, span },
        InferError::ImmutableAssignment { name, span } => {
            TypeError::ImmutableAssignment { name, span }
        }
        InferError::NotImplemented { msg, span } => TypeError::NotImplemented { msg, span },
        InferError::DepthLimitExceeded { span } => TypeError::DepthLimitExceeded { span },
        InferError::DuplicateBinding { name, span } => {
            TypeError::DuplicateField { field: name, span }
        }
        InferError::UnknownType { name, span } => TypeError::UnknownType { name, span },
        InferError::UnknownVariant {
            type_name,
            variant,
            span,
        } => TypeError::UnknownVariant {
            type_name,
            variant,
            span,
        },
        InferError::PatternArityMismatch {
            expected,
            found,
            span,
        } => TypeError::ArityMismatch {
            expected,
            found,
            span,
        },
        InferError::MissingField {
            struct_name,
            field,
            span,
        } => TypeError::MissingField {
            struct_name,
            field,
            span,
        },
        InferError::UnknownField {
            struct_name,
            field,
            span,
        } => TypeError::UnknownField {
            struct_name,
            field,
            span,
        },
        InferError::DuplicateField { field, span } => TypeError::DuplicateField { field, span },
        InferError::TupleArityLimit { max, found, span } => TypeError::ArityMismatch {
            expected: max,
            found,
            span,
        },
        InferError::NonExhaustiveMatch { witness, span } => {
            TypeError::NonExhaustiveMatch { witness, span }
        }
        InferError::UnreachablePattern { arm_index, span } => {
            TypeError::UnreachablePattern { arm_index, span }
        }
        InferError::ExhaustivenessLimitExceeded { msg, span } => {
            TypeError::ExhaustivenessLimitExceeded { msg, span }
        }
        InferError::RefutablePattern { pat_desc, span } => {
            TypeError::RefutablePattern { pat_desc, span }
        }
        InferError::EffectVarLimitExceeded { limit } => TypeError::EffectVarLimitExceeded { limit },
        InferError::EffectCycle { var } => TypeError::EffectCycle {
            var,
            span: Span { start: 0, end: 0 },
        },
        InferError::EffectChainTooDeep { depth } => TypeError::EffectChainTooDeep {
            depth,
            span: Span { start: 0, end: 0 },
        },
        InferError::InstantiationArityMismatch {
            expected_types,
            got_types,
            ..
        } => TypeError::ArityMismatch {
            expected: expected_types,
            found: got_types,
            span: Span { start: 0, end: 0 },
        },
        InferError::RefEscape { ty, context, span } => TypeError::RefEscape { ty, context, span },
    }
}

/// Convert a SolveError (which includes span) to checker TypeError
fn solve_error_to_type_error(err: super::infer::solver::SolveError) -> TypeError {
    let span = err.span;
    unifier_error_to_type_error(err.error, span)
}

/// Convert a unifier TypeError to a checker TypeError with a span
fn unifier_error_to_type_error(err: super::infer::unifier::TypeError, span: Span) -> TypeError {
    match err {
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
        super::infer::unifier::TypeError::EffectMismatch { expected, found } => {
            TypeError::EffectMismatch {
                expected,
                found,
                span,
            }
        }
        super::infer::unifier::TypeError::EffectCycle { var } => {
            TypeError::EffectCycle { var, span }
        }
        super::infer::unifier::TypeError::EffectChainTooDeep { depth } => {
            TypeError::EffectChainTooDeep { depth, span }
        }
    }
}

/// Convert a MoveError to a TypeError
fn move_error_to_type_error(err: crate::move_check::MoveError) -> TypeError {
    use crate::move_check::MoveError;
    match err {
        MoveError::AlreadyUsed {
            name,
            used_at,
            previous_use,
        } => TypeError::CapabilityAlreadyUsed {
            name,
            used_at,
            previous_use,
        },
        MoveError::UsedInLoop { name, used_at } => {
            TypeError::CapabilityUsedInLoop { name, used_at }
        }
    }
}

/// Validate that a function's capability parameters cover all concrete effects.
///
/// For each concrete effect in the effect row, checks that a matching capability
/// type is present in the parameter list. Open tail variables (representing
/// parametric effects from higher-order calls) are NOT checked.
///
/// Used by both `check_fn` and extern validation to prevent drift.
fn validate_caps_against_effects(
    fn_name: &str,
    param_caps: &[CapKind],
    effect_row: &EffectRow,
    span: Span,
    is_extern: bool,
) -> Result<(), TypeError> {
    for effect in effect_row.iter() {
        let required_cap = CapKind::from_effect(effect);
        if !param_caps.contains(&required_cap) {
            if is_extern {
                return Err(TypeError::ExternMissingCapability {
                    effect: format!("{:?}", effect),
                    cap_type: required_cap.type_name().to_string(),
                    fn_name: fn_name.to_string(),
                    span,
                });
            } else {
                return Err(TypeError::MissingCapability {
                    effect: format!("{:?}", effect),
                    cap_type: required_cap.type_name().to_string(),
                    fn_name: fn_name.to_string(),
                    span,
                });
            }
        }
    }
    Ok(())
}

/// Check if a type contains any Ty::Ref (reference) anywhere in its structure.
fn contains_ref(ty: &Ty) -> bool {
    match ty {
        Ty::Ref(_) => true,
        Ty::Const(_) | Ty::Var(_) | Ty::Never | Ty::Cap(_) => false,
        Ty::Arrow(params, ret, _) => params.iter().any(contains_ref) || contains_ref(ret),
        Ty::Tuple(tys) => tys.iter().any(contains_ref),
        Ty::List(inner) => contains_ref(inner),
        Ty::Adt { args, .. } => args.iter().any(contains_ref),
    }
}

/// Convert a SubstError to a checker TypeError with a span
fn subst_error_to_type_error(err: super::infer::subst::SubstError, span: Span) -> TypeError {
    use super::infer::subst::SubstError;
    match err {
        SubstError::EffectCycle { var } => TypeError::EffectCycle { var, span },
        SubstError::EffectChainTooDeep { depth } => TypeError::EffectChainTooDeep { depth, span },
        SubstError::InstantiationArityMismatch {
            expected_types,
            got_types,
            ..
        } => TypeError::ArityMismatch {
            expected: expected_types,
            found: got_types,
            span,
        },
    }
}
