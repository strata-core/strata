use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeVarId(pub u32);

impl fmt::Debug for TypeVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "t{}", self.0) }
}
impl Hash for TypeVarId {
    fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state); }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum TyConst { Unit, Bool, Int }

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ty {
    Var(TypeVarId),
    Const(TyConst),
    Arrow(Box<Ty>, Box<Ty>),
}

impl Ty {
    pub fn var(id: TypeVarId) -> Self { Ty::Var(id) }
    pub fn unit() -> Self { Ty::Const(TyConst::Unit) }
    pub fn bool_() -> Self { Ty::Const(TyConst::Bool) }
    pub fn int() -> Self { Ty::Const(TyConst::Int) }
    pub fn arrow(a: Ty, b: Ty) -> Self { Ty::Arrow(Box::new(a), Box::new(b)) }
}
