# Strata Charter (v0.1)

**Tagline:** Layered safety by design.  
**Essence:** Make system layers visible, verifiable, and harmonious.  
**Each line answers:** What (type), How (effects), Why (trace).

**Core principles:** effect-typed safety; capability security; supervised/structured concurrency; profiles (Kernel/Realtime/General); logic for explainability; deterministic replay; layered context sugar.

**Positioning:** Strata is a **security primitive**, not just a better scripting language. It provides capability security, effect tracking, and deterministic replay as language features—not infrastructure you have to build.

**Design philosophy:** Rigor over ergonomics. If an automation engineer has to type `using fs: FsCap` to delete a file, that explicitness is the feature, not a bug. The secure way is the easy way (zero-tax safety).

**Success metric:** An auditor can reconstruct behavior from effect traces alone. Function signatures are contracts of intent. Security auditing becomes type checking. This is compliance-as-code.
