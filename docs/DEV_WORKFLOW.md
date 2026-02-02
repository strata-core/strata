# Strata Development Workflow

> Evergreen process guide for all issues and milestones  
> Updated: February 1, 2026

This document describes the development process to follow for every issue, ensuring consistency, quality, and maintainability throughout the project.

---

## Core Principles

1. **Foundation integrity** - Fix issues before they compound
2. **Soundness over speed** - A type system that lies is worse than no type system
3. **Documentation as you go** - Don't defer docs to "later"
4. **Post-completion review** - Always audit before moving forward
5. **Deterministic behavior** - Flaky tests waste more time than they save

---

## After Completing Each Issue

### 1. Bug Hunt Session

**Goal:** Catch issues while the code is fresh in your mind, before they become blockers.

**Process:**
- Review all code paths touched by the issue
- Check for edge cases in new logic
- Verify error handling is comprehensive
- Look for potential panics or unwraps that should be Results
- Test boundary conditions (empty inputs, large inputs, invalid inputs)
- Run full test suite: `cargo test --workspace`
- Check for compiler warnings: `cargo clippy --workspace`

**When:** Immediately after implementation feels "complete" but before considering the issue done.

**Why:** Bugs found now are easy to fix. Bugs found three issues later require archaeological investigation.

---

### 2. Documentation Scrub

**Goal:** Ensure clarity and remove documentation cruft while context is fresh.

**What to update:**

#### Code Comments
- Review all function/struct/module doc comments
- Remove outdated comments that don't match current implementation
- Add missing comments for non-obvious logic
- Ensure examples in doc comments actually compile
- Check that comments explain "why" not just "what"

#### Project Docs
Update these files as appropriate:
- `IMPLEMENTED.md` - Move completed issue from IN_PROGRESS
- `IN_PROGRESS.md` - Update to reflect next issue
- `README.md` - Update if public API changed or examples need adjustment
- Issue file (e.g., `issues/005-functions.md`) - Mark as "Status: Complete" with completion date
- `DESIGN_DECISIONS.md` - Document any significant design choices made during implementation

#### What NOT to include:
- Time spent on the issue (no need to tell the world how many hours you worked)
- Temporary implementation notes that are no longer relevant
- Personal development journal entries

**When:** After bug hunt, before commit.

**Why:** Future-you (or contributors) will thank you for clear, accurate documentation.

---

### 3. Commit Checklist

Before committing, verify:

- [ ] All tests passing: `cargo test --workspace`
- [ ] No compiler warnings: `cargo clippy --workspace`
- [ ] No formatting issues: `cargo fmt --check`
- [ ] Documentation updated (code comments, IMPLEMENTED.md, issue file)
- [ ] Examples still work if they use changed code
- [ ] Commit message is clear and follows format (see below)

**Commit Message Format:**
```
Complete issue NNN: [brief description]

- [Key accomplishment 1]
- [Key accomplishment 2]
- [Key accomplishment 3]
- Test coverage: [number] tests passing
```

Example:
```
Complete issue 005: Functions & type inference

- Implemented constraint-based type inference with Algorithm W
- Added function declarations with multi-param arrows
- Generalization and instantiation for let-polymorphism
- Higher-order function support
- Test coverage: 49 tests passing
```

---

### 4. Update Project Status

**Update IN_PROGRESS.md:**
```markdown
# Strata: In Progress

**Last Updated:** [Today's date]
**Current Version:** v0.0.[N+1]
**Current Focus:** Ready to start issue [N+1]
**Progress:** Issue [N] complete ✅

---

## Recent Completions

### Issue [N]: [Title] ✅
**Completed:** [Date]

**What was built:**
- [Key feature 1]
- [Key feature 2]
...

---

## Next Up

### Issue [N+1]: [Title]
**Estimated start:** [When you plan to start]

**Why now:** [Brief rationale]

**Scope:**
- [Feature 1]
- [Feature 2]
...
```

**Tag the version:**
```fish
git tag v0.0.[N] -m "Issue [N] complete: [brief description]"
```

---

## Every 3-5 Issues (Milestone Review)

**Frequency:** After completing issues that form a natural milestone (e.g., after parser + type system basics).

### 1. Cross-Cutting Review

**Goal:** Look for patterns and emerging technical debt across recent work.

**Process:**
- Review code from last 3-5 issues as a cohesive unit
- Look for repeated patterns that should be abstracted
- Identify emerging technical debt
- Consider refactoring opportunities
- Check for architectural drift (are we still following WORKSPACE_CONTRACT?)
- Review error messages - are they clear and consistent?

**Questions to ask:**
- Are there duplicate implementations that should be unified?
- Are abstractions at the right level?
- Is the crate structure still clean?
- Are tests covering the right scenarios?
- Is performance acceptable? (Profile if uncertain)

---

### 2. Documentation Sync

**Goal:** Ensure high-level docs reflect current reality.

**Update if changed:**
- `roadmap.md` - Adjust timeline if velocity changed
- `DESIGN_DECISIONS.md` - Document any new architectural choices
- `STYLE_GUIDE.md` - Update if syntax or conventions evolved
- `README.md` - Refresh examples if they're stale
- Issue files - Update estimates for upcoming issues based on learnings

**Check for consistency:**
- Do code examples in docs still compile?
- Do docs contradict each other?
- Are there outdated claims? (e.g., "not yet implemented" for things now implemented)

---

### 3. Confidence Assessment

**Goal:** Measure technical progress against stated goals.

**Process:**
Write an updated confidence assessment comparing to previous benchmark:

```markdown
## Confidence Assessment: Post-Issue [N]

**Previous (post-issue [M]):** [X]/10
**Current (post-issue [N]):** [Y]/10

**What improved:**
- [Specific capability or foundation that's stronger]
- [Risk that was mitigated]

**What's still uncertain:**
- [Open question or risk]
- [Area needing more work]

**Overall trajectory:** [On track / Ahead of schedule / Need course correction]
```

**Criteria for confidence:**
- Foundation soundness (does the type system/compiler work correctly?)
- Test coverage (can we trust refactors won't break things?)
- Scope clarity (do we know what v0.1 needs?)
- Execution velocity (are we shipping at sustainable pace?)
- Design coherence (do pieces fit together well?)

---

## Special: Soundness Review

**When:** After implementing core type system or inference features.

**Goal:** Ensure the type system doesn't lie and can be trusted as a foundation.

**Process:**
1. List all invariants the type system should guarantee
2. For each invariant, write tests that would break if violated
3. Look for edge cases where the type system might be unsound
4. Review error messages - do they tell the truth?
5. Check for non-determinism in error output or type inference
6. Document findings in a SOUNDNESS_AUDIT.md or similar

**Example (from Issue 005-b):**
```markdown
# Soundness Audit (Issue 005-b)

**Date:** February 1, 2026
**Status:** ✅ PASS - No soundness issues found

## Issues Found and Fixed:
1. Unknown identifiers were type-checking as Unit (FIXED)
2. Unification errors reported as placeholders (FIXED)
3. Generalization unsound for nested scopes (FIXED)
...

## Audit Conclusions:
Foundation is sound. Safe to build control flow on top.
```

---

## Working with Claude Code

**When to use chat (this interface):**
- Design discussions and architectural decisions
- Code review and rubber-duck debugging
- Strategic planning and scope clarification
- Post-implementation reflection and learning
- Writing documentation that needs context and judgment

**When to use Claude Code:**
- Implementation grind with clear specifications
- Refactoring across multiple files
- Making tests pass after design is settled
- Fixing compiler errors iteratively
- Mechanical changes (renaming, restructuring)

**Hybrid workflow:**
```
Issue N planning (chat):
  └─> Design discussion, break down into phases
      └─> Claude Code: Implement Phase 1
          └─> Chat: Review Phase 1, discuss learnings
              └─> Claude Code: Implement Phase 2
                  └─> Chat: Final review and reflection
```

**Handoff to Claude Code:**
When starting a Claude Code session for implementation:
1. Reference the issue file: "Implement phase 1 of issue 006 per issues/006-blocks.md"
2. Point to design decisions: "Follow conventions in DESIGN_DECISIONS.md"
3. Specify what done looks like: "All tests in checker_tests.rs pass, no clippy warnings"
4. Return to chat for review before considering complete

---

## Development Environment Setup

**Prerequisites:**
- Rust toolchain (stable, latest)
- Git with SSH configured
- fish shell (for command examples)
- Editor with rust-analyzer (VSCode, Neovim, etc.)

**Pre-commit setup (recommended):**
```fish
# Create .git/hooks/pre-commit
#!/bin/sh
cargo fmt --check || exit 1
cargo clippy --workspace -- -D warnings || exit 1
cargo test --workspace || exit 1
```

**Make executable:**
```fish
chmod +x .git/hooks/pre-commit
```

This prevents committing code that doesn't pass quality checks.

---

## Common Pitfalls to Avoid

### Don't: Build on broken abstractions
If something feels wrong in the foundation, stop and fix it. Cost of fixing now < cost of debugging later.

### Don't: Defer documentation
"I'll document it later" means "I'll forget the context and write bad docs."

### Don't: Skip the bug hunt
"It compiled, ship it!" often turns into "Why is this failing in weird ways?" three issues later.

### Don't: Ignore flaky tests
Non-deterministic tests are worse than no tests. Fix or delete them.

### Don't: Overcommit scope
Better to ship a small, solid feature than a large, shaky one.

---

## When You're Stuck

### Debugging process:
1. Write down what you expected vs what happened
2. Identify the smallest reproduction case
3. Check if the issue is in new code or existing code
4. Review recent changes with `git diff`
5. Ask Claude in chat with specific error messages and context

### Design decision paralysis:
1. Write down the options (A, B, C)
2. List pros/cons for each
3. Consider: which is simplest? which is most reversible?
4. Pick one, document in DESIGN_DECISIONS.md, move forward
5. Remember: perfect is the enemy of shipped

### Scope creep:
1. Ask: "Is this required for the current issue's goal?"
2. If no: Create a new issue for it, continue with current issue
3. If yes: Break it down into smallest possible addition

---

## Success Metrics

**You're following this workflow well if:**
- Commits are atomic and tests always pass
- Documentation stays in sync with code
- Issues get completed without major post-completion fixes
- Test coverage stays >90%
- You can context-switch away and back without confusion
- Code reviews (even self-review) find the code understandable

**You're drifting if:**
- Commits are huge and break tests temporarily
- Docs are out of sync
- You keep finding bugs from old issues
- Tests are flaky or incomplete
- Coming back after a break is disorienting
- Code needs extensive comments to understand

---

## Version History

- **2026-02-01:** Initial version consolidating post-issue process
- Future: Will add CI/CD workflow when implemented
- Future: Will add release process when ready for external releases
