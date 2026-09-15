## 0.1.126 — 2026-09-02 — regenerated baseline: `array.init_data`/`array.new_data` + an `array.copy` bug fix -- W38 slice 3 (4 files, 117 directives)

No code changes in this crate — regenerated `tests/fixtures/testsuite-
status.json` (`--write-baseline`) after `wasm-wast-parser`/`wasm-
execution`/`wasm-validator` implemented `array.new_data`/`array.init_data`
(`code/specs/W38-wasm-gc-array-bulk-ops.md` slice 3), and after a real,
corpus-caught `array.copy` validation bug (found by this slice unblocking
`array_copy.wast`'s own module parse for the first time — see `wasm-
validator`'s own CHANGELOG for the full trace) was fixed in the same PR.
Diffed programmatically against the pre-slice baseline across all 257
files — exactly FOUR files' tallies changed, zero elsewhere:

- `array_init_data.wast`: 2/46 → 46/46 pass (44 `not_yet_supported` → 0).
- `array_new_data.wast`: 0/28 → 28/28 pass (28 `not_yet_supported` → 0).
- `array_copy.wast`: 4/35 → 35/35 pass (31 `not_yet_supported` → 0) — this
  is the dependency `wasm-conformance`'s own prior 0.1.125 entry flagged
  ("Expected to resolve once a later W38 slice implements `array.new_
  data`"), CONFIRMED resolved here, not merely assumed: re-probed directly
  before writing this entry.
- `array.wast`: gains 14 more real `Pass` directives (module 4→5,
  assert_return 10→17, assert_trap 6→12; assert_invalid unchanged 6/6) —
  its own `array.new_data`-attributable subset, exactly matching this
  spec's own predicted count. Its remaining 14 `not_yet_supported`
  directives (module 2, assert_return 7, assert_trap 5) are exclusively
  the elem-segment-sourced instruction family (`array.new_elem`/`array.
  init_elem`) plus the one pre-existing, out-of-scope `(ref struct)` case
  W37 already flagged — none attributable to this slice.

Zero regressions: every other file's tallies are byte-for-byte identical
to the pre-slice baseline (re-verified via a programmatic Python diff of
the `files` dict, not eyeballed), `fail`/`trap` counts are `0` everywhere
both before and after, and `parse_failures` is empty both before and
after.

