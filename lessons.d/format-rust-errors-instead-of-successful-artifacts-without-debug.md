---
category: Rust
---

# Format Rust errors instead of successful artifacts without Debug

A CLR01 regression attempted to format Result<CILProgramArtifact, IIRClrError>
with Debug in an assertion message. CILProgramArtifact does not implement Debug,
so the test failed to compile even though the assertion only checked is_ok().
Report the input value or extract the error before formatting; do not require
unrelated successful artifact types to implement Debug just for test diagnostics.
