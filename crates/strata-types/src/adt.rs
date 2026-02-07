//! Algebraic Data Type (ADT) definitions and registry.
//!
//! This module provides the infrastructure for struct and enum type definitions,
//! including a registry for looking up ADT metadata during type checking.

use crate::infer::ty::{Ty, TypeVarId};
use std::collections::HashMap;

/// Definition of an algebraic data type (struct or enum)
#[derive(Clone, Debug)]
pub struct AdtDef {
    /// Name of the ADT (e.g., "Option", "Point")
    pub name: String,
    /// Type parameters (e.g., ["T"] for Option<T>)
    pub type_params: Vec<String>,
    /// Kind of ADT (struct or enum)
    pub kind: AdtKind,
}

/// Kind of ADT: struct with fields or enum with variants
#[derive(Clone, Debug)]
pub enum AdtKind {
    /// Struct with named fields
    Struct(Vec<FieldDef>),
    /// Enum with variants
    Enum(Vec<VariantDef>),
}

/// Field definition in a struct
#[derive(Clone, Debug)]
pub struct FieldDef {
    /// Field name
    pub name: String,
    /// Field type (may reference type parameters)
    pub ty: Ty,
}

/// Variant definition in an enum
#[derive(Clone, Debug)]
pub struct VariantDef {
    /// Variant name (e.g., "Some", "None")
    pub name: String,
    /// Variant fields
    pub fields: VariantFields,
}

/// Fields of an enum variant
#[derive(Clone, Debug)]
pub enum VariantFields {
    /// Unit variant (no data): `None`
    Unit,
    /// Tuple variant with positional fields: `Some(T)`
    Tuple(Vec<Ty>),
}

impl AdtDef {
    /// Create a new struct definition
    pub fn new_struct(
        name: impl Into<String>,
        type_params: Vec<String>,
        fields: Vec<FieldDef>,
    ) -> Self {
        Self {
            name: name.into(),
            type_params,
            kind: AdtKind::Struct(fields),
        }
    }

    /// Create a new enum definition
    pub fn new_enum(
        name: impl Into<String>,
        type_params: Vec<String>,
        variants: Vec<VariantDef>,
    ) -> Self {
        Self {
            name: name.into(),
            type_params,
            kind: AdtKind::Enum(variants),
        }
    }

    /// Get the number of type parameters
    pub fn arity(&self) -> usize {
        self.type_params.len()
    }

    /// Check if this is a struct
    pub fn is_struct(&self) -> bool {
        matches!(self.kind, AdtKind::Struct(_))
    }

    /// Check if this is an enum
    pub fn is_enum(&self) -> bool {
        matches!(self.kind, AdtKind::Enum(_))
    }

    /// Get struct fields (if this is a struct)
    pub fn fields(&self) -> Option<&[FieldDef]> {
        match &self.kind {
            AdtKind::Struct(fields) => Some(fields),
            AdtKind::Enum(_) => None,
        }
    }

    /// Get enum variants (if this is an enum)
    pub fn variants(&self) -> Option<&[VariantDef]> {
        match &self.kind {
            AdtKind::Struct(_) => None,
            AdtKind::Enum(variants) => Some(variants),
        }
    }

    /// Find a variant by name (for enums)
    pub fn find_variant(&self, name: &str) -> Option<&VariantDef> {
        self.variants()?.iter().find(|v| v.name == name)
    }
}

impl VariantDef {
    /// Create a unit variant
    pub fn unit(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: VariantFields::Unit,
        }
    }

    /// Create a tuple variant
    pub fn tuple(name: impl Into<String>, fields: Vec<Ty>) -> Self {
        Self {
            name: name.into(),
            fields: VariantFields::Tuple(fields),
        }
    }

    /// Get the arity of this variant (number of fields)
    pub fn arity(&self) -> usize {
        match &self.fields {
            VariantFields::Unit => 0,
            VariantFields::Tuple(tys) => tys.len(),
        }
    }
}

/// Registry of all ADT definitions
#[derive(Clone, Debug, Default)]
pub struct AdtRegistry {
    /// Map from ADT name to definition
    adts: HashMap<String, AdtDef>,
}

impl AdtRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            adts: HashMap::new(),
        }
    }

    /// Create a registry with built-in types (Tuple2..Tuple8)
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        reg.register_builtins();
        reg
    }

    /// Register built-in tuple types
    fn register_builtins(&mut self) {
        // Register Tuple2 through Tuple8
        for n in 2..=8 {
            let name = format!("Tuple{}", n);
            let type_params: Vec<String> = (0..n).map(|i| format!("T{}", i)).collect();

            // Fields are _0, _1, etc. with corresponding type params
            let fields: Vec<FieldDef> = (0..n)
                .map(|i| FieldDef {
                    name: format!("_{}", i),
                    ty: Ty::Var(TypeVarId(i as u32)),
                })
                .collect();

            let def = AdtDef::new_struct(name, type_params, fields);
            // Safe to unwrap since we're registering fresh names
            let _ = self.register(def);
        }
    }

    /// Register an ADT definition
    ///
    /// Returns an error if an ADT with the same name already exists.
    pub fn register(&mut self, def: AdtDef) -> Result<(), String> {
        if self.adts.contains_key(&def.name) {
            return Err(format!("Duplicate type definition: {}", def.name));
        }
        self.adts.insert(def.name.clone(), def);
        Ok(())
    }

    /// Look up an ADT by name
    pub fn get(&self, name: &str) -> Option<&AdtDef> {
        self.adts.get(name)
    }

    /// Check if an ADT with the given name exists
    pub fn contains(&self, name: &str) -> bool {
        self.adts.contains_key(name)
    }

    /// Get all registered ADT names
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.adts.keys().map(|s| s.as_str())
    }

    /// Get the number of registered ADTs
    pub fn len(&self) -> usize {
        self.adts.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.adts.is_empty()
    }
}

/// Check if a type name is a capability type.
/// Delegates to `CapKind::from_name` for the canonical check.
pub fn is_capability_type(name: &str) -> bool {
    crate::effects::CapKind::from_name(name).is_some()
}

/// Check if a type contains any capability types.
/// Used to enforce the "no caps in ADTs" rule.
pub fn contains_capability(ty: &Ty) -> bool {
    match ty {
        Ty::Const(_) | Ty::Var(_) | Ty::Never => false,
        Ty::Cap(_) => true,
        Ty::Adt { name, args } => is_capability_type(name) || args.iter().any(contains_capability),
        Ty::Arrow(params, ret, _eff) => {
            params.iter().any(contains_capability) || contains_capability(ret)
        }
        Ty::Tuple(tys) => tys.iter().any(contains_capability),
        Ty::List(ty) => contains_capability(ty),
        Ty::Ref(inner) => contains_capability(inner),
    }
}

/// Find the name of the first capability type in a type tree.
/// Returns None if no capability type is found.
pub fn find_capability_name(ty: &Ty) -> Option<String> {
    match ty {
        Ty::Cap(kind) => Some(kind.type_name().to_string()),
        Ty::Adt { name, args } => {
            if is_capability_type(name) {
                Some(name.clone())
            } else {
                args.iter().find_map(find_capability_name)
            }
        }
        Ty::Arrow(params, ret, _eff) => params
            .iter()
            .find_map(find_capability_name)
            .or_else(|| find_capability_name(ret)),
        Ty::Tuple(tys) => tys.iter().find_map(find_capability_name),
        Ty::List(ty) => find_capability_name(ty),
        Ty::Ref(inner) => find_capability_name(inner),
        Ty::Const(_) | Ty::Var(_) | Ty::Never => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adt_def_struct() {
        let def = AdtDef::new_struct(
            "Point",
            vec![],
            vec![
                FieldDef {
                    name: "x".into(),
                    ty: Ty::int(),
                },
                FieldDef {
                    name: "y".into(),
                    ty: Ty::int(),
                },
            ],
        );
        assert_eq!(def.name, "Point");
        assert!(def.is_struct());
        assert!(!def.is_enum());
        assert_eq!(def.arity(), 0);
        assert_eq!(def.fields().unwrap().len(), 2);
    }

    #[test]
    fn test_adt_def_enum() {
        let def = AdtDef::new_enum(
            "Option",
            vec!["T".into()],
            vec![
                VariantDef::tuple("Some", vec![Ty::Var(TypeVarId(0))]),
                VariantDef::unit("None"),
            ],
        );
        assert_eq!(def.name, "Option");
        assert!(!def.is_struct());
        assert!(def.is_enum());
        assert_eq!(def.arity(), 1);
        assert_eq!(def.variants().unwrap().len(), 2);
        assert!(def.find_variant("Some").is_some());
        assert!(def.find_variant("None").is_some());
        assert!(def.find_variant("Missing").is_none());
    }

    #[test]
    fn test_variant_arity() {
        let unit = VariantDef::unit("None");
        let tuple = VariantDef::tuple("Some", vec![Ty::int()]);
        let tuple2 = VariantDef::tuple("Pair", vec![Ty::int(), Ty::string()]);

        assert_eq!(unit.arity(), 0);
        assert_eq!(tuple.arity(), 1);
        assert_eq!(tuple2.arity(), 2);
    }

    #[test]
    fn test_registry_register() {
        let mut reg = AdtRegistry::new();

        let def = AdtDef::new_struct("Point", vec![], vec![]);
        assert!(reg.register(def.clone()).is_ok());

        // Duplicate should fail
        assert!(reg.register(def).is_err());
    }

    #[test]
    fn test_registry_lookup() {
        let mut reg = AdtRegistry::new();
        reg.register(AdtDef::new_struct("Point", vec![], vec![]))
            .unwrap();

        assert!(reg.get("Point").is_some());
        assert!(reg.get("Missing").is_none());
        assert!(reg.contains("Point"));
        assert!(!reg.contains("Missing"));
    }

    #[test]
    fn test_registry_builtins() {
        let reg = AdtRegistry::with_builtins();

        // Should have Tuple2 through Tuple8
        assert!(reg.contains("Tuple2"));
        assert!(reg.contains("Tuple3"));
        assert!(reg.contains("Tuple4"));
        assert!(reg.contains("Tuple5"));
        assert!(reg.contains("Tuple6"));
        assert!(reg.contains("Tuple7"));
        assert!(reg.contains("Tuple8"));
        assert!(!reg.contains("Tuple1"));
        assert!(!reg.contains("Tuple9"));

        // Check Tuple2 structure
        let tuple2 = reg.get("Tuple2").unwrap();
        assert_eq!(tuple2.arity(), 2);
        assert!(tuple2.is_struct());
        let fields = tuple2.fields().unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "_0");
        assert_eq!(fields[1].name, "_1");
    }

    #[test]
    fn test_is_capability_type() {
        assert!(is_capability_type("NetCap"));
        assert!(is_capability_type("FsCap"));
        assert!(is_capability_type("TimeCap"));
        assert!(is_capability_type("RandCap"));
        assert!(is_capability_type("AiCap"));
        assert!(!is_capability_type("Int"));
        assert!(!is_capability_type("Option"));
    }

    #[test]
    fn test_contains_capability() {
        // Simple types don't contain caps
        assert!(!contains_capability(&Ty::int()));
        assert!(!contains_capability(&Ty::string()));

        // Cap types contain caps
        assert!(contains_capability(&Ty::adt0("NetCap")));

        // ADT with cap arg contains caps
        assert!(contains_capability(&Ty::adt(
            "Option",
            vec![Ty::adt0("FsCap")]
        )));

        // Nested ADT without caps is fine
        assert!(!contains_capability(&Ty::adt("Option", vec![Ty::int()])));

        // Tuple with cap contains caps
        assert!(contains_capability(&Ty::tuple(vec![
            Ty::int(),
            Ty::adt0("TimeCap")
        ])));

        // Function type with cap contains caps
        assert!(contains_capability(&Ty::arrow(
            vec![Ty::adt0("NetCap")],
            Ty::unit()
        )));
    }
}
