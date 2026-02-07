use super::ty::{Ty, TypeVarId};
use crate::effects::{EffectRow, EffectVarId};
use std::collections::{HashMap, HashSet};

/// Errors that can occur during substitution application
#[derive(Debug, Clone)]
pub enum SubstError {
    /// Cyclic effect variable substitution detected
    EffectCycle { var: EffectVarId },
    /// Effect substitution chain exceeded depth limit
    EffectChainTooDeep { depth: usize },
    /// Scheme instantiation arity mismatch (internal invariant violation)
    InstantiationArityMismatch {
        expected_types: usize,
        got_types: usize,
        expected_effects: usize,
        got_effects: usize,
    },
}

impl std::fmt::Display for SubstError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubstError::EffectCycle { var } => {
                write!(
                    f,
                    "cyclic effect variable substitution: {} refers to itself",
                    var
                )
            }
            SubstError::EffectChainTooDeep { depth } => {
                write!(
                    f,
                    "effect substitution chain too deep ({} steps); possible cycle",
                    depth
                )
            }
            SubstError::InstantiationArityMismatch {
                expected_types,
                got_types,
                expected_effects,
                got_effects,
            } => {
                write!(
                    f,
                    "scheme instantiation arity mismatch: expected {} type vars (got {}), {} effect vars (got {})",
                    expected_types, got_types, expected_effects, got_effects
                )
            }
        }
    }
}

impl std::error::Error for SubstError {}

#[derive(Clone, Debug, Default)]
pub struct Subst {
    map: HashMap<TypeVarId, Ty>,
    effect_map: HashMap<EffectVarId, EffectRow>,
}

impl Subst {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            effect_map: HashMap::new(),
        }
    }
    pub fn get(&self, v: &TypeVarId) -> Option<&Ty> {
        self.map.get(v)
    }
    pub fn insert(&mut self, v: TypeVarId, t: Ty) {
        self.map.insert(v, t);
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty() && self.effect_map.is_empty()
    }

    // ---- Effect variable substitution ----

    pub fn get_effect(&self, v: &EffectVarId) -> Option<&EffectRow> {
        self.effect_map.get(v)
    }

    pub fn insert_effect(&mut self, v: EffectVarId, row: EffectRow) {
        self.effect_map.insert(v, row);
    }

    /// Check if an effect variable occurs in the substitution chain starting from a row's tail.
    ///
    /// Returns true if binding `var` to `row` would create a cycle.
    /// This is the effect-variable analogue of the type occurs check.
    pub fn effect_var_occurs_in(&self, var: EffectVarId, row: &EffectRow) -> bool {
        let mut current_tail = match row.tail {
            None => return false, // Closed row — no variable to create a cycle
            Some(t) => t,
        };
        let mut visited = HashSet::new();
        visited.insert(var); // The variable we're about to bind
        loop {
            if current_tail == var {
                return true; // Found the variable — would create a cycle
            }
            if !visited.insert(current_tail) {
                return false; // Already-existing cycle in the chain, not involving var
            }
            match self.effect_map.get(&current_tail) {
                Some(replacement) => match replacement.tail {
                    Some(next) => current_tail = next,
                    None => return false, // Chain ends at a closed row
                },
                None => return false, // Unbound variable — no cycle
            }
        }
    }

    /// Apply effect substitution to an effect row, chasing chains iteratively.
    ///
    /// Uses a visited set to detect cycles and a depth bound as a safety net.
    /// Returns an error if a cycle is detected or the depth limit is exceeded.
    pub fn apply_effect_row(&self, row: &EffectRow) -> Result<EffectRow, SubstError> {
        const MAX_CHAIN_DEPTH: usize = 256;

        let mut current = *row;
        let mut visited = HashSet::new();

        for depth in 0..MAX_CHAIN_DEPTH {
            match current.tail {
                None => return Ok(current), // closed row, done
                Some(var) => {
                    if let Some(replacement) = self.effect_map.get(&var) {
                        // Cycle detection: if we've seen this var before, it's a cycle
                        if !visited.insert(var) {
                            return Err(SubstError::EffectCycle { var });
                        }
                        // Chase the chain: merge concrete effects and follow the tail
                        current = EffectRow {
                            concrete: current.concrete | replacement.concrete,
                            tail: replacement.tail,
                        };
                    } else {
                        return Ok(current); // tail var not in substitution, done
                    }
                }
            }
            // Safety: shouldn't reach here if cycle detection works, but defend anyway
            if depth == MAX_CHAIN_DEPTH - 1 {
                return Err(SubstError::EffectChainTooDeep {
                    depth: MAX_CHAIN_DEPTH,
                });
            }
        }
        Err(SubstError::EffectChainTooDeep {
            depth: MAX_CHAIN_DEPTH,
        })
    }

    pub fn apply(&self, t: &Ty) -> Result<Ty, SubstError> {
        match t {
            Ty::Var(v) => {
                if let Some(ty) = self.map.get(v) {
                    self.apply(ty) // Recursively chase substitutions!
                } else {
                    Ok(Ty::Var(*v))
                }
            }
            Ty::Const(_) | Ty::Never | Ty::Cap(_) => Ok(t.clone()),
            Ty::Arrow(params, ret, eff) => {
                let new_params: Result<Vec<Ty>, SubstError> =
                    params.iter().map(|p| self.apply(p)).collect();
                let new_ret = self.apply(ret)?;
                let new_eff = self.apply_effect_row(eff)?;
                Ok(Ty::arrow_eff(new_params?, new_ret, new_eff))
            }
            Ty::Tuple(xs) => {
                let new_xs: Result<Vec<Ty>, SubstError> =
                    xs.iter().map(|x| self.apply(x)).collect();
                Ok(Ty::tuple(new_xs?))
            }
            Ty::List(x) => Ok(Ty::list(self.apply(x)?)),
            Ty::Adt { name, args } => {
                let new_args: Result<Vec<Ty>, SubstError> =
                    args.iter().map(|a| self.apply(a)).collect();
                Ok(Ty::adt(name.clone(), new_args?))
            }
            Ty::Ref(inner) => Ok(Ty::Ref(Box::new(self.apply(inner)?))),
        }
    }

    /// self ∘ other (apply `other` first, then `self`)
    pub fn compose(&self, other: &Subst) -> Result<Subst, SubstError> {
        let mut out = Subst::new();
        // Type map
        for (v, t) in other.map.iter() {
            out.insert(*v, self.apply(t)?);
        }
        for (v, t) in self.map.iter() {
            if !other.map.contains_key(v) {
                out.insert(*v, t.clone());
            }
        }
        // Effect map
        for (v, row) in other.effect_map.iter() {
            out.insert_effect(*v, self.apply_effect_row(row)?);
        }
        for (v, row) in self.effect_map.iter() {
            if !other.effect_map.contains_key(v) {
                out.insert_effect(*v, *row);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{EffectRow, EffectVarId};

    #[test]
    fn effect_occurs_check_detects_direct_cycle() {
        let subst = Subst::new();
        // Binding e0 → {..} ∪ e0 would create a direct cycle
        let row = EffectRow::open(0, EffectVarId(0));
        assert!(subst.effect_var_occurs_in(EffectVarId(0), &row));
    }

    #[test]
    fn effect_occurs_check_detects_indirect_cycle() {
        let mut subst = Subst::new();
        // e1 → {..} ∪ e0  (existing binding)
        subst.insert_effect(EffectVarId(1), EffectRow::open(0, EffectVarId(0)));
        // Now trying to bind e0 → {..} ∪ e1 would create: e0 → e1 → e0
        let row = EffectRow::open(0, EffectVarId(1));
        assert!(subst.effect_var_occurs_in(EffectVarId(0), &row));
    }

    #[test]
    fn effect_occurs_check_no_cycle_for_closed_row() {
        let subst = Subst::new();
        let row = EffectRow::closed(0);
        assert!(!subst.effect_var_occurs_in(EffectVarId(0), &row));
    }

    #[test]
    fn effect_occurs_check_no_cycle_for_different_var() {
        let subst = Subst::new();
        let row = EffectRow::open(0, EffectVarId(1));
        assert!(!subst.effect_var_occurs_in(EffectVarId(0), &row));
    }

    #[test]
    fn apply_effect_row_detects_cycle() {
        let mut subst = Subst::new();
        // Create a cycle: e0 → e1, e1 → e0
        subst.insert_effect(EffectVarId(0), EffectRow::open(0, EffectVarId(1)));
        subst.insert_effect(EffectVarId(1), EffectRow::open(0, EffectVarId(0)));
        let row = EffectRow::open(0, EffectVarId(0));
        let result = subst.apply_effect_row(&row);
        assert!(matches!(result, Err(SubstError::EffectCycle { .. })));
    }
}
