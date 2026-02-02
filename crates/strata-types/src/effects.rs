//! Effect algebra for Strata.
//!
//! Internally this uses a `u64` bitmask. Up to 64 distinct effects
//! can be modeled without reallocations.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Effect {
    Io = 0,
    Fs = 1,
    Net = 2,
    Time = 3,
    Random = 4,
    Ffi = 5,
    Panic = 6,
    Unwind = 7,
    // Add more as needed; keep < 64 without changing representation.
}

impl Effect {
    #[inline]
    fn bit(self) -> u64 {
        1u64 << (self as u8)
    }
}

/// A small set of `Effect`s with subset/union ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct EffectRow {
    mask: u64,
}

impl EffectRow {
    /// Empty set.
    pub const fn empty() -> Self {
        Self { mask: 0 }
    }

    /// Singleton set.
    pub const fn singleton(e: Effect) -> Self {
        Self {
            mask: 1u64 << (e as u8),
        }
    }

    /// Does the row contain the given effect?
    #[inline]
    pub fn contains(&self, e: Effect) -> bool {
        (self.mask & e.bit()) != 0
    }

    /// Insert an effect (mutating).
    #[inline]
    pub fn insert(&mut self, e: Effect) {
        self.mask |= e.bit();
    }

    /// Set union.
    #[inline]
    pub fn union(self, other: Self) -> Self {
        Self {
            mask: self.mask | other.mask,
        }
    }

    /// Subset check: is `self` ⊆ `other`?
    #[inline]
    pub fn is_subset_of(&self, other: &Self) -> bool {
        (self.mask | other.mask) == other.mask
    }

    /// True if set is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.mask == 0
    }

    /// Iterate effects present in this row.
    pub fn iter(&self) -> impl Iterator<Item = Effect> {
        // Small static list is fine; there are < 64 possible slots.
        const ALL: &[Effect] = &[
            Effect::Io,
            Effect::Fs,
            Effect::Net,
            Effect::Time,
            Effect::Random,
            Effect::Ffi,
            Effect::Panic,
            Effect::Unwind,
        ];
        let mask = self.mask;
        ALL.iter().copied().filter(move |e| (mask & e.bit()) != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_subset() {
        let a = EffectRow::empty();
        let b = EffectRow::singleton(Effect::Io);
        assert!(a.is_subset_of(&b));
        assert!(a.is_subset_of(&a));
    }

    #[test]
    fn union_and_contains() {
        let a = EffectRow::singleton(Effect::Io);
        let b = EffectRow::singleton(Effect::Net);
        let u = a.union(b);
        assert!(u.contains(Effect::Io));
        assert!(u.contains(Effect::Net));
        assert!(!u.contains(Effect::Fs));
    }

    #[test]
    fn subset_logic() {
        let mut a = EffectRow::empty();
        a.insert(Effect::Io);
        a.insert(Effect::Fs);

        let mut b = EffectRow::empty();
        b.insert(Effect::Io);
        b.insert(Effect::Fs);
        b.insert(Effect::Net);

        assert!(a.is_subset_of(&b));
        assert!(!b.is_subset_of(&a));
    }

    #[test]
    fn iter_lists_present_effects() {
        let mut r = EffectRow::empty();
        r.insert(Effect::Random);
        r.insert(Effect::Time);
        let got = r.iter().collect::<Vec<_>>();
        assert!(got.contains(&Effect::Random));
        assert!(got.contains(&Effect::Time));
        assert_eq!(got.len(), 2);
    }
}
