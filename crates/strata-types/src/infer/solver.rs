//! Constraint solver
//!
//! Takes a set of constraints and solves them via unification.

use super::subst::Subst;
use super::ty::Constraint;
use super::unifier::{TypeError, Unifier};

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

    /// Solve a set of constraints
    ///
    /// Returns the resulting substitution, or a type error if solving fails.
    pub fn solve(&mut self, constraints: Vec<Constraint>) -> Result<Subst, TypeError> {
        for constraint in constraints {
            match constraint {
                Constraint::Equal(t1, t2, _span) => {
                    self.unifier.unify(&t1, &t2)?;
                }
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
        assert_eq!(subst.apply(&t0), Ty::int());
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
        assert_eq!(subst.apply(&Ty::Var(TypeVarId(0))), Ty::int());
        assert_eq!(subst.apply(&Ty::Var(TypeVarId(1))), Ty::int());
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
