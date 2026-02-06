#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(clippy::dbg_macro, clippy::todo, clippy::unimplemented)]

mod lexer;
mod parser;
mod token;

pub use parser::parse_str;

#[cfg(test)]
mod infer_smoke {
    use strata_types::prelude::*;

    #[test]
    fn visibility_and_basic_unify() {
        let mut u = Unifier::new();
        let a = InferType::var(TypeVarId(0));
        let b = InferType::int();
        u.unify(&a, &b).unwrap();
        assert_eq!(u.subst().apply(&a).unwrap(), InferType::int());
    }
}
