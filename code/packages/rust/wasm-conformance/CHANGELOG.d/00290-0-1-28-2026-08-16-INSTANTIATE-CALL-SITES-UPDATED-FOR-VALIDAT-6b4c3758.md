## 0.1.28 — 2026-08-16 — instantiate() call sites updated for ValidatedModule (task #100)

### Changed

- `wasm-runtime::WasmRuntime::instantiate()` now takes `&ValidatedModule`
  instead of `&WasmModule` (see `wasm-runtime`'s own CHANGELOG). This
  harness already called `validate()` before `instantiate()` -- it just
  discarded the `ValidatedModule` and re-passed `&validated.module`, so
  the fix is a one-line change per call site (`&validated` instead of
  `&validated.module`). No behavioral change; full baseline regen
  confirms byte-identical results.

