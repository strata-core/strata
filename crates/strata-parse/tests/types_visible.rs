use strata_types::{Effect, EffectRow, Type};

#[test]
fn fun_type_compiles() {
    let eff = EffectRow::singleton(Effect::Fs);
    let t = Type::fun(vec![Type::i64()], Type::unit(), eff);
    let _ = t;
}
