## 0.1.91 — 2026-08-26 — Exceptions proposal, fourth slice W24: real exnref/catch_ref/throw_ref

### Added

- Vendored `throw_ref.wast` (pinned SHA
  `28864811cf03bdbf880733786148feaba339582d`) — every directive in this
  file uses only plain, abstract `exnref` (never a concrete `(ref $t)`
  reference type), matching W24's own deliberate scope boundary.

### Fixed

- Real corpus, measured (`cargo run --bin wasm_conformance_report`, full
  baseline diff confirming ONLY `throw_ref.wast` (new) and `try_table.wast`
  moved — every other file's stats are byte-identical):
  - `throw_ref.wast` (new file): `module` 1/1, `assert_return` 5/5,
    `assert_invalid` 2/2, `assert_exception` 7/7 — 100% real pass, zero
    not-yet-supported.
  - `try_table.wast`: `assert_return` 28 pass / 10 fail / 5
    not-yet-supported → **38 pass / 0 fail / 5 not-yet-supported** (the
    `throw-catch_ref-param-{i32,f32,i64,f64}` cluster, previously real
    fails since `catch_ref`/`catch_all_ref` never matched, now real
    passes). `assert_invalid` 5 pass / 4 not-yet-supported → **7 pass / 2
    not-yet-supported** (two `catch_ref`/`catch_all_ref` target-arity
    mismatch cases now correctly rejected). The remaining 5
    not-yet-supported `assert_return` directives and 2 not-yet-supported
    `assert_invalid` directives are the file's one module using non-null
    CONCRETE reference types (`(ref $t)`/`(ref exn)`) — confirmed
    unrelated to this slice, genuinely out of scope (see W24's own scope
    section).
  - Aggregate: `assert_return` 44743/27/585 → 44758/17/585 pass/fail/NYS;
    `assert_invalid` 2043/96 → 2047/94 pass/NYS; `assert_exception` 11 →
    18 pass. Zero regressions anywhere else in the 140-file corpus.

See `code/specs/W24-wasm-exceptions-exnref-catch-ref.md`.

