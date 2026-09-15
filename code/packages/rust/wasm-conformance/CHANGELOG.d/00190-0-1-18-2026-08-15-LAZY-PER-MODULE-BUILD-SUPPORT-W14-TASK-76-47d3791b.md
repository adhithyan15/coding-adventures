## 0.1.18 — 2026-08-15 — lazy per-module build support (W14, task #76)

### Added

- `Executor` now handles `wasm-wast-parser` 0.1.13's `Directive::Module(
  Result<WasmModule, String>)`: a build failure grades `NotYetSupported`
  (a real capability gap, not a bug) instead of the directive not
  existing at all in a fully-aborted script.
- `current_link_failed: Option<String>` renamed and broadened to
  `current_module_status: Option<String>`, now covering BOTH build
  failures and link failures uniformly (previously link-failure-only).
  `run_action`'s two read sites simplified to surface the already-
  formatted reason directly. `Directive::Register`'s "no current module"
  arm now checks this field too: a broken current module (build or link
  failure) grades `NotYetSupported` on `register`, not the generic
  hardcoded `Fail` reserved for a genuine test-script-structure problem
  (no module directive ever ran at all).

### Fixed

- A real, previously rarely-exercised bug the same change surfaces and
  fixes: the module registry's `None` ("current module") slot was only
  ever WRITTEN on a successful `instantiate`, never CLEARED on any
  failure path -- so a module that failed structural validation or
  trapped during instantiation left the PREVIOUS module silently
  registered as "current," and a later bare `invoke`/`register` would
  operate on the wrong module instead of failing loudly. Fixed by
  unconditionally clearing the `None` registry slot at the top of every
  `Directive::Module` directive, before its outcome is even determined.
  Verified load-bearing via TEMP-REVERT-CHECK: reverting just the
  `registry.borrow_mut().remove(&None)` line reproduces the exact
  predicted false pass (a stale-module `Pass` where a `Fail` was
  expected) on a dedicated regression test, then restored.

### Baseline regen

- `tests/fixtures/testsuite-status.json` regenerated: 12 previously
  entirely-unparseable MVP corpus files (`br_table.wast`,
  `call_indirect.wast`, `const.wast`, `float_exprs.wast`,
  `float_memory.wast`, `global.wast`, `id.wast`, `memory.wast`,
  `memory_grow.wast`, `select.wast`, `stack.wast`,
  `unreached-valid.wast`) now parse and grade for real -- confirmed via
  a full JSON diff that every OTHER previously-present file's tallies
  are byte-for-byte unchanged (zero regressions, exactly these 12 files
  added, zero files removed). Aggregate: 60 files parsed, 0 failed to
  parse (up from 48 parsed / 12 failed).

See `code/specs/W14-wasm-conformance-lazy-module-build.md`.

