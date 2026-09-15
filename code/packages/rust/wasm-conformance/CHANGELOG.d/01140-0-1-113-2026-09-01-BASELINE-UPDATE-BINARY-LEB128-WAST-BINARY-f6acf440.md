## 0.1.113 — 2026-09-01 — baseline update: `binary-leb128.wast`/`binary.wast` LEB128 + data-count-section fixes

Baseline regen (`--write-baseline`) after the LEB128 under-strictness and
missing-data-count-section-gate fixes in `wasm-module-parser`/`wasm-
validator`/`wasm-types` (this repo's fresh corpus-wide prioritization
pass, `code/specs/W07-wasm-post-mvp-epics.md`'s "Addendum (2026-09-01)"
item 2 — see `wasm-validator`'s own CHANGELOG for the full root-cause
writeup, since that's where both fixes actually live). Direct-probed
(`run_wast_source` against each of the two suspect files individually,
one throwaway test per file, before touching any source) to identify
exactly which `assert_malformed` directives were failing and confirm
they all shared the harness's generic `"binary module parsed but should
have been rejected as malformed"` message — 7 in `binary-leb128.wast`
(of 58 total `assert_malformed` directives) and 2 in `binary.wast` (of
107), turning out to be two unrelated root causes once actually
root-caused rather than three assumed-identical LEB128 cases each.

### Corpus impact (programmatic before/after diff of `tests/fixtures/
testsuite-status.json`'s `files` map, all 257 vendored files)

Exactly 2 files changed:

- `binary-leb128.wast`: `assert_malformed` 51/58 (7 failing) -> 58/58 (0
  failing).
- `binary.wast`: `assert_malformed` 105/107 (2 failing) -> 107/107 (0
  failing).

Zero regressions anywhere else in the 257-file corpus. Notably including
`binary_leb128_64.wast`, which DID regress during an intermediate,
too-broad version of the fix (unconditionally bounding a memory
instruction's `offset` memarg field to 32 bits, rather than only when the
addressed memory isn't `is64`) — this diff is exactly what caught that
regression before it shipped; see `wasm-validator`'s own CHANGELOG for
the full story of why `offset`'s width is genuinely memory-dependent, not
a fixed 32 or 64 bits.

Aggregate `assert_malformed` tally: 1304 pass / 9 fail / 627 `not_yet_
supported` -> 1313 pass / 0 fail / 627 `not_yet_supported` (unchanged;
all pre-existing, unrelated gaps).

