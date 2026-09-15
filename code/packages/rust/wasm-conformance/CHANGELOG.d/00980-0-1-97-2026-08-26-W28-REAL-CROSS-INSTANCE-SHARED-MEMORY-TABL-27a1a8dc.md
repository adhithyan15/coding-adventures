## 0.1.97 — 2026-08-26 — W28: real cross-instance shared memory/table, 5 files vendored

### Context

A prior batch-vendoring session investigated `elem.wast`/`instance.wast`/
`linking0.wast`/`linking1.wast`/`linking3.wast`/`load1.wast` and found all
six blocked on the same real architectural gap: `RegistryHost::
resolve_memory`/`resolve_table` (like `wasm-runtime::instantiate()`'s
import-resolution path generally) returned an OWNED, independently-cloned
`LinearMemory`/`Table`, not a shared live view — a genuine interpreter
correctness bug (a write through one instance's imported memory/table was
invisible through the exporting instance, and vice versa), not just a
conformance-corpus gap. Fixed at the source in `wasm-execution` 0.9.73
(`LinearMemory`/`Table`'s mutable storage now lives behind
`Rc<RefCell<..>>` — see that crate's own CHANGELOG for the full
rationale) plus a second, previously-unobservable bug this fix surfaced
in `wasm-runtime` 0.6.13 (active element-segment application wasn't
atomic per segment — see that crate's own CHANGELOG).

Verified via a throwaway reproduction BEFORE trusting the prior report:
`wasm-runtime/tests/shared_memory_table_import.rs`'s three new tests
build a two-instance import scenario directly (bypassing the `.wast`
corpus entirely) and genuinely FAIL against the pre-fix code, confirming
the bug is real, not inferred.

### Added — 5 files vendored to `TESTSUITE_FILES`

`elem.wast`, `linking0.wast`, `linking1.wast`, `linking3.wast`,
`load1.wast`. Real, measured per-file numbers (`testsuite-status.json` is
the source of truth, not this entry):

- `linking1.wast` / `load1.wast`: **100% pass** on every graded directive
  (7/7 and 15/15 `assert_return` respectively, plus their
  `assert_unlinkable`/`module`/`register` directives) — both exercise
  ONLY shared-memory-import scenarios, no table funcref dispatch, so the
  storage-sharing fix alone fully resolves them.
- `linking0.wast`: `assert_trap` 1/1, `assert_unlinkable` 2/2, `module`
  1/1, `register` 1/1, `assert_return` 0/1. The one `assert_return`
  fail hits the deliberately out-of-scope remaining gap: a funcref
  written into the shared table by one module and `call_indirect`-invoked
  through the exporting module needs real cross-instance function
  IDENTITY, which this PR does not add (see `wasm-execution`'s `Table`
  doc comment).
- `linking3.wast`: `module` 2/2, `register` 2/2, `assert_unlinkable` 4/4,
  `assert_return` 5/6 — same one known-gap fail as `linking0.wast`
  (a table-funcref-dispatch case), everything else passes, including the
  file's own "store is modified if the start function traps" partial-
  instantiation-persistence assertions.
- `elem.wast`: `module` 25/26, `register` 2/3, `assert_return` 13/19 (+8
  not yet supported), `assert_trap` 1/1 (+6 not yet supported),
  `assert_invalid` 9/9 (+17 not yet supported), `assert_unlinkable`
  12/12. Real, substantial, non-zero pass rate; the large
  not-yet-supported counts are two PRE-EXISTING, unrelated gaps this PR
  does not touch — heavy `spectest` import usage (this crate has no
  `spectest` host, by design, see `RegistryHost`'s own doc comment) and
  declarative (`elem declare ...`) segments (a third element-segment mode
  this crate doesn't parse yet, distinct from active/passive).

Existing corpus impact: the already-vendored `linking.wast`'s
`assert_return` tally improved 48/65 -> 54/65 with ZERO regressions
anywhere else in the corpus — every one of the other 215 already-vendored
files' pass/fail/trap/not_yet_supported tallies are byte-identical
before and after this change (programmatically diffed, not eyeballed).

### Deferred — `instance.wast`

Investigated but NOT vendored: needs a `(module definition $M ...)` /
`(module instance $I1 $M)` generative-instantiation directive form (one
named module definition, instantiated multiple times into independent
live instances) that `wasm-wast-parser`'s script grammar has zero support
for at all — a distinct, self-contained grammar-plus-`Executor` feature,
not blocked BY the storage-sharing fix in this PR, just out of scope for
it. Left as an explicit, named remaining gap rather than silently
skipped.

