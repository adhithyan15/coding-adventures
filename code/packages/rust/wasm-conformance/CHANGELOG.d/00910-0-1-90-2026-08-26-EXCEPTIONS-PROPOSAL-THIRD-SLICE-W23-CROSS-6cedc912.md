## 0.1.90 — 2026-08-26 — Exceptions proposal, third slice W23: cross-instance tag identity

### Changed

- `RegistryHost::resolve_tag` now returns `(FuncType, u64)` — the
  exporting instance's own real, canonical tag identity
  (`instance.tag_identities[index]`, `wasm-runtime` 0.6.8) alongside its
  declared type, so an importing module's throw/catch of the SAME tag
  compares equal across the instance boundary.

### Fixed

- Real corpus, measured (`cargo run --bin wasm_conformance_report`, full
  baseline diff confirming ONLY `try_table.wast` moved): `assert_return`
  25 pass / 13 fail / 5 not-yet-supported → **28 pass / 10 fail / 5
  not-yet-supported** — `catch-imported`, `catch-imported-alias`, and
  `imported-mismatch` (the three cross-instance directives W22 named and
  deferred) now pass for real. The remaining 10 `assert_return` fails are
  all `catch_ref`/`exnref`-dependent directives, individually confirmed
  unrelated to this slice. See `code/specs/
  W23-wasm-exceptions-cross-instance-tag-identity.md`.

