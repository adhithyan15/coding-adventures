## 0.223.0 - 2026-08-13 - scalar narrowing reads the IIR, not the language

Follows McCarthy emitting `ref<any>` for tagged values instead of bare `any`.
The `concretize_scalar_any_for_{llvm,jvm,cil,wasm}` helpers decided what to
narrow from a mix of type hints and op evidence, and three of those decisions
were wrong once the types became honest:

* **Boundary contract versus heap evidence.** A `ref<any>` *parameter* is an ABI
  contract — callers box for it, so narrowing that callee breaks both sides. A
  `ref<any>` *instruction hint* is only "this value is dynamic" and is no
  evidence of a heap object. Conflating them narrowed `((LAMBDA (X) X) 5)`'s
  lambda to `i32` while `main` still boxed, and the CLR died with
  `System.AccessViolationException` inside `CastHelpers.Unbox`. Split into
  `is_tagged_boundary_param` and `is_heap_evidence_hint`.
* **Narrowing is a whole-module decision.** A `call` couples caller and callee
  value models. Narrowing `main` while its callee kept a tagged boundary emitted
  a caller storing an `object` into an `int32` local —
  `System.InvalidProgramException` on the CLR. If any function in the module is
  on the tagged model, none is narrowed.
* **Post-rename builtin names count as evidence.** `(ATOM 7)` reaches these
  helpers as `dyn_pair_p`/`dyn_not`, not `pair?`/`not`, so the op-based evidence
  missed it and a tagged program was silently narrowed to machine ints. Both
  spellings are now listed.

Also: a program's exit code is a machine integer, and the entry's declared
return type now says so. That used to work by accident — the frontend typed the
entry `any`, and every backend's type map has no entry for bare `any` and falls
through to its integer default.
