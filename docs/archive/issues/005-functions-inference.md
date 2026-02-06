# Issue 005: Function Definitions & Inference

**Status:** ✅ COMPLETE (including soundness hardening)  
**Completed:** February 1, 2026  
**Phase:** 2 (Basic Type System)  
**Depends on:** Issue 004 ✅

## Goal

Add function definitions with proper type inference using constraint-based approach.

## Scope

**Syntax:**
```strata
fn add(x: Int, y: Int) -> Int {
    x + y
}

fn apply(f, x) {  // types inferred
    f(x)
}
```

**Includes:**
- Parse function definitions
- Function types `Ty::Arrow(Vec<Ty>, Box<Ty>)` (multi-param, not curried)
- Constraint generation (constraint-based, NOT Algorithm W)
- Constraint solving (unification)
- Function calls with type checking
- Let-polymorphism (generalization/instantiation)

**Excludes:**
- Closures (deferred)
- Blocks in function bodies (Issue 006)
- Full mutual recursion (basic recursion works via two-pass)

## Definition of Done

**Phases 1-7: Functional Implementation** ✅ **COMPLETE**
- [x] AST support for `FnDecl` item (multi-param)
- [x] Parser for `fn name(param1, param2, ...) -> Type { body }`
- [x] Multi-param `Ty::Arrow(Vec<Ty>, Box<Ty>)`
- [x] Constraint generation pass (InferCtx)
- [x] Constraint solver using Unifier
- [x] Generalization (∀ introduction)
- [x] Instantiation (∀ elimination)
- [x] Function call type checking
- [x] Tests for:
  - [x] Simple functions
  - [x] Higher-order functions
  - [x] Let-polymorphism
  - [x] Type errors in functions (wrong args, wrong return)
- [x] CLI integration works
- [x] All Issue 004 tests still pass (44 total tests)

**Phase 8: Soundness Hardening (005-b)** ✅ **COMPLETE**
- [x] Unknown identifiers return hard errors (not fresh vars)
- [x] Real unification errors (no placeholder "Unit vs Unit" messages)
- [x] Correct generalization (compute env free vars properly)
- [x] Numeric constraints on arithmetic operators (no `true + true`)
- [x] Two-pass module checking (forward references/recursion)
- [x] Minimal constraint provenance (span tracking)
- [x] Determinism audit (stable error ordering - PASS)
- [x] Comprehensive negative tests for all soundness bugs

## Implementation Complete ✅

### AST & Parser ✅
```rust
// strata-ast/src/lib.rs
pub enum Item {
    Let(LetDecl),
    Fn(FnDecl),  ✅
}

pub struct FnDecl {
    pub name: Ident,
    pub params: Vec<Param>,      // Multi-param support
    pub ret_ty: Option<TypeExpr>,
    pub body: Expr,
    pub span: Span,
}

pub struct Param {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}
```

### Type System ✅
```rust
// strata-types/src/infer/ty.rs
pub enum Ty {
    Var(TypeVarId),
    Int,
    Float,
    Bool,
    String,
    Unit,
    Arrow(Vec<Ty>, Box<Ty>),  // Multi-param: (T1, T2, ...) -> R
    // ... other variants
}

pub struct Scheme {
    pub forall: Vec<TypeVarId>,  // ∀ quantified vars
    pub ty: Ty,
}

impl Scheme {
    pub fn mono(ty: Ty) -> Self;  // No quantification
    pub fn instantiate<F>(&self, fresh: F) -> Ty;
}
```

### Constraint-Based Inference ✅
```rust
// strata-types/src/infer/constraint.rs
pub struct InferCtx {
    fresh_counter: u32,
    constraints: Vec<Constraint>,
}

pub enum Constraint {
    Equal(Ty, Ty, Span),  // ✅ Now includes span for provenance
}

impl InferCtx {
    pub fn fresh_var(&mut self) -> Ty;
    pub fn add_constraint(&mut self, c: Constraint);
    pub fn take_constraints(&mut self) -> Vec<Constraint>;
    pub fn infer_expr(&mut self, env: &HashMap<String, Scheme>, expr: &Expr) 
        -> Result<Ty, InferError>;  // ✅ Returns Result, not bare Ty
    pub fn generalize(&self, ty: Ty, env_vars: &HashSet<TypeVarId>) -> Scheme;
}

// strata-types/src/infer/solver.rs
pub struct Solver {
    unifier: Unifier,
}

impl Solver {
    pub fn solve(&mut self, constraints: Vec<Constraint>) 
        -> Result<Subst, unifier::TypeError>;
}
```

### Type Checker Integration ✅
```rust
// strata-types/src/checker.rs
pub struct TypeChecker {
    env: HashMap<String, Scheme>,  // Stores schemes, not types
    infer_ctx: InferCtx,
}

impl TypeChecker {
    fn check_module(&mut self, module: &Module) -> Result<(), TypeError>;  // ✅ Two-pass
    fn check_let(&mut self, decl: &LetDecl) -> Result<(), TypeError>;
    fn check_fn(&mut self, decl: &FnDecl) -> Result<(), TypeError>;
    fn extract_fn_signature(&mut self, decl: &FnDecl) -> Result<Ty, TypeError>;  // ✅ Pass 1
}
```

## All Issues Fixed ✅

1. ✅ **Unknown identifiers now error properly**
   - Changed `infer_expr` to return `Result<Ty, InferError>`
   - Added `InferError::UnknownVariable` variant
   - Test: `type_error_unknown_variable`

2. ✅ **Real unification errors**
   - Thread `unifier::TypeError` through conversion
   - Added `unifier_error_to_type_error()` helper
   - Errors show actual types: "Int vs Bool" not "Unit vs Unit"

3. ✅ **Correct generalization**
   - Compute `free_vars_env()` before generalizing
   - Added `free_vars_scheme()` and `free_vars_env()` functions
   - Sound polymorphism ready for nested scopes

4. ✅ **Numeric constraints**
   - Arithmetic operators constrain to Int
   - Unary negation constrains to Int
   - Tests: `type_error_bool_arithmetic`, `type_error_neg_bool`

5. ✅ **Two-pass module checking**
   - Pass 1: Predeclare all functions
   - Pass 2: Check bodies
   - Tests: `forward_reference_works`, `mutual_recursion_predeclared`

6. ✅ **Constraint provenance**
   - Added `Span` to `Constraint::Equal`
   - All generation sites include span
   - Foundation for better errors in Issue 006

7. ✅ **Determinism audit - PASS**
   - No HashMap iteration in error paths
   - All operations are order-independent or sorted
   - Documented in `docs/DETERMINISM_AUDIT.md`

## Tests Implemented

**Total: 49 tests (all passing)**

**Positive tests (9 tests):**
- ✅ `simple_function_declaration` - Basic fn with annotations
- ✅ `function_with_inferred_params` - No param type annotations
- ✅ `function_with_inferred_return` - No return type annotation
- ✅ `function_call_in_let` - Call function from let binding
- ✅ `polymorphic_identity_function` - Same fn, different types
- ✅ `higher_order_function` - Functions as values
- ✅ `multiple_functions` - Multiple fn items in module
- ✅ `forward_reference_works` - Call function defined later
- ✅ `mutual_recursion_predeclared` - Mutually referencing functions

**Negative tests (6 tests):**
- ✅ `type_error_wrong_arg_count` - Arity mismatch
- ✅ `type_error_wrong_arg_type` - Type mismatch in argument
- ✅ `type_error_wrong_return_type` - Body doesn't match return annotation
- ✅ `type_error_unknown_variable` - Unknown identifier errors
- ✅ `type_error_bool_arithmetic` - `true + true` errors
- ✅ `type_error_neg_bool` - `-true` errors

## Design Decisions

### Constraint-Based vs Algorithm W ✅
**Decision:** Constraint-based (separate generation and solving)

**Rationale:**
- Cleaner separation of concerns
- Easier to extend for effects (Issue 008)
- Better error messages (can report multiple errors)
- More flexible for optimizations

### Multi-Param Arrows vs Currying ✅
**Decision:** Direct multi-param `Arrow(Vec<Ty>, Box<Ty>)`

**Rationale:**
- Matches "boring but clear" principle
- Aligns with target users (Python/Go/Rust backgrounds)
- Better error messages (can report arity mismatches)
- Simpler implementation

### Two-Pass Module Checking ✅
**Decision:** Predeclare functions before checking bodies

**Rationale:**
- Enables forward references
- Supports basic recursion
- Clear semantics (no mysterious ordering requirements)
- Foundation for mutual recursion

## Success Criteria

**All criteria met:** ✅

1. ✅ Can define and call simple functions
2. ✅ Type inference works for unannotated parameters
3. ✅ Let-polymorphism works (same function, different types)
4. ✅ Type errors in functions are caught
5. ✅ CLI can run function examples
6. ✅ All Issue 004 tests still pass
7. ✅ Unknown identifiers are hard errors
8. ✅ Error messages are accurate and helpful
9. ✅ Generalization is sound
10. ✅ Type rules prevent nonsense programs
11. ✅ Module semantics are explicit and documented
12. ✅ Deterministic, debuggable behavior

## Ready to Merge ✅

**Issue 005 is complete and ready to merge to main as v0.0.5.**

All critical soundness bugs fixed. Type system is trustworthy and ready for Issue 006.

**Completion:** February 1, 2026

## Next Steps

1. ✅ Complete functional implementation (Phases 1-7)
2. ✅ Complete soundness hardening (Phase 8 / 005-b)
3. ⏭️ Merge to main as v0.0.5
4. ⏭️ Begin Issue 006 (blocks & control flow)

## References

- Constraint-based type inference (not Algorithm W)
- Types and Programming Languages (Pierce)
- Hindley-Milner type system
- Let-polymorphism (value restriction)
