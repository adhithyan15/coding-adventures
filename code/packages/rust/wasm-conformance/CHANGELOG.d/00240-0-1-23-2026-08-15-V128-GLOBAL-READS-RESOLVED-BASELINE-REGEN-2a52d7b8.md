## 0.1.23 — 2026-08-15 — v128 global reads resolved; baseline regen (W15, task #79)

### Fixed

- `run_action`'s `Action::Get` arm (a bare `(get "name")` action reading
  an exported global directly, not via a function call) used to always
  return `None` for a `WasmValue::V128` global's resolved bytes -- at
  the time that code was written (SIMD PR1b-3), there was no way to
  resolve a v128 handle outside of an active call's `ctx.v128_heap`.
  Now that `WasmInstance.v128_heap` is a persistent, directly-readable
  field (`wasm-runtime` 0.6.1, W15), a `Get` action resolves the handle
  against it exactly like a call's result does. No vendored corpus file
  currently exercises `(get ...)` against a v128 global, so this has no
  baseline-visible effect today, but closes the gap for real ahead of a
  future corpus file that does.

### Changed

- Baseline regen following `wasm-execution` 0.9.2 / `wasm-runtime` 0.6.1
  (W15, task #79 -- v128 persistent storage). `simd_const.wast`'s
  `module` tally moves from 308/309 (1 trap) to 309/309 (the module
  whose `(global (mut v128) (v128.const ...))` initializer previously
  failed to instantiate now builds); `assert_return` moves from 235/240
  (5 fails) to 235/240 (4 fails, `not_yet_supported` +1) -- one
  previously-collateral failure now correctly grades
  `NotYetSupported("a v128 invoke ARGUMENT is not yet supported...")`,
  a real, SEPARATE, already-documented gap (v128.const literals aren't
  yet supported as `invoke` arguments -- logged as task #86, NOT fixed
  by this PR). The remaining 4 fails are a direct, expected downstream
  consequence of that same NotYetSupported case (globals a skipped
  "set" call never actually set), not a new or masked bug. Zero
  regressions anywhere else in the 61-file vendored corpus (full
  before/after diff of every file's per-kind tally).

