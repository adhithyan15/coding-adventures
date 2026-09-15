## 0.1.87 — 2026-08-25 — GC epic, first slice W20: vendor i31.wast

### Added

- Vendored `i31.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`, byte-identical re-fetch,
  8347 bytes) -- the one GC-family file at this pinned SHA that is NOT
  entangled with `call_ref`, non-null concrete reference types,
  `(rec ...)` recursive type declarations, or the `eq`/`any`/`none`
  abstract heap-type hierarchy (every other GC-family file is, confirmed
  by direct inspection -- see `code/specs/
  W20-wasm-gc-i31-conformance.md`'s "Purpose" section for the full
  investigation). Added to `TESTSUITE_FILES` in `fetch_testsuite.py`
  (lives at the testsuite repo root, no `PROPOSAL_FILES` entry needed).
- `Expected::RefI31Any` grading arm in `value_matches_expected` -- see
  `wasm-wast-parser`'s own changelog entry for this same version.
- Regenerated the committed conformance baseline
  (`testsuite-status.json`; all pass/gradeable pairs below are
  pass/(pass+fail), `NotYetSupported` called out separately, matching
  this file's own established convention): `module` rose from 1275/1276
  to 1278/1279 (+3 pass, +3 gradeable, `NotYetSupported` rose from 67 to
  71); `register` rose from 10/12 to 11/13 (+1 pass, +1 gradeable,
  `NotYetSupported` unchanged at 2); `action` rose from 145/145 to
  146/146 (+1 pass, +1 gradeable, `NotYetSupported` rose from 29 to 36);
  `assert_return` rose from 44693/44710 to 44713/44730 (+20 pass, +20
  gradeable, `NotYetSupported` rose from 545 to 580); `assert_trap` rose
  from 1478/1478 to 1480/1480 (+2 pass, +2 gradeable, `NotYetSupported`
  unchanged at 938). Every fail count (assert_return: 17, module: 1,
  assert_unlinkable: 1, register: 2) UNCHANGED, and no other
  already-vendored file's stats moved at all (confirmed via a full
  per-file diff of the regenerated baseline against the pre-change one)
  -- zero regressions. The file's
  first module (`ref.i31`/`i31.get_s`/`i31.get_u` on plain
  params/results/globals, plus two more modules that happened to need no
  further infrastructure) grades for real; its later modules using
  tables/elem-segments of a non-`funcref` reference kind correctly grade
  `NotYetSupported` for their own directives, per W14's per-module
  build-failure isolation (no blast radius onto the passing modules).

