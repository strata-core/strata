# Strata — Peer Review Prompt

You are evaluating a new programming language proposal named **Strata**.

**Concept Overview**
Strata is a general-purpose, strongly typed language designed for *safe automation, explainable AI, and resilient distributed systems*. Its philosophy: every effect and authority is explicit, composable, and provable.

**Inspirations**
Rust (ownership), Erlang/Elixir (supervision), Haskell/ML/Koka (effects), Ada/SPARK (profiles), Scala (typeclasses), Julia (numerics), Datalog/Prolog (explainability).

**Key Principles**
1. Effect-typed safety: effects in function types; inference locally, explicit at `pub` boundaries.
2. Capability security: no ambient I/O; explicit caps (`NetCap`, `FsCap`, `TimeCap`, `RandCap`).
3. Supervised concurrency: actors, backoff, structured scopes.
4. Functional core + imperative shell.
5. Profiles: Kernel / Realtime / General.
6. Logic sublanguage: Datalog-lite with proof traces.
7. Deterministic replay: seedable `Time`/`Rand`; `strata replay`.
8. Layered Context: `layer { using caps… } & {Effects}` sugar.
9. Compile-time meta: typed, hygienic `compiletime` functions.
10. Style: braces; optional semicolons; method-chaining canonical; `|>` optional.

**Example**
```strata
pub fn fetch_logs(u: Url, using net: NetCap) -> Result<[Str], HttpErr> & { Net } {
  layer Logging(using fs: FsCap) & { FS } {
    let data = http_get(u, using net)?
    write_file("/tmp/logs.txt", data, using fs)
  }
}
```

**Questions**
1) Is Strata a plausible next step or an over-fusion? Why?
2) Which research areas should shape semantics (effects, capabilities, logic, IR)?
3) What early interop targets best showcase strengths?
4) If one killer feature ships first, which should it be?
5) What adoption risks or trade-offs do you foresee?

**Name rationale**
“Strata” evokes layers—geological, architectural, computational. Tagline: *Layered safety by design.*
