## 0.1.88 — 2026-08-25 — Exceptions proposal, first slice W21: vendor tag.wast + throw.wast

### Added

- Vendored `tag.wast` (976 bytes) and `throw.wast` (1920 bytes) verbatim
  (pinned commit `28864811cf03bdbf880733786148feaba339582d`). Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py`.
- New `DirectiveKind::AssertException` (report schema) and
  `ActionError::Exception(String)` (distinguished from `ActionError::
  Trap` by `TrapError.is_exception` at the one call site that converts a
  `TrapError` into an `ActionError`) — `assert_exception` passes ONLY on
  a real uncaught exception, failing on both a normal return AND an
  ordinary trap (a real spec distinction: `try_table` never catches a
  trap, only an exception). `AssertReturn`/`AssertTrap`/`AssertExhaustion`
  grading updated symmetrically: none of them accept an exception in
  place of their own expected outcome.
- `WasmRuntime::instantiate` (`wasm-runtime`) gained an
  `ImportTypeInfo::Tag` arm: cleanly link-fails ("unknown import" —
  reuses `is_link_error`'s existing substring classifier, so this grades
  `NotYetSupported`, a real capability gap, not a hard `Fail`) since
  `HostInterface` has no `resolve_tag` method. `tag.wast`'s own
  tag-importing module has no subsequent `invoke`/`assert_return`
  exercising it, so only the bare `Directive::Module` itself is graded.

### Result (real, verified — full baseline diff, zero regressions elsewhere)

- `tag.wast`: `module` 1/1 real pass (+3 `NotYetSupported`: the tag-import
  module via the link-failure path above, and the two `(rec ...)`-using
  "link-time typing" modules — same `(rec ...)` gap W20 already named for
  GC). `register` 1/1 pass (+1 NotYetSupported). `assert_invalid` 2/2
  real pass ("non-empty tag result type"). `assert_unlinkable` 2/2 real
  pass.
- `throw.wast`: `module` 1/1 real pass. `assert_return` 1/2 (the one real
  pass: `throw-if 0 -> 0`; ONE deliberate, reviewed `Fail`:
  `test-throw-1-2`, the file's only test needing REAL `try_table` catch
  matching — explicitly out of this slice's scope, see `code/specs/
  W21-wasm-exceptions-tag-throw-slice.md`). `assert_invalid` 3/3 real
  pass. `assert_exception` 7/7 real pass (new directive kind).
- Aggregate deltas (before -> after, full-corpus baseline diff): `module`
  1278->1280 pass (+2), `NotYetSupported` 71->74 (+3, fail count
  UNCHANGED at 1); `register` 11->12 pass (+1, fail UNCHANGED at 2);
  `assert_invalid` 2033->2038 pass (+5); `assert_unlinkable` 49->51 pass
  (+2, fail UNCHANGED at 1); `assert_return` 44713->44714 pass (+1),
  `fail` 17->18 (+1, the one deliberate `test-throw-1-2` Fail); new
  `assert_exception` 7/7 pass. No other already-vendored file's stats
  changed.

