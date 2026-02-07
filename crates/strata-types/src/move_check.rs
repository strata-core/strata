//! Move checker for affine (single-use) capability types.
//!
//! This is a post-inference validation pass that ensures capabilities are used
//! at most once. It operates on a fully-typed, fully-solved AST.
//!
//! The move checker does NOT modify unification, constraint solving, or type
//! inference. It queries `Ty::kind()` on resolved types to determine which
//! bindings need single-use tracking, and resolves polymorphic return types
//! by manually instantiating callee schemes with known argument types.

use crate::adt::AdtRegistry;
use crate::infer::ty::{Kind, Scheme, Ty, TypeVarId};
use std::collections::HashMap;
use strata_ast::ast::{Block, Expr, Pat, Stmt};
use strata_ast::span::Span;

// ---------------------------------------------------------------------------
// Error types — permission/authority vocabulary, NOT Rust "moved value" language
// ---------------------------------------------------------------------------

/// Move-checking errors use permission/authority vocabulary.
#[derive(Debug, Clone)]
pub enum MoveError {
    /// Capability has already been used (double use).
    AlreadyUsed {
        name: String,
        used_at: Span,
        previous_use: Span,
    },
    /// Capability used inside a loop (would be used multiple times).
    UsedInLoop { name: String, used_at: Span },
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveError::AlreadyUsed {
                name,
                used_at,
                previous_use,
            } => write!(
                f,
                "capability '{}' has already been used at {:?}; \
                 permission was transferred at {:?}; \
                 '{}' is no longer available",
                name, used_at, previous_use, name
            ),
            MoveError::UsedInLoop { name, used_at } => write!(
                f,
                "cannot use single-use capability '{}' inside loop at {:?}; \
                 '{}' would be used on every iteration",
                name, used_at, name
            ),
        }
    }
}

impl std::error::Error for MoveError {}

// ---------------------------------------------------------------------------
// Move state tracking
// ---------------------------------------------------------------------------

/// Unique identifier for a binding, combining name with a generation counter
/// to handle shadowing correctly.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct BindingId {
    name: String,
    generation: u32,
}

/// Tracks whether a single-use binding is still available.
#[derive(Clone, Debug)]
enum MoveState {
    Alive,
    Consumed(Span),
}

/// Information about a tracked (affine) binding.
#[derive(Clone, Debug)]
struct TrackedBinding {
    state: MoveState,
    def_span: Span,
}

// ---------------------------------------------------------------------------
// Move checker
// ---------------------------------------------------------------------------

/// The move checker validates single-use semantics for affine types.
pub struct MoveChecker<'a> {
    /// Maps binding names to their current BindingId (handles shadowing).
    name_to_id: HashMap<String, BindingId>,
    /// Maps BindingId to move state for all tracked (affine) bindings.
    tracked: HashMap<BindingId, TrackedBinding>,
    /// Generation counter for creating unique BindingIds.
    generation: u32,
    /// Whether we're currently inside a loop body.
    in_loop: bool,
    /// Collected errors.
    errors: Vec<MoveError>,
    /// Maps BindingId to resolved types (keyed by generation, not just name,
    /// so shadowing cannot corrupt type lookups).
    binding_types: HashMap<BindingId, Ty>,
    /// The environment with generalized function schemes (for resolving
    /// polymorphic call return types).
    env: &'a HashMap<String, Scheme>,
    /// ADT registry for resolving generic field types in pattern destructuring.
    adt_registry: &'a AdtRegistry,
}

impl<'a> MoveChecker<'a> {
    fn new(env: &'a HashMap<String, Scheme>, adt_registry: &'a AdtRegistry) -> Self {
        MoveChecker {
            name_to_id: HashMap::new(),
            tracked: HashMap::new(),
            generation: 0,
            in_loop: false,
            errors: Vec::new(),
            binding_types: HashMap::new(),
            env,
            adt_registry,
        }
    }

    /// Introduce a new binding. If its type is affine, start tracking it.
    fn introduce_binding(&mut self, name: &str, ty: &Ty, span: Span) {
        self.generation += 1;
        let id = BindingId {
            name: name.to_string(),
            generation: self.generation,
        };
        self.name_to_id.insert(name.to_string(), id.clone());
        self.binding_types.insert(id.clone(), ty.clone());

        if ty.kind() == Kind::Affine {
            self.tracked.insert(
                id,
                TrackedBinding {
                    state: MoveState::Alive,
                    def_span: span,
                },
            );
        }
    }

    /// Look up the type of a binding by name (resolves through current generation).
    fn get_binding_type(&self, name: &str) -> Option<&Ty> {
        let id = self.name_to_id.get(name)?;
        self.binding_types.get(id)
    }

    /// Check if a binding is tracked as affine.
    fn is_affine(&self, name: &str) -> bool {
        if let Some(id) = self.name_to_id.get(name) {
            self.tracked.contains_key(id)
        } else {
            false
        }
    }

    /// Use (consume) an affine binding. Emits error if already consumed or in a loop.
    fn use_binding(&mut self, name: &str, use_span: Span) {
        let id = match self.name_to_id.get(name) {
            Some(id) => id.clone(),
            None => return,
        };

        let tracked = match self.tracked.get(&id) {
            Some(t) => t.clone(),
            None => return,
        };

        if self.in_loop {
            self.errors.push(MoveError::UsedInLoop {
                name: name.to_string(),
                used_at: use_span,
            });
            return;
        }

        match tracked.state {
            MoveState::Alive => {
                self.tracked.insert(
                    id,
                    TrackedBinding {
                        state: MoveState::Consumed(use_span),
                        def_span: tracked.def_span,
                    },
                );
            }
            MoveState::Consumed(previous_span) => {
                self.errors.push(MoveError::AlreadyUsed {
                    name: name.to_string(),
                    used_at: use_span,
                    previous_use: previous_span,
                });
            }
        }
    }

    /// Snapshot the current move states for branching.
    fn snapshot(&self) -> HashMap<BindingId, TrackedBinding> {
        self.tracked.clone()
    }

    /// Restore move states from a snapshot.
    fn restore(&mut self, snapshot: HashMap<BindingId, TrackedBinding>) {
        self.tracked = snapshot;
    }

    /// Pessimistic join: if consumed in ANY branch, mark as consumed.
    fn pessimistic_join(
        &mut self,
        base: &HashMap<BindingId, TrackedBinding>,
        branches: &[HashMap<BindingId, TrackedBinding>],
    ) {
        for (id, base_tracked) in base {
            let mut consumed_span: Option<Span> = None;

            for branch in branches {
                if let Some(branch_tracked) = branch.get(id) {
                    if let MoveState::Consumed(span) = &branch_tracked.state {
                        consumed_span = Some(*span);
                    }
                }
            }

            if let Some(span) = consumed_span {
                self.tracked.insert(
                    id.clone(),
                    TrackedBinding {
                        state: MoveState::Consumed(span),
                        def_span: base_tracked.def_span,
                    },
                );
            } else {
                self.tracked.insert(id.clone(), base_tracked.clone());
            }
        }
    }

    // -----------------------------------------------------------------------
    // ADT field type resolution for pattern destructuring
    // -----------------------------------------------------------------------

    /// Resolve variant field types by matching the scrutinee type against the
    /// ADT definition and substituting generic type parameters.
    ///
    /// For example, if `Box<T>` has variant `Val(T)` and the scrutinee is
    /// `Box<FsCap>`, this returns `[FsCap]` for the variant fields.
    ///
    /// Returns None if the ADT or variant can't be resolved (fallback to unit).
    fn resolve_variant_field_types(
        &self,
        path: &strata_ast::ast::Path,
        scrutinee_ty: &Ty,
    ) -> Option<Vec<Ty>> {
        // Path for a variant is e.g. ["Box", "Val"] — first segment is ADT name
        if path.segments.len() < 2 {
            return None;
        }
        let adt_name = &path.segments[0].text;
        let variant_name = &path.segments[1].text;

        let adt_def = self.adt_registry.get(adt_name)?;
        let variant = adt_def.find_variant(variant_name)?;

        let raw_field_types = match &variant.fields {
            crate::adt::VariantFields::Unit => return Some(vec![]),
            crate::adt::VariantFields::Tuple(tys) => tys,
        };

        // Build substitution: type_params[i] → scrutinee_args[i]
        let scrutinee_args = match scrutinee_ty {
            Ty::Adt { args, .. } => args,
            _ => return None,
        };

        if adt_def.type_params.is_empty() || scrutinee_args.is_empty() {
            // Non-generic ADT — raw field types are already concrete
            return Some(raw_field_types.clone());
        }

        // Map type param TypeVarIds to the concrete types from the scrutinee.
        // ADT type params are registered with TypeVarId(0), TypeVarId(1), etc.
        let mut mapping: HashMap<TypeVarId, Ty> = HashMap::new();
        for (i, arg) in scrutinee_args.iter().enumerate() {
            mapping.insert(TypeVarId(i as u32), arg.clone());
        }

        Some(
            raw_field_types
                .iter()
                .map(|t| apply_type_mapping(t, &mapping))
                .collect(),
        )
    }

    /// Resolve struct field types by matching the scrutinee type against the
    /// ADT definition and substituting generic type parameters.
    ///
    /// Returns a map from field name to resolved type, or None if unresolvable.
    fn resolve_struct_field_types(
        &self,
        path: &strata_ast::ast::Path,
        scrutinee_ty: &Ty,
    ) -> Option<HashMap<String, Ty>> {
        let adt_name = &path.segments.first()?.text;

        let adt_def = self.adt_registry.get(adt_name)?;
        let fields = adt_def.fields()?;

        // Build substitution from scrutinee args
        let scrutinee_args = match scrutinee_ty {
            Ty::Adt { args, .. } => args,
            _ => return None,
        };

        if adt_def.type_params.is_empty() || scrutinee_args.is_empty() {
            // Non-generic struct — raw field types are already concrete
            return Some(
                fields
                    .iter()
                    .map(|f| (f.name.clone(), f.ty.clone()))
                    .collect(),
            );
        }

        let mut mapping: HashMap<TypeVarId, Ty> = HashMap::new();
        for (i, arg) in scrutinee_args.iter().enumerate() {
            mapping.insert(TypeVarId(i as u32), arg.clone());
        }

        Some(
            fields
                .iter()
                .map(|f| (f.name.clone(), apply_type_mapping(&f.ty, &mapping)))
                .collect(),
        )
    }

    // -----------------------------------------------------------------------
    // Type resolution for function call return types
    // -----------------------------------------------------------------------

    /// Determine the resolved type of an expression.
    ///
    /// This is used to track affinity through let bindings and function calls.
    /// For variable references, returns the binding's known type.
    /// For function calls, instantiates the callee's scheme with argument types.
    fn resolve_expr_type(&self, expr: &Expr) -> Ty {
        match expr {
            Expr::Var(ident) => self
                .get_binding_type(&ident.text)
                .cloned()
                .unwrap_or_else(Ty::unit),

            Expr::Paren { inner, .. } => self.resolve_expr_type(inner),

            Expr::PathExpr(path) => {
                if path.segments.len() == 1 {
                    let name = &path.segments[0].text;
                    if let Some(ty) = self.get_binding_type(name) {
                        return ty.clone();
                    }
                }
                Ty::unit()
            }

            Expr::Call { callee, args, .. } => self.resolve_call_return_type(callee, args),

            Expr::If { then_, else_, .. } => {
                // If/else: resolve from then-branch (branches should have same type)
                if let Some(ref tail) = then_.tail {
                    return self.resolve_expr_type(tail);
                }
                if let Some(else_expr) = else_ {
                    return self.resolve_expr_type(else_expr);
                }
                Ty::unit()
            }

            Expr::Block(block) => {
                if let Some(ref tail) = block.tail {
                    return self.resolve_expr_type(tail);
                }
                Ty::unit()
            }

            Expr::Match { arms, .. } => {
                // Resolve from first arm body
                if let Some(arm) = arms.first() {
                    return self.resolve_expr_type(&arm.body);
                }
                Ty::unit()
            }

            Expr::Tuple { elems, .. } => {
                Ty::Tuple(elems.iter().map(|e| self.resolve_expr_type(e)).collect())
            }

            Expr::Borrow(inner, _) => Ty::Ref(Box::new(self.resolve_expr_type(inner))),

            // Literals, binary, unary, struct exprs, etc. are always unrestricted
            _ => Ty::unit(),
        }
    }

    /// Resolve the return type of a function call.
    ///
    /// For polymorphic callees, instantiates the scheme with argument types.
    fn resolve_call_return_type(&self, callee: &Expr, args: &[Expr]) -> Ty {
        let callee_name = match callee {
            Expr::Var(ident) => ident.text.clone(),
            Expr::PathExpr(path) => path
                .segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("::"),
            _ => return Ty::unit(),
        };

        // Check local bindings first (e.g., function parameters with arrow type)
        if let Some(ty) = self.get_binding_type(&callee_name) {
            if let Ty::Arrow(_, ref ret, _) = ty {
                return ret.as_ref().clone();
            }
            return Ty::unit();
        }

        // Check env (named functions)
        if let Some(scheme) = self.env.get(&callee_name) {
            return self.instantiate_return_type(scheme, args);
        }

        Ty::unit()
    }

    /// Instantiate a function scheme's return type using known argument types.
    fn instantiate_return_type(&self, scheme: &Scheme, args: &[Expr]) -> Ty {
        let (params, ret) = match &scheme.ty {
            Ty::Arrow(params, ret, _) => (params, ret.as_ref()),
            other => return other.clone(), // Not an arrow (e.g., unit constructor)
        };

        if scheme.type_vars.is_empty() {
            // Monomorphic — return type is directly known
            return ret.clone();
        }

        // Polymorphic: build mapping from scheme type vars to argument types
        let arg_types: Vec<Ty> = args.iter().map(|a| self.resolve_expr_type(a)).collect();

        let mut mapping: HashMap<TypeVarId, Ty> = HashMap::new();
        for (param_ty, arg_ty) in params.iter().zip(arg_types.iter()) {
            collect_var_mapping(param_ty, arg_ty, &scheme.type_vars, &mut mapping);
        }

        apply_type_mapping(ret, &mapping)
    }

    // -----------------------------------------------------------------------
    // Expression walking
    // -----------------------------------------------------------------------

    /// Check an expression for move violations.
    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Lit(_, _) => {}

            Expr::Var(ident) => {
                if self.is_affine(&ident.text) {
                    self.use_binding(&ident.text, ident.span);
                }
            }

            Expr::Paren { inner, .. } => {
                self.check_expr(inner);
            }

            Expr::Unary { expr: inner, .. } => {
                self.check_expr(inner);
            }

            Expr::Binary { lhs, rhs, .. } => {
                self.check_expr(lhs);
                self.check_expr(rhs);
            }

            Expr::Call { callee, args, .. } => {
                // Walk callee (usually a variable name — won't be affine)
                self.check_expr(callee);

                // Walk arguments left-to-right (evaluation order matters!)
                // Consumption in one argument is visible to subsequent arguments.
                for arg in args {
                    self.check_expr(arg);
                }
            }

            Expr::Block(block) => {
                self.check_block(block);
            }

            Expr::If {
                cond, then_, else_, ..
            } => {
                // Condition is always unrestricted (Bool)
                self.check_expr(cond);

                // Save state before branches
                let base = self.snapshot();

                // Check then-branch
                self.check_block(then_);
                let then_state = self.snapshot();

                // Restore and check else-branch
                self.restore(base.clone());
                if let Some(else_expr) = else_ {
                    self.check_expr(else_expr);
                }
                let else_state = self.snapshot();

                // Pessimistic join
                self.restore(base.clone());
                self.pessimistic_join(&base, &[then_state, else_state]);
            }

            Expr::While { cond, body, .. } => {
                self.check_expr(cond);

                let was_in_loop = self.in_loop;
                self.in_loop = true;
                self.check_block(body);
                self.in_loop = was_in_loop;
            }

            Expr::Match {
                scrutinee, arms, ..
            } => {
                // Resolve scrutinee type BEFORE checking (so we can type pattern bindings)
                let scrut_ty = self.resolve_expr_type(scrutinee);

                // Check scrutinee (may consume an affine binding)
                self.check_expr(scrutinee);

                let base = self.snapshot();
                let mut arm_states = Vec::new();

                for arm in arms {
                    self.restore(base.clone());

                    // Introduce pattern bindings with the scrutinee's type
                    // so that capability bindings are correctly tracked as affine.
                    self.introduce_pattern_bindings(&arm.pat, &scrut_ty);

                    self.check_expr(&arm.body);
                    arm_states.push(self.snapshot());
                }

                self.restore(base.clone());
                if !arm_states.is_empty() {
                    self.pessimistic_join(&base, &arm_states);
                }
            }

            Expr::Tuple { elems, .. } => {
                for elem in elems {
                    self.check_expr(elem);
                }
            }

            Expr::StructExpr { fields, .. } => {
                for field in fields {
                    self.check_expr(&field.value);
                }
            }

            Expr::PathExpr(path) => {
                if path.segments.len() == 1 {
                    let name = &path.segments[0].text;
                    if self.is_affine(name) {
                        self.use_binding(name, path.span);
                    }
                }
            }

            Expr::Borrow(inner, span) => {
                // Borrow checks that the inner var is alive but does NOT consume it.
                // No loop restriction — borrows are repeatable.
                if let Expr::Var(ident) = inner.as_ref() {
                    if let Some(id) = self.name_to_id.get(&ident.text) {
                        if let Some(tracked) = self.tracked.get(id) {
                            if let MoveState::Consumed(previous_span) = &tracked.state {
                                self.errors.push(MoveError::AlreadyUsed {
                                    name: ident.text.clone(),
                                    used_at: *span,
                                    previous_use: *previous_span,
                                });
                            }
                            // No state change — borrow doesn't consume
                        }
                    }
                } else {
                    // Complex borrow expression — recurse
                    self.check_expr(inner);
                }
            }
        }
    }

    /// Check a block for move violations.
    fn check_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }

        if let Some(ref tail) = block.tail {
            self.check_expr(tail);
        }
    }

    /// Check a statement for move violations.
    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                pat, value, span, ..
            } => {
                // Determine the resolved type of the RHS
                let rhs_ty = self.resolve_expr_type(value);

                // Check the RHS expression (may consume affine bindings)
                self.check_expr(value);

                // Introduce bindings from the pattern
                self.introduce_pattern_bindings(pat, &rhs_ty);

                let _ = span;
            }

            Stmt::Assign { target, value, .. } => {
                let rhs_ty = self.resolve_expr_type(value);
                self.check_expr(value);

                // If the new value is affine, re-introduce the target as alive
                if rhs_ty.kind() == Kind::Affine {
                    self.introduce_binding(&target.text, &rhs_ty, target.span);
                }
            }

            Stmt::Expr { expr, .. } => {
                self.check_expr(expr);
            }

            Stmt::Return { value, .. } => {
                if let Some(val_expr) = value {
                    self.check_expr(val_expr);
                }
            }
        }
    }

    /// Introduce bindings from a pattern.
    fn introduce_pattern_bindings(&mut self, pat: &Pat, ty: &Ty) {
        match pat {
            Pat::Ident(ident) => {
                self.introduce_binding(&ident.text, ty, ident.span);
            }
            Pat::Wildcard(_) | Pat::Literal(_, _) => {}
            Pat::Tuple(pats, _) => {
                if let Ty::Tuple(tys) = ty {
                    for (p, t) in pats.iter().zip(tys.iter()) {
                        self.introduce_pattern_bindings(p, t);
                    }
                } else {
                    for p in pats {
                        self.introduce_pattern_bindings(p, &Ty::unit());
                    }
                }
            }
            Pat::Variant { path, fields, .. } => {
                // DEFENSE-IN-DEPTH: When the caps-in-ADTs ban is lifted (post-linear
                // types), this substitution ensures generic variant fields are properly
                // resolved so capability bindings are tracked as affine.
                // Currently the ban prevents caps from reaching ADT fields, so all
                // field types resolve to unrestricted. But when the ban is lifted,
                // e.g. Box<FsCap> matched as Box::Val(inner), inner must be FsCap.
                let field_types = self.resolve_variant_field_types(path, ty);
                let unit = Ty::unit();
                for (i, p) in fields.iter().enumerate() {
                    let field_ty = field_types
                        .as_ref()
                        .and_then(|tys| tys.get(i))
                        .unwrap_or(&unit);
                    self.introduce_pattern_bindings(p, field_ty);
                }
            }
            Pat::Struct { path, fields, .. } => {
                // DEFENSE-IN-DEPTH: Same as variant — resolve struct field types
                // through the generic substitution when possible.
                let field_types = self.resolve_struct_field_types(path, ty);
                let unit = Ty::unit();
                for f in fields {
                    let field_ty = field_types
                        .as_ref()
                        .and_then(|m| m.get(&f.name.text))
                        .unwrap_or(&unit);
                    self.introduce_pattern_bindings(&f.pat, field_ty);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Type mapping helpers (for polymorphic instantiation)
// ---------------------------------------------------------------------------

/// Collect variable-to-type mappings by matching parameter types to argument types.
fn collect_var_mapping(
    param: &Ty,
    arg: &Ty,
    bound_vars: &[TypeVarId],
    mapping: &mut HashMap<TypeVarId, Ty>,
) {
    match param {
        Ty::Var(v) if bound_vars.contains(v) => {
            mapping.entry(*v).or_insert_with(|| arg.clone());
        }
        Ty::Arrow(params, ret, _) => {
            if let Ty::Arrow(arg_params, arg_ret, _) = arg {
                for (p, a) in params.iter().zip(arg_params.iter()) {
                    collect_var_mapping(p, a, bound_vars, mapping);
                }
                collect_var_mapping(ret, arg_ret, bound_vars, mapping);
            }
        }
        Ty::Tuple(tys) => {
            if let Ty::Tuple(arg_tys) = arg {
                for (p, a) in tys.iter().zip(arg_tys.iter()) {
                    collect_var_mapping(p, a, bound_vars, mapping);
                }
            }
        }
        Ty::List(inner) => {
            if let Ty::List(arg_inner) = arg {
                collect_var_mapping(inner, arg_inner, bound_vars, mapping);
            }
        }
        Ty::Adt { args, .. } => {
            if let Ty::Adt { args: arg_args, .. } = arg {
                for (p, a) in args.iter().zip(arg_args.iter()) {
                    collect_var_mapping(p, a, bound_vars, mapping);
                }
            }
        }
        Ty::Ref(inner) => {
            if let Ty::Ref(arg_inner) = arg {
                collect_var_mapping(inner, arg_inner, bound_vars, mapping);
            }
        }
        _ => {}
    }
}

/// Apply a type variable mapping to a type.
fn apply_type_mapping(ty: &Ty, mapping: &HashMap<TypeVarId, Ty>) -> Ty {
    if mapping.is_empty() {
        return ty.clone();
    }
    match ty {
        Ty::Var(v) => mapping.get(v).cloned().unwrap_or_else(|| ty.clone()),
        Ty::Const(_) | Ty::Never | Ty::Cap(_) => ty.clone(),
        Ty::Arrow(params, ret, eff) => Ty::arrow_eff(
            params
                .iter()
                .map(|t| apply_type_mapping(t, mapping))
                .collect(),
            apply_type_mapping(ret, mapping),
            *eff,
        ),
        Ty::Tuple(tys) => Ty::Tuple(tys.iter().map(|t| apply_type_mapping(t, mapping)).collect()),
        Ty::List(t) => Ty::List(Box::new(apply_type_mapping(t, mapping))),
        Ty::Adt { name, args } => Ty::Adt {
            name: name.clone(),
            args: args
                .iter()
                .map(|t| apply_type_mapping(t, mapping))
                .collect(),
        },
        Ty::Ref(inner) => Ty::Ref(Box::new(apply_type_mapping(inner, mapping))),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Move-check a function body given resolved parameter types.
///
/// `params` is a list of (name, resolved_type, span) triples for parameters.
/// `body` is the function body block.
/// `env` is the type environment with generalized function schemes.
///
/// Returns the first error found, or Ok(()).
pub fn check_function_body(
    params: &[(String, Ty, Span)],
    body: &Block,
    env: &HashMap<String, Scheme>,
    adt_registry: &AdtRegistry,
) -> Result<(), MoveError> {
    let mut checker = MoveChecker::new(env, adt_registry);

    // Introduce function parameters as alive bindings
    for (name, ty, span) in params {
        checker.introduce_binding(name, ty, *span);
    }

    // Check the body
    checker.check_block(body);

    // Return first error
    if let Some(err) = checker.errors.into_iter().next() {
        Err(err)
    } else {
        Ok(())
    }
}
