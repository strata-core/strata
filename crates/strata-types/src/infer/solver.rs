//! Constraint solver
//!
//! Takes a set of constraints and solves them via unification.

use super::subst::{Subst, SubstError};
use super::ty::Constraint;
use super::unifier::{TypeError, Unifier};
use crate::effects::{EffectRow, EffectVarId};
use strata_ast::span::Span;

/// Error from constraint solving, including the span where the error occurred
#[derive(Debug, Clone)]
pub struct SolveError {
    /// The underlying type error from unification
    pub error: TypeError,
    /// The span of the constraint that failed
    pub span: Span,
}

impl std::fmt::Display for SolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {:?}", self.error, self.span)
    }
}

impl std::error::Error for SolveError {}

/// Constraint solver
pub struct Solver {
    unifier: Unifier,
}

impl Solver {
    /// Create a new solver
    pub fn new() -> Self {
        Solver {
            unifier: Unifier::new(),
        }
    }

    /// Helper: convert a SubstError to a SolveError with a span
    fn subst_err(&self, err: SubstError, span: Span) -> SolveError {
        SolveError {
            error: TypeError::from(err),
            span,
        }
    }

    /// Chase the effect substitution chain to find the canonical (terminal) variable.
    ///
    /// If the tail variable has been bound (aliased) in the substitution, follow
    /// the chain to the end. Returns the end-of-chain variable, or None if the
    /// row resolves to a closed row.
    fn canonical_effect_var(&self, var: Option<EffectVarId>) -> Option<EffectVarId> {
        let mut current = var?;
        let mut visited = std::collections::HashSet::new();
        loop {
            if !visited.insert(current) {
                // Cycle — return current as canonical (cycle will be caught elsewhere)
                return Some(current);
            }
            match self.unifier.subst().get_effect(&current) {
                Some(replacement) => match replacement.tail {
                    Some(next) => current = next,
                    None => return None, // Chain ends at a closed row
                },
                None => return Some(current), // Not aliased, this is canonical
            }
        }
    }

    /// Solve a set of constraints
    ///
    /// Returns the resulting substitution, or a solve error with span context if solving fails.
    ///
    /// Phase A: process type equality constraints through the unifier.
    /// Phase B: resolve effect subset constraints by accumulating required effects
    ///          for each effect variable to a fixpoint, then checking all constraints.
    pub fn solve(&mut self, constraints: Vec<Constraint>) -> Result<Subst, SolveError> {
        // Separate equality and effect constraints
        let mut equalities = Vec::new();
        let mut effect_subsets = Vec::new();

        for constraint in constraints {
            match constraint {
                Constraint::Equal(..) => equalities.push(constraint),
                Constraint::EffectSubset(..) => effect_subsets.push(constraint),
            }
        }

        // Phase A: Solve type equalities
        for constraint in equalities {
            match constraint {
                Constraint::Equal(t1, t2, span) => {
                    self.unifier
                        .unify(&t1, &t2)
                        .map_err(|error| SolveError { error, span })?;
                }
                Constraint::EffectSubset(..) => {
                    // Filtered above; this arm satisfies exhaustiveness without unreachable!()
                }
            }
        }

        // Phase B: Resolve effect subset constraints.
        //
        // Two-step approach:
        // Step 1 (Structural): For open-open subset constraints (both sub and sup
        //   have unresolved tail vars), bind sup's tail to track sub's tail.
        //   This preserves the structural relationship through generalization,
        //   enabling correct effect propagation through higher-order functions.
        // Step 2 (Fixpoint): Collect minimum required concrete effects per effect
        //   variable to a fixpoint, then check all constraints.

        // Step 1: Bind open-open effect tails
        for constraint in &effect_subsets {
            if let Constraint::EffectSubset(sub, sup, span) = constraint {
                let sub_resolved = self
                    .unifier
                    .subst()
                    .apply_effect_row(sub)
                    .map_err(|e| self.subst_err(e, *span))?;
                let sup_resolved = self
                    .unifier
                    .subst()
                    .apply_effect_row(sup)
                    .map_err(|e| self.subst_err(e, *span))?;

                if let (Some(sub_tail), Some(sup_tail)) = (sub_resolved.tail, sup_resolved.tail) {
                    if sub_tail == sup_tail {
                        // Same tail var — just check concrete subset
                        if (sub_resolved.concrete & !sup_resolved.concrete) != 0 {
                            return Err(SolveError {
                                error: TypeError::EffectMismatch {
                                    expected: sup_resolved,
                                    found: sub_resolved,
                                },
                                span: *span,
                            });
                        }
                        continue;
                    }
                    // Bind sup's tail to follow sub: e_sup → {sub.concrete \ sup.concrete} ∪ e_sub
                    // This makes sup = {sup.concrete ∪ sub.concrete} ∪ e_sub, ensuring sup ⊇ sub.
                    let extra_concrete = sub_resolved.concrete & !sup_resolved.concrete;
                    let target_row = EffectRow::open(extra_concrete, sub_tail);
                    // Effect occurs check: verify binding wouldn't create a cycle
                    if self
                        .unifier
                        .subst()
                        .effect_var_occurs_in(sup_tail, &target_row)
                    {
                        return Err(SolveError {
                            error: TypeError::EffectCycle { var: sup_tail },
                            span: *span,
                        });
                    }
                    self.unifier.subst_mut().insert_effect(sup_tail, target_row);
                }
            }
        }

        // Step 2: Fixpoint accumulation for concrete effects
        use std::collections::HashMap;

        let mut required: HashMap<EffectVarId, u64> = HashMap::new();

        // Iterate to fixpoint: subset constraints may chain (a <= b <= c),
        // so resolving one may reveal requirements for another.
        const MAX_EFFECT_ITERATIONS: usize = 64;
        let mut converged = false;

        // Track a span for error reporting if fixpoint doesn't converge
        let first_effect_span = effect_subsets
            .iter()
            .find_map(|c| match c {
                Constraint::EffectSubset(_, _, span) => Some(*span),
                _ => None,
            })
            .unwrap_or(Span { start: 0, end: 0 });

        for _ in 0..MAX_EFFECT_ITERATIONS {
            let mut changed = false;

            for constraint in &effect_subsets {
                if let Constraint::EffectSubset(sub, sup, span) = constraint {
                    // Resolve sub through the unifier's substitution
                    let sub_resolved = self
                        .unifier
                        .subst()
                        .apply_effect_row(sub)
                        .map_err(|e| self.subst_err(e, *span))?;
                    // Chase to canonical tail variable (MLR-3: alias resolution)
                    let sub_canonical_tail = self.canonical_effect_var(sub_resolved.tail);
                    // Add any accumulated requirements for sub's canonical tail
                    let sub_concrete = match sub_canonical_tail {
                        None => sub_resolved.concrete,
                        Some(var) => {
                            sub_resolved.concrete | required.get(&var).copied().unwrap_or(0)
                        }
                    };

                    // Resolve sup through the unifier's substitution
                    let sup_resolved = self
                        .unifier
                        .subst()
                        .apply_effect_row(sup)
                        .map_err(|e| self.subst_err(e, *span))?;
                    // Chase to canonical tail variable (MLR-3: alias resolution)
                    let sup_canonical_tail = self.canonical_effect_var(sup_resolved.tail);
                    // Current concrete effects of sup (including accumulated requirements)
                    let sup_concrete_with_req = match sup_canonical_tail {
                        None => sup_resolved.concrete,
                        Some(var) => {
                            sup_resolved.concrete | required.get(&var).copied().unwrap_or(0)
                        }
                    };

                    // What effects does sup need beyond what it already has?
                    let needed = sub_concrete & !sup_concrete_with_req;

                    if needed != 0 {
                        if let Some(sup_var) = sup_canonical_tail {
                            let entry = required.entry(sup_var).or_insert(0);
                            let old = *entry;
                            *entry |= needed;
                            if *entry != old {
                                changed = true;
                            }
                        }
                        // If sup is closed and needed != 0, that's an error
                        // (handled in the check pass below)
                    }
                }
            }

            if !changed {
                converged = true;
                break;
            }
        }

        if !converged {
            // MLR-6: Return a specific non-convergence error instead of a fake mismatch
            return Err(SolveError {
                error: TypeError::EffectChainTooDeep {
                    depth: MAX_EFFECT_ITERATIONS,
                },
                span: first_effect_span,
            });
        }

        // Now check all constraints with the accumulated requirements.
        for constraint in &effect_subsets {
            if let Constraint::EffectSubset(sub, sup, span) = constraint {
                let sub_resolved = self
                    .unifier
                    .subst()
                    .apply_effect_row(sub)
                    .map_err(|e| self.subst_err(e, *span))?;
                let sub_canonical_tail = self.canonical_effect_var(sub_resolved.tail);
                let sub_concrete = match sub_canonical_tail {
                    None => sub_resolved.concrete,
                    Some(var) => sub_resolved.concrete | required.get(&var).copied().unwrap_or(0),
                };

                let sup_resolved = self
                    .unifier
                    .subst()
                    .apply_effect_row(sup)
                    .map_err(|e| self.subst_err(e, *span))?;
                let sup_canonical_tail = self.canonical_effect_var(sup_resolved.tail);
                let sup_concrete = match sup_canonical_tail {
                    None => sup_resolved.concrete,
                    Some(var) => sup_resolved.concrete | required.get(&var).copied().unwrap_or(0),
                };

                // Check: sub's effects must be a subset of sup's effects
                if (sub_concrete & !sup_concrete) != 0 {
                    let found = EffectRow::closed(sub_concrete);
                    let expected = EffectRow::closed(sup_concrete);
                    return Err(SolveError {
                        error: TypeError::EffectMismatch { expected, found },
                        span: *span,
                    });
                }
            }
        }

        // Bind the accumulated effects into the substitution so the final
        // substitution reflects resolved effect rows.
        for (var, effects) in &required {
            if self.unifier.subst().get_effect(var).is_none() {
                self.unifier
                    .subst_mut()
                    .insert_effect(*var, EffectRow::closed(*effects));
            }
        }

        Ok(self.unifier.clone().into_subst())
    }
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::ty::{Ty, TypeVarId};
    use strata_ast::span::Span;

    #[test]
    fn solve_simple_equality() {
        let mut solver = Solver::new();

        // Constraint: t0 = Int
        let constraints = vec![Constraint::Equal(
            Ty::Var(TypeVarId(0)),
            Ty::int(),
            Span { start: 0, end: 0 },
        )];

        let subst = solver.solve(constraints).unwrap();

        // t0 should map to Int
        let t0 = Ty::Var(TypeVarId(0));
        assert_eq!(subst.apply(&t0).unwrap(), Ty::int());
    }

    #[test]
    fn solve_transitive() {
        let mut solver = Solver::new();

        // Constraints: t0 = t1, t1 = Int
        let constraints = vec![
            Constraint::Equal(
                Ty::Var(TypeVarId(0)),
                Ty::Var(TypeVarId(1)),
                Span { start: 0, end: 0 },
            ),
            Constraint::Equal(Ty::Var(TypeVarId(1)), Ty::int(), Span { start: 0, end: 0 }),
        ];

        let subst = solver.solve(constraints).unwrap();

        // Both t0 and t1 should resolve to Int
        assert_eq!(subst.apply(&Ty::Var(TypeVarId(0))).unwrap(), Ty::int());
        assert_eq!(subst.apply(&Ty::Var(TypeVarId(1))).unwrap(), Ty::int());
    }

    #[test]
    fn solve_fails_on_mismatch() {
        let mut solver = Solver::new();

        // Constraint: Int = Bool (impossible!)
        let constraints = vec![Constraint::Equal(
            Ty::int(),
            Ty::bool_(),
            Span { start: 0, end: 0 },
        )];

        let result = solver.solve(constraints);
        assert!(result.is_err());
    }
}
