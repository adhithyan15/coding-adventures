# PR73: Prolog Runtime Backend Selector

## Goal

Let callers choose the Prolog execution backend through the main compiler and
runtime APIs instead of learning two parallel helper families.

PR71 made the bytecode VM path executable. PR72 proved stress parity. PR73
makes that convergence ergonomic by adding a shared `backend` selector with a
safe default:

- `backend="structured"` keeps the original `logic-vm` behavior.
- `backend="bytecode"` routes the same compiled Prolog program through
  `logic-bytecode` and `logic-bytecode-vm`.

## Scope

- Add the public `PrologVMBackend` type alias.
- Add `load_compiled_prolog_backend_vm(...)`.
- Add `backend=...` to compiled query, named answer, initialization, and
  stateful runtime helpers.
- Keep all existing bytecode-specific helpers as compatibility wrappers.
- Cover compiled query execution, named answers, initialization state, source
  runtimes, and module-aware project runtimes through the selector.

## Acceptance

- Existing callers continue to use the structured backend by default.
- Passing `backend="bytecode"` returns the same source-level answers as the
  bytecode-specific helpers.
- Stateful runtime commits still persist dynamic database changes when the
  selected backend is bytecode.

## Future Direction

Once the bytecode backend becomes the default internal runtime, this selector
gives us a low-risk migration seam. Tests can exercise both backends during the
transition while user-facing APIs remain stable.
