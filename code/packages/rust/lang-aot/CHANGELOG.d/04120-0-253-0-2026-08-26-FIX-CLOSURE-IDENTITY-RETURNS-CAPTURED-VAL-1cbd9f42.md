## 0.253.0 - 2026-08-26 (fix: `closure_identity_returns_captured_value` for real)

Fixes the recurring `e6d7a_wasm_closures.rs` failure on
`((lambda (x) x) 42)` ("the minimal apply") that at least four consecutive
PRs in this campaign independently re-confirmed as pre-existing/unrelated via
a `git stash` comparison and moved on from without fixing. Root cause: a
Twig/Nib closure body with NO heap evidence of its own (its entire body is
`ret x`) was never concretized to a concrete WASM type by either of the two
passes meant to partition the module between them — see `iir-builtin-lowering`
0.41.0's changelog for the full root cause (a partition disagreement between
`lower_dyn_repr_structural` and `concretize_scalar_any_for_wasm`, plus a
stale hardcoded `ref<any>` hint on the synthesized closure dispatcher's call
to such a body).

`concretize_scalar_any_for_wasm` here now:
- Partitions against `iir_builtin_lowering::lisp_functions` (the exact set
  `lower_dyn_repr_structural` uses) instead of its own separate whole-module
  heuristic, so the two passes can no longer disagree about which one owns a
  given function.
- Also narrows a scalar function's **parameters**, not just its return type
  and instruction hints (mirroring `concretize_scalar_any_for_jvm`, which
  already did this) — needed once a raw closure body like this one is
  actually reached by this pass for the first time.

All 5 tests in `tests/e6d7a_wasm_closures.rs` pass for real now, including
`closure_identity_returns_captured_value`
(previously WASM-only: `ValidationFailed(["UntypedInstruction: ... op \"ret\"
has type_hint \"any\""])`, then after the partition fix alone a second latent
bug surfaced, `TypeMismatch: expected Anyref, found I64`, fixed by the stale
dispatcher-call-hint correction in `iir-builtin-lowering`) — and native/LLVM
closures are unaffected (verified they still pass; an earlier attempt to fix
the stale hint in the shared `closure_heap.rs` emitter instead regressed
`closures_run_on_native`/`_llvm`, which is why the fix lives in the
WASM-specific `lower_dyn_repr_structural` pass instead).

