//! Effect algebra for Strata.
//!
//! Internally this uses a `u64` bitmask for the concrete (known) effects.
//! Up to 64 distinct effects can be modeled without reallocations.
//!
//! An `EffectRow` may be *closed* (no tail variable — all effects are known)
//! or *open* (has a tail `EffectVarId` representing unknown additional effects).

use std::fmt;

/// Unique identifier for an effect-row variable (analogous to `TypeVarId`).
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EffectVarId(pub u32);

impl fmt::Debug for EffectVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}", self.0)
    }
}

impl fmt::Display for EffectVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Effect {
    Fs = 0,
    Net = 1,
    Time = 2,
    Rand = 3,
    Ai = 4,
    // Add more as needed; keep < 64 without changing representation.
}

impl Effect {
    #[inline]
    pub fn bit(self) -> u64 {
        1u64 << (self as u8)
    }
}

/// All known effects, in discriminant order.
pub const ALL_EFFECTS: &[Effect] = &[
    Effect::Fs,
    Effect::Net,
    Effect::Time,
    Effect::Rand,
    Effect::Ai,
];

/// Capability kind — each capability gates exactly one effect.
///
/// Capabilities are first-class types in Strata's type system (`Ty::Cap`).
/// A function that performs a concrete effect must have the corresponding
/// capability type in its parameter list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapKind {
    Fs,
    Net,
    Time,
    Rand,
    Ai,
}

impl CapKind {
    /// Which effect does this capability gate?
    pub fn gates_effect(self) -> Effect {
        match self {
            CapKind::Fs => Effect::Fs,
            CapKind::Net => Effect::Net,
            CapKind::Time => Effect::Time,
            CapKind::Rand => Effect::Rand,
            CapKind::Ai => Effect::Ai,
        }
    }

    /// Reverse mapping: which capability gates this effect?
    pub fn from_effect(e: Effect) -> CapKind {
        match e {
            Effect::Fs => CapKind::Fs,
            Effect::Net => CapKind::Net,
            Effect::Time => CapKind::Time,
            Effect::Rand => CapKind::Rand,
            Effect::Ai => CapKind::Ai,
        }
    }

    /// Parse a capability type name (e.g., "FsCap") to a CapKind.
    pub fn from_name(name: &str) -> Option<CapKind> {
        match name {
            "FsCap" => Some(CapKind::Fs),
            "NetCap" => Some(CapKind::Net),
            "TimeCap" => Some(CapKind::Time),
            "RandCap" => Some(CapKind::Rand),
            "AiCap" => Some(CapKind::Ai),
            _ => None,
        }
    }

    /// Display name of the capability type.
    pub fn type_name(self) -> &'static str {
        match self {
            CapKind::Fs => "FsCap",
            CapKind::Net => "NetCap",
            CapKind::Time => "TimeCap",
            CapKind::Rand => "RandCap",
            CapKind::Ai => "AiCap",
        }
    }
}

/// A row of effects, optionally open (with a tail variable).
///
/// - Closed row: `{ Fs, Net }` — `concrete = 0b011, tail = None`
/// - Open row: `{ Fs } ∪ e0` — `concrete = 0b001, tail = Some(EffectVarId(0))`
/// - Pure (closed empty): `concrete = 0, tail = None`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EffectRow {
    /// Bitmask of known-present effects.
    pub concrete: u64,
    /// If `Some`, this row is *open*: the tail variable may stand for more effects.
    pub tail: Option<EffectVarId>,
}

impl fmt::Debug for EffectRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Default for EffectRow {
    fn default() -> Self {
        Self::pure()
    }
}

impl EffectRow {
    // ---- Constructors ----

    /// Empty, closed row (pure — no effects).
    pub const fn pure() -> Self {
        Self {
            concrete: 0,
            tail: None,
        }
    }

    /// Closed row with the given bitmask.
    pub const fn closed(mask: u64) -> Self {
        Self {
            concrete: mask,
            tail: None,
        }
    }

    /// Open row: known effects + a tail variable.
    pub const fn open(concrete: u64, tail: EffectVarId) -> Self {
        Self {
            concrete,
            tail: Some(tail),
        }
    }

    /// Singleton closed row.
    pub const fn singleton(e: Effect) -> Self {
        Self {
            concrete: 1u64 << (e as u8),
            tail: None,
        }
    }

    // ---- Queries ----

    /// True if the row is closed (no tail variable).
    #[inline]
    pub fn is_closed(&self) -> bool {
        self.tail.is_none()
    }

    /// True if the row is empty **and** closed.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.concrete == 0 && self.tail.is_none()
    }

    /// Does the concrete part contain the given effect?
    #[inline]
    pub fn contains(&self, e: Effect) -> bool {
        (self.concrete & e.bit()) != 0
    }

    /// Insert an effect into the concrete part (mutating).
    #[inline]
    pub fn insert(&mut self, e: Effect) {
        self.concrete |= e.bit();
    }

    // ---- Algebra (closed rows only) ----

    /// Set union of two **closed** rows.
    ///
    /// # Panics
    /// Panics if either row has a tail (caller must resolve first).
    pub fn union(self, other: Self) -> Self {
        assert!(
            self.is_closed() && other.is_closed(),
            "EffectRow::union requires closed rows"
        );
        Self {
            concrete: self.concrete | other.concrete,
            tail: None,
        }
    }

    /// Subset check for **closed** rows: is `self ⊆ other`?
    ///
    /// # Panics
    /// Panics if either row has a tail.
    pub fn is_subset_of(&self, other: &Self) -> bool {
        assert!(
            self.is_closed() && other.is_closed(),
            "EffectRow::is_subset_of requires closed rows"
        );
        (self.concrete | other.concrete) == other.concrete
    }

    /// Iterate effects present in the concrete part.
    pub fn iter(&self) -> impl Iterator<Item = Effect> {
        let mask = self.concrete;
        ALL_EFFECTS
            .iter()
            .copied()
            .filter(move |e| (mask & e.bit()) != 0)
    }
}

impl fmt::Display for EffectRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for e in ALL_EFFECTS {
            if (self.concrete & e.bit()) != 0 {
                if !first {
                    write!(f, ", ")?;
                }
                first = false;
                write!(f, "{:?}", e)?;
            }
        }
        if let Some(tail) = &self.tail {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", tail)?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_subset() {
        let a = EffectRow::pure();
        let b = EffectRow::singleton(Effect::Fs);
        assert!(a.is_subset_of(&b));
        assert!(a.is_subset_of(&a));
    }

    #[test]
    fn union_and_contains() {
        let a = EffectRow::singleton(Effect::Fs);
        let b = EffectRow::singleton(Effect::Net);
        let u = a.union(b);
        assert!(u.contains(Effect::Fs));
        assert!(u.contains(Effect::Net));
        assert!(!u.contains(Effect::Time));
    }

    #[test]
    fn subset_logic() {
        let mut a = EffectRow::pure();
        a.insert(Effect::Fs);
        a.insert(Effect::Net);

        let mut b = EffectRow::pure();
        b.insert(Effect::Fs);
        b.insert(Effect::Net);
        b.insert(Effect::Time);

        assert!(a.is_subset_of(&b));
        assert!(!b.is_subset_of(&a));
    }

    #[test]
    fn iter_lists_present_effects() {
        let mut r = EffectRow::pure();
        r.insert(Effect::Rand);
        r.insert(Effect::Time);
        let got = r.iter().collect::<Vec<_>>();
        assert!(got.contains(&Effect::Rand));
        assert!(got.contains(&Effect::Time));
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn effect_var_id_display() {
        assert_eq!(format!("{}", EffectVarId(0)), "e0");
        assert_eq!(format!("{}", EffectVarId(42)), "e42");
    }

    #[test]
    fn pure_is_empty_and_closed() {
        let p = EffectRow::pure();
        assert!(p.is_empty());
        assert!(p.is_closed());
    }

    #[test]
    fn open_row_is_not_closed() {
        let o = EffectRow::open(0, EffectVarId(0));
        assert!(!o.is_closed());
        assert!(!o.is_empty());
    }

    #[test]
    fn display_closed() {
        let r = EffectRow::closed(Effect::Fs.bit() | Effect::Net.bit());
        assert_eq!(format!("{}", r), "{Fs, Net}");
    }

    #[test]
    fn display_open() {
        let r = EffectRow::open(Effect::Fs.bit(), EffectVarId(3));
        assert_eq!(format!("{}", r), "{Fs, e3}");
    }

    #[test]
    fn display_pure() {
        assert_eq!(format!("{}", EffectRow::pure()), "{}");
    }
}
