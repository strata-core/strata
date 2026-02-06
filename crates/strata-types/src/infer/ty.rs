use std::fmt;
use std::hash::{Hash, Hasher};
use strata_ast::span::Span;

use crate::effects::{CapKind, EffectRow, EffectVarId};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeVarId(pub u32);

impl fmt::Debug for TypeVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

impl fmt::Display for TypeVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}
impl Hash for TypeVarId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum TyConst {
    Unit,
    Bool,
    Int,
    Float,
    String,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ty {
    Var(TypeVarId),
    Const(TyConst),
    Arrow(Vec<Ty>, Box<Ty>, EffectRow),
    /// Heterogeneous, fixed-length tuple: (t0, t1, ..., tn)
    /// Note: Tuples are nominal and desugar to Tuple2<A,B> etc. in type checking.
    Tuple(Vec<Ty>),
    /// Homogeneous list: [elem]
    List(Box<Ty>),
    /// Algebraic data type (struct or enum) with type arguments
    /// Examples: Option<Int>, Point, Result<T, E>
    Adt {
        name: String,
        args: Vec<Ty>,
    },
    /// Capability type — gates access to the corresponding effect.
    /// A leaf type (no type arguments, no free variables).
    Cap(CapKind),
    /// The bottom type for diverging expressions (return, panic, etc.)
    /// Never unifies with any type (as a subtype of all types).
    /// This prevents "unreachable code" unification failures.
    Never,
}

impl Ty {
    #[inline]
    pub fn var(id: TypeVarId) -> Self {
        Ty::Var(id)
    }
    #[inline]
    pub fn unit() -> Self {
        Ty::Const(TyConst::Unit)
    }
    #[inline]
    pub fn bool_() -> Self {
        Ty::Const(TyConst::Bool)
    }
    #[inline]
    pub fn int() -> Self {
        Ty::Const(TyConst::Int)
    }
    /// Create a float type
    #[inline]
    pub fn float() -> Self {
        Ty::Const(TyConst::Float)
    }
    /// Create a string type
    #[inline]
    pub fn string() -> Self {
        Ty::Const(TyConst::String)
    }
    /// Create a function type with pure effects (default).
    #[inline]
    pub fn arrow(params: Vec<Ty>, ret: Ty) -> Self {
        Ty::Arrow(params, Box::new(ret), EffectRow::pure())
    }
    /// Create a function type with an explicit effect row.
    #[inline]
    pub fn arrow_eff(params: Vec<Ty>, ret: Ty, eff: EffectRow) -> Self {
        Ty::Arrow(params, Box::new(ret), eff)
    }
    // Convenience for single-param arrows (pure)
    pub fn arrow1(param: Ty, ret: Ty) -> Self {
        Ty::Arrow(vec![param], Box::new(ret), EffectRow::pure())
    }
    // Convenience for zero-param (thunks, pure)
    pub fn thunk(ret: Ty) -> Self {
        Ty::Arrow(vec![], Box::new(ret), EffectRow::pure())
    }
    #[inline]
    pub fn tuple(elems: impl Into<Vec<Ty>>) -> Self {
        Ty::Tuple(elems.into())
    }
    #[inline]
    pub fn list(elem: Ty) -> Self {
        Ty::List(Box::new(elem))
    }

    /// Create a capability type.
    #[inline]
    pub fn cap(kind: CapKind) -> Self {
        Ty::Cap(kind)
    }
    /// Convenience: FsCap type.
    #[inline]
    pub fn fs_cap() -> Self {
        Ty::Cap(CapKind::Fs)
    }
    /// Convenience: NetCap type.
    #[inline]
    pub fn net_cap() -> Self {
        Ty::Cap(CapKind::Net)
    }

    /// Create an ADT type with the given name and type arguments
    #[inline]
    pub fn adt(name: impl Into<String>, args: Vec<Ty>) -> Self {
        Ty::Adt {
            name: name.into(),
            args,
        }
    }

    /// Create an ADT type with no type arguments
    #[inline]
    pub fn adt0(name: impl Into<String>) -> Self {
        Ty::Adt {
            name: name.into(),
            args: vec![],
        }
    }
}

// Optional but nice: readable printing for errors
impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Var(v) => write!(f, "t{}", v.0),
            Ty::Const(TyConst::Unit) => write!(f, "Unit"),
            Ty::Const(TyConst::Bool) => write!(f, "Bool"),
            Ty::Const(TyConst::Int) => write!(f, "Int"),
            Ty::Const(TyConst::Float) => write!(f, "Float"),
            Ty::Const(TyConst::String) => write!(f, "String"),
            Ty::Arrow(params, ret, eff) => {
                if params.is_empty() {
                    write!(f, "() -> {}", ret)?;
                } else if params.len() == 1 {
                    write!(f, "{} -> {}", params[0], ret)?;
                } else {
                    write!(f, "(")?;
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", p)?;
                    }
                    write!(f, ") -> {}", ret)?;
                }
                if !eff.is_empty() {
                    write!(f, " & {}", eff)?;
                }
                Ok(())
            }
            Ty::Tuple(xs) => {
                let mut first = true;
                write!(f, "(")?;
                for x in xs {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{x}")?;
                }
                write!(f, ")")
            }
            Ty::List(x) => write!(f, "[{}]", x),
            Ty::Adt { name, args } => {
                if args.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}<", name)?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")
                }
            }
            Ty::Cap(kind) => write!(f, "{}", kind.type_name()),
            Ty::Never => write!(f, "!"),
        }
    }
}

use std::collections::HashSet;

/// Type scheme for polymorphism: ∀α β. τ
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct Scheme {
    pub type_vars: Vec<TypeVarId>,     // The ∀-bound type variables
    pub effect_vars: Vec<EffectVarId>, // The ∀-bound effect variables
    pub ty: Ty,
}

impl Scheme {
    /// Create a monomorphic scheme (no ∀ variables)
    pub fn mono(ty: Ty) -> Scheme {
        Scheme {
            type_vars: vec![],
            effect_vars: vec![],
            ty,
        }
    }

    /// Instantiate a scheme with fresh type and effect variables
    ///
    /// Example: ∀α e. (α -> α & e) becomes (β -> β & e') (where β, e' are fresh)
    ///
    /// `fresh_types` must have exactly one entry per ∀-bound type variable.
    /// `fresh_effects` must have exactly one entry per ∀-bound effect variable.
    ///
    /// Returns Err if the substitution encounters a cycle (should not happen with
    /// locally-built substitutions, but we propagate rather than panic).
    pub fn instantiate(
        &self,
        fresh_types: &[Ty],
        fresh_effects: &[EffectVarId],
    ) -> Result<Ty, super::subst::SubstError> {
        if self.type_vars.is_empty() && self.effect_vars.is_empty() {
            // Monomorphic - just return the type
            return Ok(self.ty.clone());
        }

        if fresh_types.len() != self.type_vars.len()
            || fresh_effects.len() != self.effect_vars.len()
        {
            return Err(super::subst::SubstError::InstantiationArityMismatch {
                expected_types: self.type_vars.len(),
                got_types: fresh_types.len(),
                expected_effects: self.effect_vars.len(),
                got_effects: fresh_effects.len(),
            });
        }

        use super::subst::Subst;
        let mut subst = Subst::new();

        // Each ∀-bound type var maps to a fresh type var
        for (var, fresh) in self.type_vars.iter().zip(fresh_types) {
            subst.insert(*var, fresh.clone());
        }

        // Each ∀-bound effect var maps to a fresh open effect row
        for (var, fresh_id) in self.effect_vars.iter().zip(fresh_effects) {
            subst.insert_effect(*var, EffectRow::open(0, *fresh_id));
        }

        // Apply substitution to the type (handles both type vars and effect rows)
        subst.apply(&self.ty)
    }
}

/// Constraint for constraint-based type inference
#[derive(Clone, Debug)]
pub enum Constraint {
    /// Type equality: t1 ~ t2
    /// Includes the span where this constraint was generated
    Equal(Ty, Ty, Span),
    /// Effect subset: row1 ⊆ row2
    /// Used to enforce that a function body's effects fit within its declaration.
    EffectSubset(EffectRow, EffectRow, Span),
}

/// Find free type variables in a type
pub fn free_vars(ty: &Ty) -> HashSet<TypeVarId> {
    match ty {
        Ty::Const(_) | Ty::Never | Ty::Cap(_) => HashSet::new(),
        Ty::Var(id) => {
            let mut set = HashSet::new();
            set.insert(*id);
            set
        }
        Ty::Arrow(params, ret, _eff) => {
            let mut set = HashSet::new();
            for param in params {
                set.extend(free_vars(param));
            }
            set.extend(free_vars(ret));
            set
        }
        Ty::Tuple(tys) => {
            let mut set = HashSet::new();
            for ty in tys {
                set.extend(free_vars(ty));
            }
            set
        }
        Ty::List(ty) => free_vars(ty),
        Ty::Adt { args, .. } => {
            let mut set = HashSet::new();
            for arg in args {
                set.extend(free_vars(arg));
            }
            set
        }
    }
}

/// Find free effect variables in a type
pub fn free_effect_vars(ty: &Ty) -> HashSet<EffectVarId> {
    match ty {
        Ty::Const(_) | Ty::Never | Ty::Var(_) | Ty::Cap(_) => HashSet::new(),
        Ty::Arrow(params, ret, eff) => {
            let mut set = HashSet::new();
            for param in params {
                set.extend(free_effect_vars(param));
            }
            set.extend(free_effect_vars(ret));
            if let Some(tail) = eff.tail {
                set.insert(tail);
            }
            set
        }
        Ty::Tuple(tys) => {
            let mut set = HashSet::new();
            for ty in tys {
                set.extend(free_effect_vars(ty));
            }
            set
        }
        Ty::List(ty) => free_effect_vars(ty),
        Ty::Adt { args, .. } => {
            let mut set = HashSet::new();
            for arg in args {
                set.extend(free_effect_vars(arg));
            }
            set
        }
    }
}

/// Find free type variables in a type scheme
///
/// Free variables are those in the type that are NOT ∀-bound.
///
/// Example: ∀t0. t0 -> t1 has free var t1 (t0 is bound)
pub fn free_vars_scheme(scheme: &Scheme) -> HashSet<TypeVarId> {
    let ty_vars = free_vars(&scheme.ty);
    let bound_vars: HashSet<TypeVarId> = scheme.type_vars.iter().copied().collect();

    // Free vars = vars in type - bound vars
    ty_vars.difference(&bound_vars).copied().collect()
}

/// Find free effect variables in a type scheme
pub fn free_effect_vars_scheme(scheme: &Scheme) -> HashSet<EffectVarId> {
    let eff_vars = free_effect_vars(&scheme.ty);
    let bound_vars: HashSet<EffectVarId> = scheme.effect_vars.iter().copied().collect();
    eff_vars.difference(&bound_vars).copied().collect()
}

/// Find free type variables across an entire environment
///
/// This is the union of free vars in all schemes in the environment.
pub fn free_vars_env(env: &std::collections::HashMap<String, Scheme>) -> HashSet<TypeVarId> {
    let mut vars = HashSet::new();
    for scheme in env.values() {
        vars.extend(free_vars_scheme(scheme));
    }
    vars
}

/// Find free effect variables across an entire environment
pub fn free_effect_vars_env(
    env: &std::collections::HashMap<String, Scheme>,
) -> HashSet<EffectVarId> {
    let mut vars = HashSet::new();
    for scheme in env.values() {
        vars.extend(free_effect_vars_scheme(scheme));
    }
    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instantiate_monomorphic() {
        let scheme = Scheme::mono(Ty::int());
        let ty = scheme.instantiate(&[], &[]).unwrap();
        assert_eq!(ty, Ty::int()); // Unchanged
    }

    #[test]
    fn instantiate_polymorphic() {
        // Scheme: ∀t0. t0 -> t0
        let scheme = Scheme {
            type_vars: vec![TypeVarId(0)],
            effect_vars: vec![],
            ty: Ty::arrow1(Ty::Var(TypeVarId(0)), Ty::Var(TypeVarId(0))),
        };

        let fresh_types = vec![Ty::Var(TypeVarId(100))];
        let ty = scheme.instantiate(&fresh_types, &[]).unwrap();
        // Should be: t100 -> t100
        assert_eq!(
            ty,
            Ty::arrow1(Ty::Var(TypeVarId(100)), Ty::Var(TypeVarId(100)))
        );
    }

    #[test]
    fn instantiate_effect_vars() {
        // Scheme: ∀t0 e0. t0 -> t0 & e0
        let scheme = Scheme {
            type_vars: vec![TypeVarId(0)],
            effect_vars: vec![EffectVarId(0)],
            ty: Ty::arrow_eff(
                vec![Ty::Var(TypeVarId(0))],
                Ty::Var(TypeVarId(0)),
                EffectRow::open(0, EffectVarId(0)),
            ),
        };

        let fresh_types = vec![Ty::Var(TypeVarId(50))];
        let fresh_effects = vec![EffectVarId(51)];
        let ty = scheme.instantiate(&fresh_types, &fresh_effects).unwrap();
        // Should be: t50 -> t50 & e51
        assert_eq!(
            ty,
            Ty::arrow_eff(
                vec![Ty::Var(TypeVarId(50))],
                Ty::Var(TypeVarId(50)),
                EffectRow::open(0, EffectVarId(51)),
            )
        );
    }

    #[test]
    fn instantiate_arity_mismatch_returns_error() {
        use crate::infer::subst::SubstError;

        // Scheme: ∀t0. t0 -> t0 (expects 1 type var, 0 effect vars)
        let scheme = Scheme {
            type_vars: vec![TypeVarId(0)],
            effect_vars: vec![],
            ty: Ty::arrow1(Ty::Var(TypeVarId(0)), Ty::Var(TypeVarId(0))),
        };

        // Pass wrong number of fresh types
        let result = scheme.instantiate(&[], &[]);
        assert!(matches!(
            result,
            Err(SubstError::InstantiationArityMismatch { .. })
        ));

        // Pass too many fresh types
        let result = scheme.instantiate(&[Ty::Var(TypeVarId(10)), Ty::Var(TypeVarId(11))], &[]);
        assert!(matches!(
            result,
            Err(SubstError::InstantiationArityMismatch { .. })
        ));
    }
}
