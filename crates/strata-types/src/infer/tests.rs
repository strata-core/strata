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

/// Test that Ty::Never displays correctly
#[test]
fn test_never_display() {
    assert_eq!(format!("{}", Ty::Never), "!");
}

/// Test that Never does NOT unify with Int (soundness fix).
/// Divergence is handled in inference, not unification.
#[test]
fn test_never_does_not_unify_with_int() {
    let mut u = Unifier::new();
    assert!(u.unify(&Ty::Never, &Ty::int()).is_err());
}

/// Test that Int does NOT unify with Never (soundness fix).
#[test]
fn test_int_does_not_unify_with_never() {
    let mut u = Unifier::new();
    assert!(u.unify(&Ty::int(), &Ty::Never).is_err());
}

/// Test that Never CAN unify with type variables (var case matches first).
/// The soundness fix is in inference (infer_if), not unification.
/// When Never unifies with a var, the var gets bound to Never.
#[test]
fn test_never_unifies_with_var_binds_to_never() {
    let mut u = Unifier::new();
    let v = Ty::var(TypeVarId(0));
    u.unify(&Ty::Never, &v).unwrap();
    // The variable gets substituted to Never
    assert_eq!(u.subst().apply(&v), Ty::Never);
}

/// Test that Never does NOT unify with complex types (soundness fix).
#[test]
fn test_never_does_not_unify_with_function() {
    let mut u = Unifier::new();
    let fn_ty = Ty::arrow(vec![Ty::int()], Ty::bool_());
    assert!(u.unify(&Ty::Never, &fn_ty).is_err());
}

/// Test that Never unifies with Never (trivially)
#[test]
fn test_never_unifies_with_never() {
    let mut u = Unifier::new();
    u.unify(&Ty::Never, &Ty::Never).unwrap();
}
