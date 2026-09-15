## 0.1.95 — 2026-08-26 — vendor return_call.wast / return_call_indirect.wast (W11 addendum)

### Added

- Vendored the real, pinned-commit `return_call.wast` and
  `return_call_indirect.wast` to `TESTSUITE_FILES` — closes the one
  backlog item W11's own tail-call spec explicitly left open. Both files
  were blocked on a single narrow construct present in each: a "Result
  subtyping" helper declaring `(func $f (result (ref null $t))
  (ref.null $t))`, a nullable reference to a CONCRETE function type —
  function-references-proposal grammar this crate had no support for at
  all. Unblocked by `wasm-types` 0.1.12's `ValueType::ConcreteFuncRef`,
  `wasm-wast-parser` 0.1.81's `(ref null $t)`/`ref.null $t` support, and
  `wasm-validator` 0.2.70's matching one-directional subtyping rule.
- Real, measured numbers (see `tests/fixtures/testsuite/NOTICE` for the
  full accounting; zero new `fail` anywhere in either file):
  `return_call.wast` — module 2/2 pass (+1 not_yet_supported),
  assert_invalid 12/12 pass, assert_return 0/34 (all not_yet_supported).
  `return_call_indirect.wast` — module 2/2 pass (+1 nys), assert_invalid
  15/17 pass (+2 nys), assert_return 0/43, assert_trap 0/7,
  assert_malformed 0/11 (all not_yet_supported). The not_yet_supported
  directives are pre-existing, already-documented gap categories, not new
  ones: both files' main module imports `spectest` (this crate has no
  `spectest` host, by design), and `return_call_indirect.wast`'s
  malformed cases need inline-signature clause-ordering validation this
  crate's parser doesn't do (same class of gap several other already-
  vendored files have).

