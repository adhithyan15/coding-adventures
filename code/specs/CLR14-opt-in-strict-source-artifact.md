# CLR14 opt-in strict source artifact entry

## Evidence and scope

Twig `(+ 2147483647 1)` uses the arithmetic syntax already tested by the
language matrix. Its unmodified frontend IIR is i64 throughout. Strict encoded
execution halts with Int64(2147483648); the existing default source API halts
with Int(-2147483648). The equivalent defined helper emits any boundaries and
is refused by strict lowering. McCarthy 4294967296 emits ref<any> and is also
refused. These are distinct source representation contracts, not permission to
relabel dynamic or narrow types. Fresh ownership found no CLR overlap.

## Public API

Add `compile_source_to_typed_cil_artifact(language: Language, source: &str,
name: &str) -> Result<CILProgramArtifact, LangAotError>` in lang-aot.
Compile via the existing compile_source_to_iir entry, then pass that unmodified
module to lower_typed_scalars_to_cil using IIRClrConfig::new(name). Propagate
frontend errors unchanged; wrap strict backend errors as ClrBackendError using
the existing diagnostic formatting. Do not return a partial artifact on error.

This is explicitly opt-in. Do not change compile_source_to_cil_artifact,
compile_source_to_cil_text, CLI routing, source compilers, or strict backend
semantics. Apply no concretization, width rewriting, heap/closure transforms,
comparison retyping, immediate materialization, or fallback to legacy lowering.
Every source language may be passed to the API, but acceptance is determined
solely by its actual emitted IIR and the strict backend's existing contract.
No language-wide compatibility or new input support is claimed.

## Result and safety contract

The artifact retains the strict entry method's exact scalar return type and
encoded instructions. The caller loads the complete method table, selects the
artifact entry label, and observes Int64 through the simulator for i64 results;
there is no process-exit conversion or textual launcher. Existing strict type,
SSA, register bounds, forward-flow, definite assignment, and builtin rules
remain authoritative. Dynamic any/ref types, narrow u8/u4 operations, mutable
redefinitions and backward flow remain refusals where the backend refuses them.
Host input injection and stream lifetime retain CLR13's separate contract.

## Validation

Permanent source-level tests must execute complete artifacts for Twig wide
arithmetic crossing i32 and a small arithmetic control. The existing strict
CLR01 literal gate rejects out-of-i32 constants even with an i64 hint; assert
that refusal rather than expanding the backend literal contract. Assert
exact return/local types and Int64 values, not just successful compilation.
Demonstrate the default API's existing wide-literal refusal and arithmetic
wrapping separately, without claiming to fix its overflow behavior. Assert
refusal of a dynamic Twig helper, McCarthy ref<any> literal and Nib narrow
arithmetic; assert frontend errors propagate. Use backend existing suites for
strict malformed-IIR coverage instead of duplicating them through source tests.
Run lang-aot CIL source and strict scalar/flow/long-branch integration tests,
backend suite, relevant Clippy, docs build/shard checks. Update public API docs
and a correctly named changelog shard. Security review exact HEAD before push;
one ready PR, all latest push/PR workflows passing before auto-squash merge.
