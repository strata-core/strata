# Issue 004: Basic Type Checking

**Status:** COMPLETED  
**Estimated time:** 5-7 days  
**Phase:** 2 (Basic Type System)

**⚠️ Scope Note:**
The original plan had Issue 004 as "Traits & Trait Impl" with full constraint collection for trait obligations. After re-evaluation, the scope was revised to "Basic Type Checking" - a simpler, faster-to-ship foundation. This gets type checking working on what we already parse before adding new syntax. Traits are now Issue 007, after we have functions and ADTs. The constraint-based inference approach will be introduced in Issue 005 (functions).

---

## Goal

Add a type checking pass that validates the programs we can already parse and evaluate.

## Scope

**What we're building:**
- TypeChecker that walks AST and assigns types
- Type inference for literals and binary operators
- Error reporting with source spans
- CLI integration

**What we're NOT building:**
- Constraint-based inference (Issue 005)
- Polymorphism (Issue 005)
- Effect checking (Issue 008)
- Function types (Issue 005)

## Definition of Done

- [ ] Create `strata-types/src/checker.rs` with TypeChecker struct
- [ ] Extend `TyConst` with `Float` and `String` variants
- [ ] Implement `check_module()` for Module
- [ ] Implement `infer_expr()` for all Expr variants
- [ ] Add type environment for let bindings
- [ ] Wire into CLI (run type checker before eval)
- [ ] Add `strata-types` dependency to `strata-parse`
- [ ] Update workspace `Cargo.toml` if needed
- [ ] Tests in `strata-types/src/checker_tests.rs`:
  - Literals infer correctly
  - Binops type check
  - Type errors caught (e.g., `1 + "hello"`)
  - Let bindings work
  - Variable references work
- [ ] Error messages include spans
- [ ] All existing examples still work
- [ ] At least 3 negative tests (bad programs rejected)

## Type Rules

```
Lit(Int(_))       → Ty::Const(TyConst::Int)
Lit(Float(_))     → Ty::Const(TyConst::Float)
Lit(Bool(_))      → Ty::Const(TyConst::Bool)
Lit(Str(_))       → Ty::Const(TyConst::String)
Lit(Nil)          → Ty::Const(TyConst::Unit)

Var(name)         → lookup in env, error if not found

Unary(Not, e)     → e must be Bool, result Bool
Unary(Neg, e)     → e must be Int or Float, result same

Binary(Add, l, r) → both Int → Int, or both Float → Float
Binary(Sub, l, r) → both Int → Int, or both Float → Float
Binary(Mul, l, r) → both Int → Int, or both Float → Float
Binary(Div, l, r) → both Int → Int, or both Float → Float

Binary(Eq, l, r)  → both same type, result Bool
Binary(Ne, l, r)  → both same type, result Bool

Binary(Lt, l, r)  → both Int or both Float, result Bool
Binary(Le, l, r)  → both Int or both Float, result Bool
Binary(Gt, l, r)  → both Int or both Float, result Bool
Binary(Ge, l, r)  → both Int or both Float, result Bool

Binary(And, l, r) → both Bool, result Bool
Binary(Or, l, r)  → both Bool, result Bool

Call(...)         → error "functions not implemented yet"

Let(name, ann, val) → 
  - infer type of val
  - if ann present, check val type == ann type
  - add name : type to env
```

## Implementation Outline

### `strata-types/src/checker.rs`

```rust
use std::collections::HashMap;
use super::infer::ty::{Ty, TyConst};
use strata_ast::ast::{Module, Item, Expr, LetDecl, BinOp, UnOp, Lit};
use strata_ast::span::Span;

#[derive(Debug, Clone)]
pub enum TypeError {
    Mismatch { expected: Ty, found: Ty, span: Span },
    UnknownVariable { name: String, span: Span },
    NotImplemented { msg: String, span: Span },
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TypeError::Mismatch { expected, found, span } => {
                write!(f, "Type mismatch at {:?}: expected {:?}, found {:?}", 
                       span, expected, found)
            }
            TypeError::UnknownVariable { name, span } => {
                write!(f, "Unknown variable '{}' at {:?}", name, span)
            }
            TypeError::NotImplemented { msg, span } => {
                write!(f, "{} at {:?}", msg, span)
            }
        }
    }
}

pub struct TypeChecker {
    env: HashMap<String, Ty>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self { env: HashMap::new() }
    }

    pub fn check_module(&mut self, module: &Module) -> Result<(), TypeError> {
        for item in &module.items {
            self.check_item(item)?;
        }
        Ok(())
    }

    fn check_item(&mut self, item: &Item) -> Result<(), TypeError> {
        match item {
            Item::Let(decl) => self.check_let(decl),
        }
    }

    fn check_let(&mut self, decl: &LetDecl) -> Result<(), TypeError> {
        let val_ty = self.infer_expr(&decl.value)?;
        
        if let Some(ann_ty) = &decl.ty {
            let ann = self.ty_from_type_expr(ann_ty)?;
            if val_ty != ann {
                return Err(TypeError::Mismatch {
                    expected: ann,
                    found: val_ty,
                    span: decl.span,
                });
            }
        }
        
        self.env.insert(decl.name.text.clone(), val_ty);
        Ok(())
    }

    fn infer_expr(&mut self, expr: &Expr) -> Result<Ty, TypeError> {
        // implement based on rules above
        todo!()
    }

    fn ty_from_type_expr(&self, te: &TypeExpr) -> Result<Ty, TypeError> {
        // Convert TypeExpr::Path to Ty
        // For now just handle simple names: Int, Bool, String, Float
        todo!()
    }
}
```

### `strata-types/src/infer/ty.rs` (extend)

```rust
pub enum TyConst {
    Unit,
    Bool,
    Int,
    Float,   // ADD THIS
    String,  // ADD THIS
}
```

### `strata-cli/src/main.rs` (integrate)

```rust
use strata_types::checker::TypeChecker;

fn main() -> Result<()> {
    // ... existing code ...
    let module = parse_str(&cli.path, &src)?;

    // NEW: Type check
    let mut tc = TypeChecker::new();
    if let Err(e) = tc.check_module(&module) {
        eprintln!("Type error: {}", e);
        std::process::exit(1);
    }

    if cli.eval {
        eval_module(&module)?;
        return Ok(());
    }
    // ...
}
```

## Tests

### Positive tests (should pass)

```strata
let x: Int = 1;
let y = 2 + 3;
let z = true && false;
let w = 1 < 2;
let f: Float = 3.14;
let s: String = "hello";
```

### Negative tests (should fail with clear error)

```strata
let bad1 = 1 + true;        // Type mismatch
let bad2 = "hi" * 5;        // Type mismatch
let bad3: Bool = 123;       // Annotation mismatch
let bad4 = unknown_var;     // Unknown variable
```

## Success Criteria

1. All existing examples still parse and run
2. Type checker catches the bad examples above
3. Error messages show span and expected/found types
4. No false positives (good code doesn't fail)
5. Tests achieve >80% coverage of checker.rs

## Notes

- This is intentionally simple - just checking what we parse
- No polymorphism, no generics, no constraints
- Sets up infrastructure for real inference in Issue 005
- Immediate value: catches bugs in user code

-----

✅ COMPLETED - January 31, 2026
Status: Complete
Version: v0.0.4
Time Spent: ~1 day actual work (across multiple sessions)
What Was Built
Type Checker (strata-types/src/checker.rs):

280 lines of type inference logic
Expression type inference for all AST nodes
Type environment for let bindings
Variable lookup and scoping
Clear error messages with source spans

Tests (strata-types/src/checker_tests.rs):

34 comprehensive tests
19 positive tests (valid programs)
11 negative tests (type errors)
4 edge case tests


90% code coverage



CLI Integration:

Automatic type checking after parsing
Type errors displayed before evaluation
Clean exit codes (1 on type error)

Stats

Files Created: 2 main files + 6 test examples
Files Modified: 5 files
Lines Added: ~500 lines
Tests Added: 36 tests (2 for new types + 34 for checker)
Commits: 8 commits across 6 steps

Key Capabilities
✅ Type inference for all literals (Int, Float, Bool, String, Unit)
✅ Type checking for all operators (arithmetic, comparison, logical, unary)
✅ Let bindings with optional type annotations
✅ Variable references with environment lookup
✅ Parenthesized expressions
✅ Clear error messages with spans
✅ Full CLI integration
What Works
stratalet x: Int = 42;
let y = 2 + 3;
let z = true && false;
let result = (1 + 2) * 3;
What Fails (Correctly)
stratalet bad1 = 1 + true;       // Type mismatch
let bad2: Bool = 123;      // Annotation mismatch
let bad3 = unknown_var;    // Unknown variable
Lessons Learned

Starting with a simple type checker before full inference was the right call
Incremental steps (6.1-6.6) made the work manageable
Pre-commit hooks caught formatting/style issues early
AST structure differences required iteration (Paren field name, struct variants)
Test-driven approach caught edge cases

Next Steps
Issue 005 will add:

Function definitions
Function calls
Constraint-based inference
Polymorphism

Merged to main: January 31, 2026
Tagged: v0.0.4
