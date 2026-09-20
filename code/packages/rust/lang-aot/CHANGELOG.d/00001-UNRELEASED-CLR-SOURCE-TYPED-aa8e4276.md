## Unreleased — opt-in strict CLR source artifacts

Add `compile_source_to_typed_cil_artifact` to lower unmodified frontend IIR
through the strict encoded scalar backend. Preserve i64 arithmetic results
without changing default routing or retyping dynamic/narrow source values.
Source execution and refusal tests retain the CLR01 literal gate.
