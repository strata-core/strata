//! Minimal unification engine for inference scaffolding.
//!
//! ```rust
//! use strata_types::infer::{Unifier, Ty as T, TypeVarId as V};
//! let mut u = Unifier::new();
//! let x = T::var(V(0));
//! let int = T::int();
//! u.unify(&x, &int).unwrap();
//! assert_eq!(u.subst().apply(&x), T::int());
//! ```

use super::subst::Subst;
use super::ty::{Ty, TypeVarId};
use std::fmt;

pub type TypeResult<T> = Result<T, TypeError>;

#[derive(Debug, Clone)]
pub enum TypeError {
    Mismatch(Ty, Ty),
    Occurs {
        var: TypeVarId,
        ty: Ty,
    },
    /// Tuple arity differs: (a,b,...) vs (x,y,...) with different lengths
    Arity {
        left: usize,
        right: usize,
    },
}
impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::Mismatch(a, b) => write!(f, "type mismatch: {:?} vs {:?}", a, b),
            TypeError::Occurs { var, ty } => {
                write!(f, "occurs check failed: {:?} in {:?}", var, ty)
            }
            TypeError::Arity { left, right } => {
                write!(f, "tuple arity mismatch: {left} vs {right}")
            }
        }
    }
}
impl std::error::Error for TypeError {}

#[derive(Clone, Debug, Default)]
pub struct Unifier {
    subst: Subst,
}

impl Unifier {
    pub fn new() -> Self {
        Self {
            subst: Subst::new(),
        }
    }
    pub fn subst(&self) -> &Subst {
        &self.subst
    }
    pub fn into_subst(self) -> Subst {
        self.subst
    }

    pub fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), TypeError> {
        let a = self.subst.apply(a);
        let b = self.subst.apply(b);
        match (a, b) {
            // Never (bottom type) only unifies with itself.
            // Divergence handling (e.g., if one branch returns) is done in inference,
            // not unification. This prevents soundness holes where Never would allow
            // mismatched types to unify (e.g., `if c { return 1; } else { "str" }`).
            (Ty::Never, Ty::Never) => Ok(()),

            (Ty::Var(v), t) | (t, Ty::Var(v)) => self.unify_var(v, t),
            (Ty::Const(c1), Ty::Const(c2)) if c1 == c2 => Ok(()),

            (Ty::Arrow(params1, ret1), Ty::Arrow(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return Err(TypeError::Arity {
                        left: params1.len(),
                        right: params2.len(),
                    });
                }
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    self.unify(p1, p2)?;
                }
                self.unify(&ret1, &ret2)
            }

            (Ty::Tuple(xs), Ty::Tuple(ys)) => {
                if xs.len() != ys.len() {
                    return Err(TypeError::Arity {
                        left: xs.len(),
                        right: ys.len(),
                    });
                }
                for (x, y) in xs.iter().zip(ys.iter()) {
                    self.unify(x, y)?;
                }
                Ok(())
            }

            (Ty::List(x), Ty::List(y)) => self.unify(&x, &y),

            // ADT unification: names must match, then unify type arguments
            (Ty::Adt { name: n1, args: a1 }, Ty::Adt { name: n2, args: a2 }) => {
                if n1 != n2 {
                    return Err(TypeError::Mismatch(Ty::adt(n1, a1), Ty::adt(n2, a2)));
                }
                if a1.len() != a2.len() {
                    return Err(TypeError::Arity {
                        left: a1.len(),
                        right: a2.len(),
                    });
                }
                for (arg1, arg2) in a1.iter().zip(a2.iter()) {
                    self.unify(arg1, arg2)?;
                }
                Ok(())
            }

            (x, y) => Err(TypeError::Mismatch(x, y)),
        }
    }

    fn unify_var(&mut self, v: TypeVarId, t: Ty) -> Result<(), TypeError> {
        if let Some(existing) = self.subst.get(&v).cloned() {
            return self.unify(&existing, &t);
        }
        if matches!(t, Ty::Var(w) if w == v) {
            return Ok(());
        }
        if occurs_in(v, &t, &self.subst) {
            return Err(TypeError::Occurs { var: v, ty: t });
        }
        self.subst.insert(v, t);
        Ok(())
    }
}

fn occurs_in(v: TypeVarId, ty: &Ty, subst: &Subst) -> bool {
    let ty = subst.apply(ty);
    match ty {
        Ty::Var(w) => w == v,
        Ty::Const(_) | Ty::Never => false,
        Ty::Arrow(ref params, ref ret) => {
            params.iter().any(|p| occurs_in(v, p, subst)) || occurs_in(v, ret, subst)
        }
        Ty::Tuple(ref xs) => xs.iter().any(|x| occurs_in(v, x, subst)),
        Ty::List(ref x) => occurs_in(v, x, subst),
        Ty::Adt { ref args, .. } => args.iter().any(|a| occurs_in(v, a, subst)),
    }
}
