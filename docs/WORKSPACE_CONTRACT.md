	•	No crate may depend upward (no cycles).
	•	strata-ast → strata-parse → strata-types → (strata-ir,strata-vm,strata-codegen …).
	•	“Kernel first”: anything that might require I/O belongs in runtime crates, not in types or parse.
	•	Effects are grouped in MVP (Net,FS,Time,Rand)—no subtyping yet.