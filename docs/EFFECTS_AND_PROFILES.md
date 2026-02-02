// crates/strata-types/src/effects.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect { Net, Fs, Time, Rand, Fail /* parameterization later */ }

pub struct EffectRow(/* canonical set */);

impl EffectRow {
    pub fn is_subset_of(&self, other: &Self) -> bool { /* … */ }
    pub fn union(&self, other: &Self) -> Self { /* … */ }
}

// crates/strata-types/src/profiles.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile { Kernel, Realtime, General }

impl Profile {
    pub fn allowed_effects(&self) -> EffectRow { /* … */ }
}