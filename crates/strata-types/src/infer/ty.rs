use std::fmt;
use std::hash::{Hash, Hasher};
use strata_ast::span::Span;

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
    Arrow(Vec<Ty>, Box<Ty>),
    /// Heterogeneous, fixed-length tuple: (t0, t1, ..., tn)
    Tuple(Vec<Ty>),
    /// Homogeneous list: [elem]
    List(Box<Ty>),
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
    #[inline]
    pub fn arrow(params: Vec<Ty>, ret: Ty) -> Self {
        Ty::Arrow(params, Box::new(ret))
    }
    // Convenience for single-param arrows
    pub fn arrow1(param: Ty, ret: Ty) -> Self {
        Ty::Arrow(vec![param], Box::new(ret))
    }
    // Convenience for zero-param (thunks)
    pub fn thunk(ret: Ty) -> Self {
        Ty::Arrow(vec![], Box::new(ret))
    }
    #[inline]
    pub fn tuple(elems: impl Into<Vec<Ty>>) -> Self {
        Ty::Tuple(elems.into())
    }
    #[inline]
    pub fn list(elem: Ty) -> Self {
        Ty::List(Box::new(elem))
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
            Ty::Arrow(params, ret) => {
                if params.is_empty() {
                    write!(f, "() -> {}", ret)
                } else if params.len() == 1 {
                    write!(f, "{} -> {}", params[0], ret)
                } else {
                    write!(f, "(")?;
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", p)?;
                    }
                    write!(f, ") -> {}", ret)
                }
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
            Ty::Never => write!(f, "!"),
        }
    }
}

use std::collections::HashSet;

/// Type scheme for polymorphism: ∀α β. τ
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct Scheme {
    pub vars: Vec<TypeVarId>, // The ∀-bound variables
    pub ty: Ty,
}

impl Scheme {
    /// Create a monomorphic scheme (no ∀ variables)
    pub fn mono(ty: Ty) -> Scheme {
        Scheme { vars: vec![], ty }
    }

    /// Instantiate a scheme with fresh type variables
    ///
    /// Example: ∀α. α -> α becomes β -> β (where β is fresh)
    ///
    /// The `fresh` function should generate a fresh type variable each time it's called.
    pub fn instantiate<F>(&self, mut fresh: F) -> Ty
    where
        F: FnMut() -> Ty,
    {
        if self.vars.is_empty() {
            // Monomorphic - just return the type
            return self.ty.clone();
        }

        // Create substitution: each ∀-bound var maps to a fresh var
        use super::subst::Subst;
        let mut subst = Subst::new();
        for var in &self.vars {
            subst.insert(*var, fresh());
        }

        // Apply substitution to the type
        subst.apply(&self.ty)
    }
}

/// Constraint for constraint-based type inference
#[derive(Clone, Debug)]
pub enum Constraint {
    /// Type equality: t1 ~ t2
    /// Includes the span where this constraint was generated
    Equal(Ty, Ty, Span),
}

/// Find free type variables in a type
pub fn free_vars(ty: &Ty) -> HashSet<TypeVarId> {
    match ty {
        Ty::Const(_) | Ty::Never => HashSet::new(),
        Ty::Var(id) => {
            let mut set = HashSet::new();
            set.insert(*id);
            set
        }
        Ty::Arrow(params, ret) => {
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
    }
}

/// Find free type variables in a type scheme
///
/// Free variables are those in the type that are NOT ∀-bound.
///
/// Example: ∀t0. t0 -> t1 has free var t1 (t0 is bound)
pub fn free_vars_scheme(scheme: &Scheme) -> HashSet<TypeVarId> {
    let ty_vars = free_vars(&scheme.ty);
    let bound_vars: HashSet<TypeVarId> = scheme.vars.iter().copied().collect();

    // Free vars = vars in type - bound vars
    ty_vars.difference(&bound_vars).copied().collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instantiate_monomorphic() {
        let scheme = Scheme::mono(Ty::int());
        let fresh = || Ty::Var(TypeVarId(999));
        let ty = scheme.instantiate(fresh);
        assert_eq!(ty, Ty::int()); // Unchanged
    }

    #[test]
    fn instantiate_polymorphic() {
        // Scheme: ∀t0. t0 -> t0
        let scheme = Scheme {
            vars: vec![TypeVarId(0)],
            ty: Ty::arrow1(Ty::Var(TypeVarId(0)), Ty::Var(TypeVarId(0))),
        };

        let mut counter = 100;
        let fresh = || {
            let v = Ty::Var(TypeVarId(counter));
            counter += 1;
            v
        };

        let ty = scheme.instantiate(fresh);
        // Should be: t100 -> t100
        assert_eq!(
            ty,
            Ty::arrow1(Ty::Var(TypeVarId(100)), Ty::Var(TypeVarId(100)))
        );
    }
}
