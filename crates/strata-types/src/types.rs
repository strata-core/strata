//! Core `Type` definitions for Strata.

use crate::EffectRow;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimType {
    Unit,
    Bool,
    I64,
    F64,
    Str,
    // Extend as needed
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Primitive types.
    Prim(PrimType),

    /// Tuple types.
    Tuple(Vec<Type>),

    /// Nominal type (e.g., user-defined ADT).
    Nominal(String),

    /// Capability nominal (marker) type. No behavior in Issue 002.
    Capability(String),

    /// Function type: `(params) -> ret ! effects`
    Fun {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: EffectRow,
    },
}

impl Type {
    pub fn fun(params: Vec<Type>, ret: Type, effects: EffectRow) -> Self {
        Type::Fun {
            params,
            ret: Box::new(ret),
            effects,
        }
    }

    pub fn unit() -> Self {
        Type::Prim(PrimType::Unit)
    }
    pub fn bool() -> Self {
        Type::Prim(PrimType::Bool)
    }
    pub fn i64() -> Self {
        Type::Prim(PrimType::I64)
    }
    pub fn f64() -> Self {
        Type::Prim(PrimType::F64)
    }
    pub fn str_() -> Self {
        Type::Prim(PrimType::Str)
    }
}
