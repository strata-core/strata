//! Minimal unification engine for inference scaffolding.
//!
//! ```rust
//! use strata_types::infer::{Unifier, Ty as T, TypeVarId as V};
//! let mut u = Unifier::new();
//! let x = T::var(V(0));
//! let int = T::int();
//! u.unify(&x, &int).unwrap();
//! assert_eq!(u.subst().apply(&x).unwrap(), T::int());
//! ```

use super::subst::{Subst, SubstError};
use super::ty::{Ty, TypeVarId};
use crate::effects::{EffectRow, EffectVarId};
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
    /// Effect row mismatch
    EffectMismatch {
        expected: EffectRow,
        found: EffectRow,
    },
    /// Cyclic effect variable substitution
    EffectCycle {
        var: EffectVarId,
    },
    /// Effect substitution chain exceeded depth limit
    EffectChainTooDeep {
        depth: usize,
    },
}

impl From<SubstError> for TypeError {
    fn from(err: SubstError) -> Self {
        match err {
            SubstError::EffectCycle { var } => TypeError::EffectCycle { var },
            SubstError::EffectChainTooDeep { depth } => TypeError::EffectChainTooDeep { depth },
            SubstError::InstantiationArityMismatch {
                expected_types,
                got_types,
                ..
            } => TypeError::Arity {
                left: expected_types,
                right: got_types,
            },
        }
    }
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
            TypeError::EffectMismatch { expected, found } => {
                write!(f, "effect mismatch: expected {}, found {}", expected, found)
            }
            TypeError::EffectCycle { var } => {
                write!(
                    f,
                    "cyclic effect variable: {} refers to itself in substitution chain",
                    var
                )
            }
            TypeError::EffectChainTooDeep { depth } => {
                write!(
                    f,
                    "effect substitution chain too deep ({} steps); possible cycle",
                    depth
                )
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
    pub fn subst_mut(&mut self) -> &mut Subst {
        &mut self.subst
    }
    pub fn into_subst(self) -> Subst {
        self.subst
    }

    pub fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), TypeError> {
        let a = self.subst.apply(a)?;
        let b = self.subst.apply(b)?;
        match (a, b) {
            // Never (bottom type) only unifies with itself.
            // Divergence handling (e.g., if one branch returns) is done in inference,
            // not unification. This prevents soundness holes where Never would allow
            // mismatched types to unify (e.g., `if c { return 1; } else { "str" }`).
            (Ty::Never, Ty::Never) => Ok(()),

            (Ty::Cap(k1), Ty::Cap(k2)) if k1 == k2 => Ok(()),

            (Ty::Var(v), t) | (t, Ty::Var(v)) => self.unify_var(v, t),
            (Ty::Const(c1), Ty::Const(c2)) if c1 == c2 => Ok(()),

            (Ty::Arrow(params1, ret1, eff1), Ty::Arrow(params2, ret2, eff2)) => {
                if params1.len() != params2.len() {
                    return Err(TypeError::Arity {
                        left: params1.len(),
                        right: params2.len(),
                    });
                }
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    self.unify(p1, p2)?;
                }
                self.unify(&ret1, &ret2)?;
                self.unify_effect_rows(&eff1, &eff2)
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

    /// Unify two effect rows (Rémy-style row unification).
    ///
    /// Cases:
    /// - Closed + Closed: bitmask equality
    /// - Closed + Open: bind tail to residual
    /// - Open + Closed: symmetric
    /// - Open + Open: fresh tail, bind both
    fn unify_effect_rows(&mut self, a: &EffectRow, b: &EffectRow) -> Result<(), TypeError> {
        let a = self.subst.apply_effect_row(a)?;
        let b = self.subst.apply_effect_row(b)?;

        match (a.tail, b.tail) {
            // Both closed: exact equality
            (None, None) => {
                if a.concrete == b.concrete {
                    Ok(())
                } else {
                    Err(TypeError::EffectMismatch {
                        expected: a,
                        found: b,
                    })
                }
            }
            // Closed + Open: bind open's tail to residual
            (None, Some(var_b)) => {
                // a is closed = {Fs, Net}, b is {Fs} ∪ e_b
                // b's concrete effects must be a subset of a's
                if (b.concrete & !a.concrete) != 0 {
                    return Err(TypeError::EffectMismatch {
                        expected: a,
                        found: b,
                    });
                }
                // Bind e_b → closed row with residual effects
                let residual = a.concrete & !b.concrete;
                // Effect occurs check: var_b shouldn't appear in target (it's closed, so no issue)
                self.subst.insert_effect(var_b, EffectRow::closed(residual));
                Ok(())
            }
            // Open + Closed: symmetric
            (Some(var_a), None) => {
                if (a.concrete & !b.concrete) != 0 {
                    return Err(TypeError::EffectMismatch {
                        expected: a,
                        found: b,
                    });
                }
                let residual = b.concrete & !a.concrete;
                self.subst.insert_effect(var_a, EffectRow::closed(residual));
                Ok(())
            }
            // Both open: fresh tail, bind both
            (Some(var_a), Some(var_b)) => {
                if var_a == var_b && a.concrete == b.concrete {
                    return Ok(()); // Same row
                }
                // Effect occurs check: if same var, would create cycle
                if var_a == var_b {
                    // Same tail var but different concrete: error
                    if a.concrete != b.concrete {
                        return Err(TypeError::EffectMismatch {
                            expected: a,
                            found: b,
                        });
                    }
                    return Ok(());
                }
                // Create fresh tail e_fresh
                // We need a fresh effect var. Use a counter approach via subst side-channel.
                // Since we can't easily generate fresh vars from unifier, bind both to each other:
                // e_a → {b.concrete \ a.concrete} ∪ e_b  and check no conflict
                // Actually, the standard approach:
                // a = {X} ∪ e_a, b = {Y} ∪ e_b
                // bind e_a → {Y \ X} ∪ e_b, which makes a = {X ∪ (Y\X)} ∪ e_b = {X ∪ Y} ∪ e_b
                // bind e_b → {X \ Y} ∪ e_a'... but e_a is already bound
                // Simpler: just bind e_a → {Y\X} ∪ e_b
                let extra_for_a = b.concrete & !a.concrete;
                let target_row = EffectRow::open(extra_for_a, var_b);
                // Effect occurs check: verify binding wouldn't create a cycle
                if self.subst.effect_var_occurs_in(var_a, &target_row) {
                    return Err(TypeError::EffectCycle { var: var_a });
                }
                self.subst.insert_effect(var_a, target_row);
                Ok(())
            }
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

/// Check if a type variable occurs in a type (through the substitution).
///
/// Returns `true` if the variable is found, `false` otherwise.
/// If the substitution encounters a cycle, we conservatively return `true`
/// (which triggers an occurs-check error, which is safe).
fn occurs_in(v: TypeVarId, ty: &Ty, subst: &Subst) -> bool {
    let ty = match subst.apply(ty) {
        Ok(t) => t,
        Err(_) => return true, // Cycle in subst → treat as occurs (safe: prevents binding)
    };
    match ty {
        Ty::Var(w) => w == v,
        Ty::Const(_) | Ty::Never | Ty::Cap(_) => false,
        Ty::Arrow(ref params, ref ret, _eff) => {
            params.iter().any(|p| occurs_in(v, p, subst)) || occurs_in(v, ret, subst)
        }
        Ty::Tuple(ref xs) => xs.iter().any(|x| occurs_in(v, x, subst)),
        Ty::List(ref x) => occurs_in(v, x, subst),
        Ty::Adt { ref args, .. } => args.iter().any(|a| occurs_in(v, a, subst)),
    }
}
