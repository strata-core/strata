# Strata Style Guide (v0)

- **Blocks**: braces required `{ ... }`; **semicolons optional**.
- **Paradigm**: functional core, effect-typed imperative shell, supervised concurrency.
- **Pipes vs chaining**: Method-chaining is canonical; `|>` is optional sugar.
- **Types**:
  - Local `let` → inference by default.
  - `pub` items (fn/actor/trait) → explicit **types and effect rows**.
  - Capability params explicit: `using fs: FsCap`. Using cap: CapType for capability params; layer {} sugar permitted—but desugars to functions.
- **Pattern matching**: prefer `match` with exhaustiveness; guards allowed.
- **Profiles**: declare at package/module; compiler enforces constraints.
- **Errors**: use `Result<E, A>` or `&{ Fail[E] }`; `?` for propagation.
- **Public APIs**: must annotate effect rows; local inference OK.
