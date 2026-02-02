//! Build/profile awareness at the types layer (no enforcement yet).
//!
//! This allows frontends or later phases to tag contexts with a
//! profile. Enforcement will be added in later issues.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    /// Safety-first, restricted effects.
    Safe,
    /// Full system capabilities available.
    System,
    /// Test builds; may allow extra effects/fixtures.
    Test,
    /// Constrained embedded environment.
    Embedded,
    /// No stdlib available; affects capability & effect availability.
    NoStd,
}
