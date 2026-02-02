use super::ty::TyConst;

use super::{
    // ctx::TypeCtx,
    // subst::Subst,
    ty::{Ty, TypeVarId},
    unifier::{TypeError, Unifier},
};

#[test]
fn unify_tuples_same_arity_elementwise() {
    let mut u = Unifier::new();
    let a = Ty::tuple(vec![Ty::int(), Ty::bool_()]);
    let b = Ty::tuple(vec![Ty::int(), Ty::bool_()]);
    u.unify(&a, &b).unwrap();
}

#[test]
fn unify_tuples_mismatched_arity_errors() {
    let mut u = Unifier::new();
    let a = Ty::tuple(vec![Ty::int(), Ty::bool_()]);
    let b = Ty::tuple(vec![Ty::int()]);
    let err = u.unify(&a, &b).unwrap_err();
    assert!(matches!(err, TypeError::Arity { .. }));
}

#[test]
fn unify_tuples_propagates_vars() {
    let mut u = Unifier::new();
    let v0 = Ty::var(TypeVarId(0));
    let v1 = Ty::var(TypeVarId(1));
    let a = Ty::tuple(vec![v0.clone(), Ty::int(), v1.clone()]);
    let b = Ty::tuple(vec![Ty::bool_(), Ty::int(), Ty::bool_()]);
    u.unify(&a, &b).unwrap();

    let s = u.subst();
    assert_eq!(s.apply(&v0), Ty::bool_());
    assert_eq!(s.apply(&v1), Ty::bool_());
}

#[test]
fn unify_lists_element_type() {
    let mut u = Unifier::new();
    let x = Ty::list(Ty::var(TypeVarId(0)));
    let y = Ty::list(Ty::int());
    u.unify(&x, &y).unwrap();
    let t0 = u.subst().apply(&Ty::var(TypeVarId(0)));
    assert_eq!(t0, Ty::int());
}

#[test]
fn occurs_in_list_blocks_infinite() {
    let mut u = Unifier::new();
    let v = Ty::var(TypeVarId(0));
    let list_v = Ty::list(v.clone());
    let res = u.unify(&v, &list_v);
    assert!(matches!(res, Err(TypeError::Occurs { .. })));
}

#[test]
fn unify_tuple_of_list_with_vars() {
    use super::ty::TypeVarId as V;
    let mut u = Unifier::new();
    let a = Ty::tuple(vec![Ty::list(Ty::var(V(0))), Ty::int()]);
    let b = Ty::tuple(vec![Ty::list(Ty::bool_()), Ty::var(V(1))]);
    u.unify(&a, &b).unwrap();
    assert_eq!(u.subst().apply(&Ty::var(V(0))), Ty::bool_());
    assert_eq!(u.subst().apply(&Ty::var(V(1))), Ty::int());
}

#[test]
fn test_new_type_constants() {
    let float_ty = Ty::float();
    let string_ty = Ty::string();

    assert_eq!(format!("{}", float_ty), "Float");
    assert_eq!(format!("{}", string_ty), "String");

    // Verify they're actually TyConst variants
    match float_ty {
        Ty::Const(TyConst::Float) => {}
        _ => panic!("Expected Float constant"),
    }

    match string_ty {
        Ty::Const(TyConst::String) => {}
        _ => panic!("Expected String constant"),
    }
}
