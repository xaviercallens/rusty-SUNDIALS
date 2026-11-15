# Lean gate at 4e3e32d (3 changed files; same method as af4886f, cores 4,5, 300 s)

|file|class|reason|thms|flagged|native_decide|axiom decls|sorry code/comment|s|af4886f class|
|-|-|-|-|-|-|-|-|-|-|
|audit_unsoundness_demo.lean|FAIL-AXIOM|non-whitelisted axioms: AuditDemo.FLAGNO.flagno_l4_telemetry_oracle, AuditDemo.Monopole.gcp_l4_telemetry_oracle|2|0|0|2|0/3|15.3|None|
|fusion_sop_flagno.lean|FAIL-IMPORT|fusion_sop_flagno.lean:21:0: error: object file '/mnt/disks/disk-socrateai-local-1/AutoevolveAI/formal_.lake/packages/mathlib/.lake/build/lib/lean/Mathlib/LinearAlgebra/Matrix/Spectrum.olean' of module Mathlib.LinearAlgebra.Matrix.Spectrum |3|0|0|1|0/2|1.8|FAIL-IMPORT|
|fusion_sop_monopole.lean|FAIL-COMPILE|fusion_sop_monopole.lean:43:22: error: invalid binder annotation, type is not a class instance [possible staging artifact: error on appended #print axioms line]|3|0|0|2|0/3|25.1|FAIL-COMPILE|

audit_unsoundness_demo.lean is an intentional unsoundness demonstration (its own header says so).
