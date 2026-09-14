---
category: Compiler / VM / language pipeline
---

# Long-running Rust test files re-define identical helpers across phases

`tests/test_vm.rs` already had `eval_at`, `contains_head`, `trapezoid` at the top of the Phase 26+ block by the time Phase 34 was added; appending another `fn eval_at(...)` produced `E0428: the name eval_at is defined multiple times` and the entire test binary failed to compile. Fix: prefix new helpers with the phase name (`phase34_eval_at`, `phase34_subst`, `phase34_numerical_derivative`) when their semantics differ — the Phase 34 evaluator routed through `SymbolicBackend::eval` to handle `Tan`/`Sqrt`/`Atan` correctly, which the existing hand-rolled `eval_at` did not. Same trap applies to TypeScript test files when the test count grows past one phase block.
