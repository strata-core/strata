use super::ty::{Ty, TypeVarId};
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct TypeCtx {
    next_id: u32,
    env: HashMap<String, Ty>,
}

impl TypeCtx {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            env: HashMap::new(),
        }
    }

    pub fn fresh_var(&mut self) -> Ty {
        let id = TypeVarId(self.next_id);
        self.next_id += 1;
        Ty::var(id)
    }

    pub fn register(&mut self, name: impl Into<String>, ty: Ty) {
        self.env.insert(name.into(), ty);
    }

    pub fn lookup(&self, name: &str) -> Option<&Ty> {
        self.env.get(name)
    }
}
