## 0.1.30 — 2026-08-16 — vendor memory_init.wast; baseline regen (task #95)

### Changed

- Baseline regen: vendored `memory_init.wast` -- 100% pass, every
  directive kind, on the first regen after implementation. Needed real
  new interpreter state (`memory.init`/`data.drop` were entirely
  unimplemented) and real passive data segment support (`wasm-module-
  parser`'s binary decoder only ever handled segment-mode flag `0x00`
  before this task). Zero regressions anywhere else in the now-67-file
  vendored corpus (full before/after per-file/per-kind diff). Only uses
  single-memory numeric segment indices, so it's vendorable standalone
  -- unlike its siblings `bulk.wast` (task #97 still pending) and
  `memory-multi.wast` (task #92, now unblocked and vendorable as a
  near-zero-cost follow-up).

See `code/packages/rust/wasm-module-parser/CHANGELOG.md`, `code/
packages/rust/wasm-wast-parser/CHANGELOG.md`, `code/packages/rust/
wasm-execution/CHANGELOG.md`, and `code/packages/rust/wasm-validator/
CHANGELOG.md`/`code/packages/rust/wasm-runtime/CHANGELOG.md` for the
parsing, interpreter, and validation-layer changes this vendoring pass
needed.

