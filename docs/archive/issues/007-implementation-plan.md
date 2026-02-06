# Issue 007: ADTs & Pattern Matching — Implementation Plan

**For:** AI-assisted implementation
**Issue:** 007  
**Estimated Time:** 10-14 days  
**Prerequisites:** v0.0.7.0 tagged, CI green, ADR-0003 approved

## Overview

Add algebraic data types (structs, enums), nominal tuples, generics, and pattern matching to Strata. This is the largest issue yet — work in phases, verify each phase before proceeding.

**Reference:** `docs/adr-0003-adts-pattern-matching.md` for all design decisions.

---

## Phase 1: AST & Parsing (2-3 days)

### Goal
Parse struct, enum, tuple, and match syntax. No type checking yet.

### 1.1 New AST Nodes in `strata-ast`

```rust
// In src/lib.rs or src/ast.rs

/// Struct definition
pub struct StructDef {
    pub name: Ident,
    pub type_params: Vec<Ident>,  // <T, U>
    pub fields: Vec<Field>,
    pub span: Span,
}

pub struct Field {
    pub name: Ident,
    pub ty: TypeExpr,
    pub span: Span,
}

/// Enum definition  
pub struct EnumDef {
    pub name: Ident,
    pub type_params: Vec<Ident>,
    pub variants: Vec<Variant>,
    pub span: Span,
}

pub struct Variant {
    pub name: Ident,
    pub fields: VariantFields,
    pub span: Span,
}

pub enum VariantFields {
    Unit,                        // None
    Tuple(Vec<TypeExpr>),        // Some(T)
    // Struct(Vec<Field>),       // DEFERRED to v0.2
}

/// Pattern AST
pub enum Pat {
    Wildcard(Span),                              // _
    Ident(Ident, Span),                          // x
    Literal(Lit, Span),                          // 0, "hello", true
    Tuple(Vec<Pat>, Span),                       // (a, b)
    Struct {                                     // Point { x, y: 0 }
        path: Path,
        fields: Vec<PatField>,
        span: Span,
    },
    Variant {                                    // Option::Some(x)
        path: Path,
        fields: Vec<Pat>,                        // tuple-style only for v0.1
        span: Span,
    },
}

pub struct PatField {
    pub name: Ident,
    pub pat: Pat,
    pub span: Span,
}

/// Match expression
pub struct MatchExpr {
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

pub struct MatchArm {
    pub pat: Pat,
    pub body: Box<Expr>,
    pub span: Span,
}

/// Path for qualified names
pub struct Path {
    pub segments: Vec<Ident>,  // ["Option", "Some"]
    pub span: Span,
}

/// Tuple expression and type
// Add to Expr enum:
//   Tuple(Vec<Expr>, Span),
// Add to TypeExpr enum:
//   Tuple(Vec<TypeExpr>, Span),

/// Update Item enum
pub enum Item {
    Fn(FnDef),
    Struct(StructDef),  // NEW
    Enum(EnumDef),      // NEW
}
```

### 1.2 Lexer Updates in `strata-parse`

New keywords:
- `struct`
- `enum`
- `match`

New tokens (if not present):
- `=>` (fat arrow)
- `::` (path separator) — already have from 006-hardening

### 1.3 Parser Updates in `strata-parse`

**Parse struct:**
```strata
struct Point { x: Int, y: Int }
struct Pair<A, B> { fst: A, snd: B }
```

**Parse enum:**
```strata
enum Option<T> {
    Some(T),
    None,
}
```

**Parse match:**
```strata
match expr {
    Pattern => body,
    Pattern => body,
}
```

**Parse tuple expressions:**
```strata
let pair = (1, "hello");
let triple = (a, b, c);
```

**Parse tuple types:**
```strata
let x: (Int, String) = ...;
fn foo() -> (Int, Bool) { ... }
```

**Parse patterns recursively:**
- Wildcard: `_`
- Ident: `x`, `_unused`
- Literal: `0`, `true`, `"hi"`
- Tuple: `(a, b)`
- Struct: `Point { x, y }`, `Point { x: 0, y }`
- Variant: `Option::Some(x)`, `Option::None`

**Pattern depth limit:** Use existing nesting guard (512).

### 1.4 Tests for Phase 1

Create `crates/strata-parse/tests/adt_parse.rs`:

```rust
#[test]
fn parse_struct_def() { ... }

#[test]
fn parse_struct_generic() { ... }

#[test]
fn parse_enum_def() { ... }

#[test]
fn parse_enum_generic() { ... }

#[test]
fn parse_match_simple() { ... }

#[test]
fn parse_match_nested_patterns() { ... }

#[test]
fn parse_tuple_expr() { ... }

#[test]
fn parse_tuple_type() { ... }

#[test]
fn parse_pattern_wildcard() { ... }

#[test]
fn parse_pattern_literal() { ... }

#[test]
fn parse_pattern_struct() { ... }

#[test]
fn parse_pattern_variant() { ... }

#[test]
fn parse_pattern_depth_limit() { ... }
```

### 1.5 Verification

```fish
cargo test -p strata-parse
cargo clippy --workspace --all-targets -- -D warnings
```

All new tests pass. Parsing works. No type checking yet.

---

## Phase 2: Type Representation (1-2 days)

### Goal
Extend `Ty` to represent ADTs, tuples, and generics.

### 2.1 Updates to `strata-types/src/infer/ty.rs`

```rust
pub enum Ty {
    // Existing...
    Var(TypeVarId),
    Unit,
    Bool,
    Int,
    Float,
    String,
    Arrow(Vec<Ty>, Box<Ty>),  // multi-param
    Never,
    
    // NEW:
    /// Named ADT (struct or enum) with type arguments
    Adt {
        name: String,
        args: Vec<Ty>,
    },
    
    /// Tuple (nominal, desugars to Tuple2<A,B> etc.)
    Tuple(Vec<Ty>),
}
```

### 2.2 ADT Registry

Create `strata-types/src/adt.rs`:

```rust
use std::collections::HashMap;

/// ADT definition stored in environment
#[derive(Clone, Debug)]
pub struct AdtDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub kind: AdtKind,
}

#[derive(Clone, Debug)]
pub enum AdtKind {
    Struct(Vec<FieldDef>),
    Enum(Vec<VariantDef>),
}

#[derive(Clone, Debug)]
pub struct FieldDef {
    pub name: String,
    pub ty: Ty,  // May contain type params as Ty::Var
}

#[derive(Clone, Debug)]
pub struct VariantDef {
    pub name: String,
    pub fields: VariantFields,
}

#[derive(Clone, Debug)]
pub enum VariantFields {
    Unit,
    Tuple(Vec<Ty>),
}

/// Registry of all ADT definitions
#[derive(Default)]
pub struct AdtRegistry {
    pub adts: HashMap<String, AdtDef>,
}

impl AdtRegistry {
    pub fn register(&mut self, def: AdtDef) -> Result<(), String> {
        if self.adts.contains_key(&def.name) {
            return Err(format!("Duplicate type definition: {}", def.name));
        }
        self.adts.insert(def.name.clone(), def);
        Ok(())
    }
    
    pub fn get(&self, name: &str) -> Option<&AdtDef> {
        self.adts.get(name)
    }
}
```

### 2.3 Built-in Tuple Types

Register `Tuple2` through `Tuple8` as built-in ADTs:

```rust
impl AdtRegistry {
    pub fn with_builtins() -> Self {
        let mut reg = Self::default();
        
        // Tuple2<A, B>
        reg.adts.insert("Tuple2".into(), AdtDef {
            name: "Tuple2".into(),
            type_params: vec!["A".into(), "B".into()],
            kind: AdtKind::Struct(vec![
                FieldDef { name: "_0".into(), ty: Ty::Var(/* A */) },
                FieldDef { name: "_1".into(), ty: Ty::Var(/* B */) },
            ]),
        });
        
        // ... Tuple3 through Tuple8
        
        reg
    }
}
```

### 2.4 Unification Updates

Update `strata-types/src/infer/unifier.rs`:

```rust
// In unify():
(Ty::Adt { name: n1, args: a1 }, Ty::Adt { name: n2, args: a2 }) => {
    if n1 != n2 {
        return Err(TypeError::Mismatch { expected: t1, found: t2 });
    }
    if a1.len() != a2.len() {
        return Err(TypeError::Arity { expected: a1.len(), found: a2.len() });
    }
    for (arg1, arg2) in a1.iter().zip(a2.iter()) {
        self.unify(arg1, arg2)?;
    }
    Ok(())
}

(Ty::Tuple(ts1), Ty::Tuple(ts2)) => {
    if ts1.len() != ts2.len() {
        return Err(TypeError::Arity { expected: ts1.len(), found: ts2.len() });
    }
    for (t1, t2) in ts1.iter().zip(ts2.iter()) {
        self.unify(t1, t2)?;
    }
    Ok(())
}
```

### 2.5 Tests for Phase 2

```rust
#[test]
fn unify_adt_same() { ... }

#[test]
fn unify_adt_different_names() { ... }

#[test]
fn unify_adt_different_args() { ... }

#[test]
fn unify_tuple_same() { ... }

#[test]
fn unify_tuple_different_arity() { ... }
```

### 2.6 Verification

```fish
cargo test -p strata-types
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Phase 3: ADT Registration & Constructors (2 days)

### Goal
Register struct/enum definitions and add constructors to environment as polymorphic functions.

### 3.1 Two-Pass Module Checking (Extend Existing)

In `strata-types/src/checker.rs`, extend the existing two-pass:

**Pass 1:** Collect function signatures AND ADT definitions
**Pass 2:** Check function bodies (constructors already available)

```rust
// Pass 1: Register ADTs
for item in &module.items {
    match item {
        Item::Struct(def) => self.register_struct(def)?,
        Item::Enum(def) => self.register_enum(def)?,
        Item::Fn(fn_def) => { /* existing signature extraction */ }
    }
}

// After ADTs registered, add constructors to env
for item in &module.items {
    if let Item::Enum(def) = item {
        self.register_enum_constructors(def)?;
    }
}

// Pass 2: Check bodies
// ... existing code ...
```

### 3.2 Register Struct

```rust
fn register_struct(&mut self, def: &StructDef) -> Result<(), TypeError> {
    // Check for caps in fields
    for field in &def.fields {
        let field_ty = self.resolve_type_expr(&field.ty)?;
        if self.contains_cap(&field_ty) {
            return Err(TypeError::CapInAdt {
                field: field.name.clone(),
                span: field.span,
            });
        }
    }
    
    // Register in ADT registry
    let adt_def = AdtDef {
        name: def.name.clone(),
        type_params: def.type_params.clone(),
        kind: AdtKind::Struct(/* ... */),
    };
    self.adt_registry.register(adt_def)?;
    
    // Struct constructor: Point { x: Int, y: Int } -> Point
    // (handled at construction site, not as function)
    
    Ok(())
}
```

### 3.3 Register Enum Constructors

```rust
fn register_enum_constructors(&mut self, def: &EnumDef) -> Result<(), TypeError> {
    for variant in &def.variants {
        // Constructor type: ∀T. (T) -> Option<T> for Some
        //                   ∀T. () -> Option<T> for None
        
        let scheme = self.build_constructor_scheme(def, variant)?;
        
        // Add to environment: "Option::Some" -> scheme
        let full_name = format!("{}::{}", def.name, variant.name);
        self.env.insert(full_name, scheme);
    }
    Ok(())
}

fn build_constructor_scheme(&self, enum_def: &EnumDef, variant: &Variant) -> TypeScheme {
    // Create fresh type vars for each type param
    let type_vars: Vec<TypeVarId> = enum_def.type_params
        .iter()
        .map(|_| self.fresh_var())
        .collect();
    
    // Build result type: Option<T>
    let result_ty = Ty::Adt {
        name: enum_def.name.clone(),
        args: type_vars.iter().map(|v| Ty::Var(*v)).collect(),
    };
    
    // Build param types based on variant fields
    let param_tys = match &variant.fields {
        VariantFields::Unit => vec![],
        VariantFields::Tuple(tys) => /* substitute type params */ ,
    };
    
    // Arrow type: (params) -> Result
    let fn_ty = Ty::Arrow(param_tys, Box::new(result_ty));
    
    TypeScheme {
        quantified: type_vars,
        ty: fn_ty,
    }
}
```

### 3.4 Caps-in-ADT Check

```rust
fn contains_cap(&self, ty: &Ty) -> bool {
    match ty {
        Ty::Adt { name, args } => {
            if is_cap_type(name) {
                return true;
            }
            args.iter().any(|a| self.contains_cap(a))
        }
        Ty::Tuple(tys) => tys.iter().any(|t| self.contains_cap(t)),
        Ty::Arrow(params, ret) => {
            params.iter().any(|p| self.contains_cap(p)) || self.contains_cap(ret)
        }
        _ => false,
    }
}

fn is_cap_type(name: &str) -> bool {
    matches!(name, "NetCap" | "FsCap" | "TimeCap" | "RandCap" | "AiCap")
}
```

### 3.5 Tests for Phase 3

```rust
#[test]
fn register_struct_simple() { ... }

#[test]
fn register_struct_generic() { ... }

#[test]
fn register_enum_simple() { ... }

#[test]
fn register_enum_generic() { ... }

#[test]
fn constructor_type_some() { ... }

#[test]
fn constructor_type_none() { ... }

#[test]
fn cap_in_struct_error() { ... }

#[test]
fn cap_in_enum_error() { ... }

#[test]
fn cap_in_generic_error() { ... }
```

### 3.6 Verification

```fish
cargo test -p strata-types
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Phase 4: Match Type Inference (2-3 days)

### Goal
Type check match expressions, infer pattern bindings.

### 4.1 Infer Match Expression

```rust
fn infer_match(&mut self, match_expr: &MatchExpr) -> Result<Ty, TypeError> {
    // Infer scrutinee type
    let scrutinee_ty = self.infer_expr(&match_expr.scrutinee)?;
    
    // Special case: matching on Never
    if scrutinee_ty == Ty::Never {
        // Empty match on Never is valid, returns Never
        return Ok(Ty::Never);
    }
    
    let mut result_ty: Option<Ty> = None;
    
    for arm in &match_expr.arms {
        // Check pattern against scrutinee type, get bindings
        let bindings = self.check_pattern(&arm.pat, &scrutinee_ty)?;
        
        // Infer arm body with bindings in scope
        let arm_ty = self.with_bindings(&bindings, |ctx| {
            ctx.infer_expr(&arm.body)
        })?;
        
        // Join arm types (Never doesn't contribute)
        if arm_ty != Ty::Never {
            match &result_ty {
                None => result_ty = Some(arm_ty),
                Some(existing) => {
                    self.unify(existing, &arm_ty)?;
                }
            }
        }
    }
    
    Ok(result_ty.unwrap_or(Ty::Never))
}
```

### 4.2 Check Pattern

```rust
struct PatternBinding {
    name: String,
    ty: Ty,
    span: Span,
}

fn check_pattern(&mut self, pat: &Pat, expected: &Ty) -> Result<Vec<PatternBinding>, TypeError> {
    match pat {
        Pat::Wildcard(_) => Ok(vec![]),
        
        Pat::Ident(name, span) => {
            // Check for duplicate bindings (handled in parent)
            Ok(vec![PatternBinding {
                name: name.clone(),
                ty: expected.clone(),
                span: *span,
            }])
        }
        
        Pat::Literal(lit, span) => {
            let lit_ty = self.infer_literal(lit)?;
            self.unify(&lit_ty, expected)?;
            Ok(vec![])
        }
        
        Pat::Tuple(pats, span) => {
            // Expected must be Tuple of same arity
            match expected {
                Ty::Tuple(tys) if tys.len() == pats.len() => {
                    let mut bindings = vec![];
                    for (pat, ty) in pats.iter().zip(tys.iter()) {
                        bindings.extend(self.check_pattern(pat, ty)?);
                    }
                    self.check_duplicate_bindings(&bindings)?;
                    Ok(bindings)
                }
                _ => Err(TypeError::PatternMismatch { ... })
            }
        }
        
        Pat::Variant { path, fields, span } => {
            // Look up constructor, instantiate, check fields
            let ctor_name = path.to_string();  // "Option::Some"
            let scheme = self.env.get(&ctor_name)?;
            let (param_tys, result_ty) = self.instantiate_constructor(scheme)?;
            
            self.unify(&result_ty, expected)?;
            
            // Check each field pattern
            let mut bindings = vec![];
            for (pat, ty) in fields.iter().zip(param_tys.iter()) {
                bindings.extend(self.check_pattern(pat, ty)?);
            }
            self.check_duplicate_bindings(&bindings)?;
            Ok(bindings)
        }
        
        Pat::Struct { path, fields, span } => {
            // Similar to variant, but with named fields
            // ...
        }
    }
}
```

### 4.3 Duplicate Binding Check

```rust
fn check_duplicate_bindings(&self, bindings: &[PatternBinding]) -> Result<(), TypeError> {
    let mut seen = HashSet::new();
    for b in bindings {
        if !seen.insert(&b.name) {
            return Err(TypeError::DuplicatePatternBinding {
                name: b.name.clone(),
                span: b.span,
            });
        }
    }
    Ok(())
}
```

### 4.4 Infer Tuple Expression

```rust
fn infer_tuple(&mut self, exprs: &[Expr]) -> Result<Ty, TypeError> {
    if exprs.len() > 8 {
        return Err(TypeError::TupleArityLimit { max: 8, found: exprs.len() });
    }
    
    let tys: Vec<Ty> = exprs
        .iter()
        .map(|e| self.infer_expr(e))
        .collect::<Result<_, _>>()?;
    
    Ok(Ty::Tuple(tys))
}
```

### 4.5 Infer Struct Construction

```rust
fn infer_struct_construction(&mut self, name: &str, fields: &[(String, Expr)]) -> Result<Ty, TypeError> {
    let adt_def = self.adt_registry.get(name)?;
    
    // Check all fields provided, no extras
    // Infer each field expression, unify with expected type
    // Return Ty::Adt { name, args }
    
    // ...
}
```

### 4.6 Tests for Phase 4

```rust
#[test]
fn infer_match_option_simple() { ... }

#[test]
fn infer_match_nested_pattern() { ... }

#[test]
fn infer_match_all_never() { ... }

#[test]
fn infer_match_on_never() { ... }

#[test]
fn infer_tuple_construction() { ... }

#[test]
fn infer_tuple_destructure() { ... }

#[test]
fn infer_struct_construction() { ... }

#[test]
fn pattern_duplicate_binding_error() { ... }

#[test]
fn pattern_type_mismatch_error() { ... }
```

### 4.7 Verification

```fish
cargo test -p strata-types
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Phase 5: Exhaustiveness Checking (2-3 days)

### Goal
Implement Maranget's algorithm for exhaustiveness and redundancy detection.

### 5.1 Pattern Matrix Representation

Create `strata-types/src/exhaustive.rs`:

```rust
/// A pattern matrix for exhaustiveness checking
pub struct PatternMatrix {
    rows: Vec<PatternRow>,
    col_types: Vec<Ty>,
}

pub struct PatternRow {
    patterns: Vec<SimplifiedPat>,
    arm_index: usize,
}

/// Simplified pattern for matrix operations
pub enum SimplifiedPat {
    Wildcard,
    Constructor {
        name: String,
        args: Vec<SimplifiedPat>,
    },
    Literal(Lit),
}
```

### 5.2 Maranget Algorithm

```rust
impl PatternMatrix {
    /// Check if the matrix is exhaustive
    /// Returns None if exhaustive, Some(witness) if not
    pub fn check_exhaustive(&self, ctx: &CheckContext) -> Option<Vec<SimplifiedPat>> {
        if self.rows.is_empty() {
            // No rows = not exhaustive, return wildcard witness
            return Some(self.col_types.iter().map(|_| SimplifiedPat::Wildcard).collect());
        }
        
        if self.col_types.is_empty() {
            // No columns, has rows = exhaustive
            return None;
        }
        
        // Pick first column, get all constructors for its type
        let head_ty = &self.col_types[0];
        let all_ctors = ctx.constructors_for_type(head_ty);
        
        if self.is_complete_signature(&all_ctors) {
            // All constructors covered, specialize on each
            for ctor in &all_ctors {
                let specialized = self.specialize(ctor);
                if let Some(mut witness) = specialized.check_exhaustive(ctx) {
                    // Prepend constructor to witness
                    let ctor_pat = SimplifiedPat::Constructor {
                        name: ctor.name.clone(),
                        args: witness.drain(..ctor.arity).collect(),
                    };
                    witness.insert(0, ctor_pat);
                    return Some(witness);
                }
            }
            None
        } else {
            // Incomplete, check default matrix
            let default = self.default_matrix();
            if let Some(mut witness) = default.check_exhaustive(ctx) {
                // Find missing constructor
                let missing = self.find_missing_constructor(&all_ctors);
                let ctor_pat = SimplifiedPat::Constructor {
                    name: missing.name,
                    args: vec![SimplifiedPat::Wildcard; missing.arity],
                };
                witness.insert(0, ctor_pat);
                return Some(witness);
            }
            None
        }
    }
    
    /// Check for redundant rows
    pub fn check_redundant(&self, ctx: &CheckContext) -> Vec<usize> {
        let mut redundant = vec![];
        
        for i in 0..self.rows.len() {
            let prefix = PatternMatrix {
                rows: self.rows[..i].to_vec(),
                col_types: self.col_types.clone(),
            };
            
            if !prefix.is_useful(&self.rows[i], ctx) {
                redundant.push(self.rows[i].arm_index);
            }
        }
        
        redundant
    }
    
    fn specialize(&self, ctor: &Constructor) -> PatternMatrix { ... }
    fn default_matrix(&self) -> PatternMatrix { ... }
    fn is_useful(&self, row: &PatternRow, ctx: &CheckContext) -> bool { ... }
}
```

### 5.3 Integration with Type Checker

```rust
fn check_match_exhaustiveness(&mut self, match_expr: &MatchExpr, scrutinee_ty: &Ty) -> Result<(), TypeError> {
    // Build pattern matrix
    let matrix = self.build_pattern_matrix(match_expr, scrutinee_ty)?;
    
    // Check exhaustiveness
    if let Some(witness) = matrix.check_exhaustive(self) {
        return Err(TypeError::NonExhaustiveMatch {
            witness: self.witness_to_string(&witness),
            span: match_expr.span,
        });
    }
    
    // Check redundancy (warnings)
    let redundant = matrix.check_redundant(self);
    for arm_idx in redundant {
        self.warnings.push(Warning::UnreachablePattern {
            span: match_expr.arms[arm_idx].span,
        });
    }
    
    Ok(())
}
```

### 5.4 DoS Limits

```rust
const MAX_PATTERN_MATRIX_SIZE: usize = 10_000;
const MAX_EXHAUSTIVENESS_DEPTH: usize = 100;

impl PatternMatrix {
    fn check_limits(&self) -> Result<(), TypeError> {
        let size = self.rows.len() * self.col_types.len();
        if size > MAX_PATTERN_MATRIX_SIZE {
            return Err(TypeError::PatternMatrixTooLarge { size, max: MAX_PATTERN_MATRIX_SIZE });
        }
        Ok(())
    }
}
```

### 5.5 Tests for Phase 5

```rust
#[test]
fn exhaustive_option_both() { ... }

#[test]
fn non_exhaustive_option_some_only() { ... }

#[test]
fn exhaustive_bool() { ... }

#[test]
fn exhaustive_with_wildcard() { ... }

#[test]
fn exhaustive_nested() { ... }

#[test]
fn redundant_after_wildcard() { ... }

#[test]
fn redundant_duplicate_pattern() { ... }

#[test]
fn exhaustive_enum_many_variants() { ... }

#[test]
fn pattern_matrix_size_limit() { ... }
```

### 5.6 Verification

```fish
cargo test -p strata-types
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Phase 6: Hardening & Polish (2 days)

### Goal
Edge cases, error messages, evaluator support, destructuring let.

### 6.1 Destructuring Let

In parser and type checker:

```rust
// Parser: recognize irrefutable patterns in let
fn parse_let(&mut self) -> Result<Stmt, ParseError> {
    // let (a, b) = expr;
    // let Point { x, y } = expr;
}

// Type checker: verify pattern is irrefutable
fn check_let_pattern(&mut self, pat: &Pat, ty: &Ty) -> Result<Vec<PatternBinding>, TypeError> {
    if !self.is_irrefutable(pat, ty) {
        return Err(TypeError::RefutableLetPattern {
            span: pat.span(),
            help: "use `match` for refutable patterns",
        });
    }
    self.check_pattern(pat, ty)
}

fn is_irrefutable(&self, pat: &Pat, ty: &Ty) -> bool {
    match pat {
        Pat::Wildcard(_) | Pat::Ident(_, _) => true,
        Pat::Tuple(pats, _) => {
            match ty {
                Ty::Tuple(tys) => pats.iter().zip(tys).all(|(p, t)| self.is_irrefutable(p, t)),
                _ => false,
            }
        }
        Pat::Struct { .. } => {
            // Struct patterns are irrefutable if struct has no variants
            true
        }
        Pat::Variant { .. } => {
            // Enum variants are refutable (except single-variant enums)
            self.is_single_variant_enum(ty)
        }
        Pat::Literal(_, _) => false,  // Literals are refutable
    }
}
```

### 6.2 Evaluator Support

In `strata-cli/src/eval.rs`:

```rust
fn eval_match(&mut self, match_expr: &MatchExpr) -> Result<Value, EvalError> {
    let scrutinee = self.eval_expr(&match_expr.scrutinee)?;
    
    for arm in &match_expr.arms {
        if let Some(bindings) = self.match_pattern(&arm.pat, &scrutinee)? {
            return self.with_bindings(bindings, |ctx| {
                ctx.eval_expr(&arm.body)
            });
        }
    }
    
    // Should never reach here if exhaustiveness check passed
    unreachable!("non-exhaustive match reached at runtime")
}

fn eval_tuple(&mut self, exprs: &[Expr]) -> Result<Value, EvalError> {
    let values: Vec<Value> = exprs
        .iter()
        .map(|e| self.eval_expr(e))
        .collect::<Result<_, _>>()?;
    
    Ok(Value::Tuple(values))
}

fn eval_struct_construction(&mut self, name: &str, fields: &[(String, Expr)]) -> Result<Value, EvalError> {
    // ...
}

fn eval_variant_construction(&mut self, path: &Path, args: &[Expr]) -> Result<Value, EvalError> {
    // ...
}
```

### 6.3 Error Message Polish

Good error messages for:
- Non-exhaustive match (show witness)
- Unreachable pattern (show which arm)
- Caps in ADT (show field, suggest alternative)
- Refutable let (suggest match)
- Duplicate pattern binding
- Unknown constructor
- Wrong number of constructor args
- Pattern type mismatch

### 6.4 Example Files

Create `examples/option.strata`:
```strata
enum Option<T> {
    Some(T),
    None,
}

fn unwrap_or<T>(opt: Option<T>, default: T) -> T {
    match opt {
        Option::Some(x) => x,
        Option::None => default,
    }
}

fn main() {
    let x = Option::Some(42);
    let y = unwrap_or(x, 0);
    y
}
```

Create `examples/result.strata`:
```strata
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn map<T, U, E>(res: Result<T, E>, f: fn(T) -> U) -> Result<U, E> {
    match res {
        Result::Ok(x) => Result::Ok(f(x)),
        Result::Err(e) => Result::Err(e),
    }
}
```

Create `examples/tuple.strata`:
```strata
fn swap<A, B>(pair: (A, B)) -> (B, A) {
    let (a, b) = pair;
    (b, a)
}

fn main() {
    let p = (1, "hello");
    let q = swap(p);
    q
}
```

### 6.5 Final Test Suite

Target: ~50 new tests for Issue 007, bringing total to ~215.

Categories:
- Parsing (15 tests)
- Type representation (5 tests)
- ADT registration (10 tests)
- Match inference (10 tests)
- Exhaustiveness (10 tests)
- Evaluator (10 tests)
- Error messages (5 tests)

### 6.6 Final Verification

```fish
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Run examples
cargo run -p strata-cli -- --eval examples/option.strata
cargo run -p strata-cli -- --eval examples/result.strata
cargo run -p strata-cli -- --eval examples/tuple.strata
```

---

## Definition of Done

- [ ] All phases complete
- [ ] ~215 tests passing
- [ ] Zero clippy warnings
- [ ] Example files run correctly
- [ ] Error messages are clear and helpful
- [ ] DoS limits enforced (pattern depth, matrix size)
- [ ] Caps-in-ADT check working
- [ ] Exhaustiveness check working
- [ ] Redundancy warnings working
- [ ] Destructuring let working (irrefutable only)
- [ ] Documentation updated (IMPLEMENTED.md, IN_PROGRESS.md)

---

## Post-Issue Checklist

1. Bug hunt session
2. Documentation scrub
3. Multi-source external review (3 independent reviewers)
4. Update IN_PROGRESS.md for Issue 008
5. Tag v0.1.0 or v0.0.8.0

---

## Reference Documents

- `docs/adr-0003-adts-pattern-matching.md` — Design decisions
- `docs/DEV_WORKFLOW.md` — Development process
- `docs/lessons-learned.md` — Patterns to avoid