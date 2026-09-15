## 0.1.89 — 2026-08-25 — Exceptions proposal, second slice W22: vendor try_table.wast

### Added

- Vendored `try_table.wast` (13554 bytes, pinned SHA
  `28864811cf03bdbf880733786148feaba339582d`) — real catch/catch_all
  matching (`wasm-execution` 0.9.66) now grades most of this file's
  directives as genuine passes. Added to `TESTSUITE_FILES` in
  `fetch_testsuite.py`.
- `RegistryHost::resolve_tag` (new `HostInterface` impl method) — real
  cross-module tag import resolution for `register`-based linking, the
  same mechanism this crate's `resolve_function`/`resolve_global`/
  `resolve_memory`/`resolve_table` already provide.
- Regenerated `tests/fixtures/testsuite-status.json`: real gains in
  `tag.wast` (`module` 1/2 → 2/2, from the `resolve_tag` fix above),
  `throw.wast` (`assert_return` 1/2 → 2/2 — `test-throw-1-2`, W21's one
  documented `Fail`, now passes for real), and the newly-vendored
  `try_table.wast`. Confirmed via a full diff that no other
  already-vendored file's stats moved. See `code/specs/
  W22-wasm-exceptions-catch-clause-matching.md` for the exact numbers
  and the (13, all individually confirmed out-of-scope) remaining real
  fails.

