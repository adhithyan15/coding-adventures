## 0.1.34 — 2026-08-17 — vendor table.wast; baseline regen (task #99)

### Changed

- Baseline regen: vendored `table.wast`, previously deferred as blocked
  on hex-literal table limits and a `spectest` import -- both rescoped
  as tractable this session (see `wasm-wast-parser`'s own CHANGELOG
  entry for the hex-literal fix; the `spectest` directive grades
  `NotYetSupported` via the harness's existing unresolved-import
  handling, same pattern `linking.wast` already established).
- One genuine, EXPECTED `module` failure, not a bug: `(module
  definition (table 0xffff_ffff funcref))` declares a table with a
  4-billion-element minimum. The real spec permits arbitrarily large
  declared minimums; this interpreter's own `MAX_TABLE_ELEMENTS`
  resource-limit guard (task #96/#98, a deliberate DoS-safety
  tradeoff) rejects it. This is the same class of documented,
  accepted divergence as the pre-existing 17 `assert_return`/2
  `register`/1 `assert_unlinkable` failures already in the baseline --
  not something to "fix" by loosening the cap.

