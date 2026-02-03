use super::ty::{Ty, TypeVarId};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Subst {
    map: HashMap<TypeVarId, Ty>,
}

impl Subst {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn get(&self, v: &TypeVarId) -> Option<&Ty> {
        self.map.get(v)
    }
    pub fn insert(&mut self, v: TypeVarId, t: Ty) {
        self.map.insert(v, t);
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn apply(&self, t: &Ty) -> Ty {
        match t {
            Ty::Var(v) => {
                if let Some(ty) = self.map.get(v) {
                    self.apply(ty) // Recursively chase substitutions!
                } else {
                    Ty::Var(*v)
                }
            }
            Ty::Const(_) | Ty::Never => t.clone(),
            Ty::Arrow(params, ret) => {
                let new_params = params.iter().map(|p| self.apply(p)).collect();
                Ty::arrow(new_params, self.apply(ret))
            }
            Ty::Tuple(xs) => Ty::tuple(xs.iter().map(|x| self.apply(x)).collect::<Vec<_>>()),
            Ty::List(x) => Ty::list(self.apply(x)),
        }
    }

    /// self ∘ other (apply `other` first, then `self`)
    pub fn compose(&self, other: &Subst) -> Subst {
        let mut out = Subst::new();
        for (v, t) in other.map.iter() {
            out.insert(*v, self.apply(t));
        }
        for (v, t) in self.map.iter() {
            if !other.map.contains_key(v) {
                out.insert(*v, t.clone());
            }
        }
        out
    }
}
