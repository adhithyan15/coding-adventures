## 0.1.24 — 2026-08-15 — v128 invoke arguments; baseline regen (task #86, W15 follow-up)

### Fixed

- `run_action`'s `Action::Invoke` arm rejected any `(v128.const ...)`
  invoke ARGUMENT with `NotYetSupported` -- at the time that code was
  written (SIMD PR1b-3), no live heap existed before a call started to
  allocate its handle into. Now that `WasmInstance.v128_heap` is
  persistent and exists from `instantiate()` onward (W15, task #79), a
  v128 argument allocates directly into it, the same "push and return
  the new index" shape `evaluate_const_expr`/`push_v128` already use --
  a real `WasmValue::V128(handle)`, not a synthesized/placeholder one.
  Bounds-checked against `wasm_execution::MAX_V128_HEAP_LEN` (now `pub`
  in `wasm-execution` 0.9.3 for exactly this reuse).
- Existing test `invoke_with_a_v128_argument_grades_not_yet_supported_
  not_a_silent_wrong_pass` renamed to `invoke_with_v128_arguments_
  passes_for_real` and rewritten to assert the new, correct outcome
  (`Pass`, byte-exact) instead of the old capability-gap `NotYetSupported`.

### Changed

- Baseline regen following `wasm-execution` 0.9.3: `simd_const.wast`'s
  `assert_return` tally moves from 235/240 (4 fails) to 243/243 (fully
  clean, 0 fails, 0 traps -- the file's directive count itself also
  changed slightly, since some `NotYetSupported` invoke-argument cases
  now grade as real `Pass`es instead). Aggregate `assert_return` across
  the 61-file vendored corpus moves to 15523/15523 (100%, zero fails
  anywhere). Zero regressions (full before/after diff of every file's
  per-kind tally).

