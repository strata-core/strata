#![deny(unused_must_use)]
#![warn(clippy::dbg_macro, clippy::todo, clippy::unimplemented)]
#![forbid(unsafe_code)]

pub mod adt;
mod checker;
mod effects;
pub mod exhaustive;
pub mod move_check;
mod profile;
mod types;

#[cfg(test)]
mod checker_tests;

pub use checker::{TypeChecker, TypeError};
pub use effects::{CapKind, Effect, EffectRow, EffectVarId};
pub use profile::Profile;
pub use types::{PrimType, Type};

// inference lives in its own namespace (no collisions)
pub mod infer {
    pub mod constraint;
    pub mod ctx;
    pub mod solver;
    pub mod subst;
    pub mod ty;
    pub mod unifier;

    // Re-exports for convenience inside `infer`
    pub use constraint::InferCtx;
    pub use ctx::TypeCtx;
    pub use solver::Solver;
    pub use subst::Subst;
    pub use ty::{Kind, Ty, TyConst, TypeVarId};
    pub use unifier::{TypeError, Unifier};

    #[cfg(test)]
    mod tests;
}

// Ergonomic prelude -- short names in dependents.
// Remove this block if you prefer explicit paths like `strata_types::infer::Ty`.
pub mod prelude {
    // keep your nominal types visible
    pub use crate::types::{PrimType, Type};

    // inference API under clear names to avoid confusion
    pub use crate::infer::{
        constraint::InferCtx,
        ctx::TypeCtx,
        solver::Solver,
        subst::Subst,
        ty::{Ty as InferType, TyConst as InferConst, TypeVarId},
        unifier::{TypeError, Unifier},
    };
}
