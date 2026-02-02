# Strata: Clean Start for Issue 004

> Instructions for discarding broken work and starting fresh  
> Date: January 30, 2026

## Current Situation

You're on branch `issue-004` with:
- **Modified:** `crates/strata-parse/src/lib.rs`
- **Untracked:** `crates/strata-parse/infer_bridge/` (broken)
- **Untracked:** `crates/strata-types/src/infer/{constraint.rs, solver.rs}` (broken)

These files were never compiled and contain import errors. We're discarding them.

---

## Step 1: Discard Broken Work

```bash
# Make sure you're on issue-004 branch
git status

# Discard modifications to lib.rs
git checkout -- crates/strata-parse/src/lib.rs

# Remove untracked files
rm -rf crates/strata-parse/infer_bridge/
rm crates/strata-types/src/infer/constraint.rs
rm crates/strata-types/src/infer/solver.rs

# Verify clean state
git status
# Should show: "nothing to commit, working tree clean"
```

---

## Step 2: Return to Main

```bash
# Switch back to main branch
git checkout main

# Verify you're on main
git branch
# Should show: * main

# Verify all tests pass
cargo test --workspace
```

---

## Step 3: Update Documentation

Copy the new docs into your repo:

```bash
# Create docs directory if it doesn't exist
mkdir -p docs

# Add these files from Claude's output:
# - IMPLEMENTED.md
# - IN_PROGRESS.md  
# - PLANNED.md

# Move old docs to archive if you want to keep them
mkdir -p docs/archive
mv docs/DoD.md docs/archive/  # if you want to keep it

# Keep the issues/ directory but update it
# Replace issues/004-vm-bytecode-replay.md with new 004-basic-type-checking.md
```

**File updates needed:**

1. `docs/IMPLEMENTED.md` - Replace with new version
2. `docs/IN_PROGRESS.md` - New file
3. `docs/PLANNED.md` - New file
4. `issues/004-basic-type-checking.md` - Replace old 004
5. `issues/005-functions-inference.md` - New file
6. `issues/006-010-summary.md` - New file

---

## Step 4: Commit Documentation Updates

```bash
# Stage the documentation changes
git add docs/IMPLEMENTED.md
git add docs/IN_PROGRESS.md
git add docs/PLANNED.md
git add issues/004-basic-type-checking.md
git add issues/005-functions-inference.md
git add issues/006-010-summary.md

# Commit with clear message
git commit -m "Update roadmap and docs: realistic timeline & issue 004 rewrite

- Add IMPLEMENTED.md documenting completed work (issues 001-003)
- Add IN_PROGRESS.md for current issue 004 (basic type checking)
- Add PLANNED.md with 12-18 month realistic roadmap
- Rewrite issue 004: constraint collection -> basic type checking
- Add issues 005-010 with detailed specs
- Archive old DoD.md approach in favor of new doc structure"
```

---

## Step 5: Create Fresh Issue 004 Branch

```bash
# Create and switch to new issue-004 branch from main
git checkout -b issue-004

# Verify you're on the new branch
git branch
# Should show: * issue-004

# At this point you have a clean slate to start issue 004
```

---

## Step 6: Start Issue 004 Implementation

Now you're ready to begin. Here's the recommended order:

### 6.1: Extend TyConst

**File:** `crates/strata-types/src/infer/ty.rs`

Add Float and String:
```rust
pub enum TyConst {
    Unit,
    Bool,
    Int,
    Float,   // NEW
    String,  // NEW
}
```

Update Display impl:
```rust
impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ... existing cases ...
            Ty::Const(TyConst::Float) => write!(f, "Float"),
            Ty::Const(TyConst::String) => write!(f, "String"),
        }
    }
}
```

Add convenience constructors:
```rust
impl Ty {
    // ... existing methods ...
    #[inline]
    pub fn float() -> Self {
        Ty::Const(TyConst::Float)
    }
    #[inline]
    pub fn string() -> Self {
        Ty::Const(TyConst::String)
    }
}
```

Test:
```bash
cargo test -p strata-types
```

Commit:
```bash
git add crates/strata-types/src/infer/ty.rs
git commit -m "Add Float and String type constants

- Extend TyConst enum
- Add Display cases
- Add convenience constructors
- All tests pass"
```

### 6.2: Create TypeChecker Skeleton

**File:** `crates/strata-types/src/checker.rs` (new file)

Start with basic structure:
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
                write!(f, "Type mismatch at {:?}: expected {}, found {}", 
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

impl std::error::Error for TypeError {}

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
        // TODO: implement
        unimplemented!()
    }

    fn infer_expr(&mut self, expr: &Expr) -> Result<Ty, TypeError> {
        // TODO: implement
        unimplemented!()
    }
}
```

Update `crates/strata-types/src/lib.rs`:
```rust
mod checker;  // NEW
mod effects;
mod profile;
mod types;

pub use checker::{TypeChecker, TypeError};  // NEW
pub use effects::{Effect, EffectRow};
pub use profile::Profile;
pub use types::{PrimType, Type};

// ... rest unchanged
```

Test it compiles:
```bash
cargo build -p strata-types
```

Commit:
```bash
git add crates/strata-types/src/checker.rs
git add crates/strata-types/src/lib.rs
git commit -m "Add TypeChecker skeleton

- Create checker.rs with TypeChecker struct
- Add TypeError enum
- Add module/item checking stubs
- Export from lib.rs
- Compiles but unimplemented"
```

### 6.3: Implement Type Inference

Fill in the implementation in `checker.rs`. See issue 004 spec for rules.

This is the main work - expect 2-3 days here.

### 6.4: Add Tests

Create `crates/strata-types/src/checker_tests.rs` with comprehensive tests.

### 6.5: Wire Into CLI

Add dependency and call checker from main.

### 6.6: Final Testing

Run all workspace tests, test examples, create negative test cases.

---

## Recommended Work Sessions

**Session 1 (2-3 hours):** Steps 6.1-6.2 - Extend types, create skeleton  
**Session 2 (3-4 hours):** Step 6.3 - Implement infer_expr for literals and simple ops  
**Session 3 (3-4 hours):** Step 6.3 - Implement binops and let checking  
**Session 4 (2-3 hours):** Step 6.4 - Write comprehensive tests  
**Session 5 (2 hours):** Steps 6.5-6.6 - CLI integration and validation  

---

## When You're Done

```bash
# Make sure all tests pass
cargo test --workspace

# Commit final work
git add -A
git commit -m "Complete issue 004: basic type checking

- Implemented TypeChecker with full expr inference
- Added Float and String type constants
- Comprehensive test coverage
- CLI integration
- All examples work
- Negative tests catch errors"

# Merge to main
git checkout main
git merge issue-004

# Tag the release
git tag v0.0.4 -m "Issue 004 complete: Basic type checking"

# Update IN_PROGRESS.md to point to issue 005
```

---

## Need Help During Implementation?

Come back with:
1. What step you're on
2. What's not working (error messages, behavior)
3. What you've tried

I'll help debug and keep you moving forward.

---

## Success Looks Like

At the end of Issue 004:
- ✅ `let x: Int = 1;` type checks
- ✅ `let y = 2 + 3;` infers Int
- ✅ `let bad = 1 + true;` fails with clear error
- ✅ All existing examples work
- ✅ CLI runs type checker before eval
- ✅ Ready to tackle functions in Issue 005

Let's ship this! 🚀
