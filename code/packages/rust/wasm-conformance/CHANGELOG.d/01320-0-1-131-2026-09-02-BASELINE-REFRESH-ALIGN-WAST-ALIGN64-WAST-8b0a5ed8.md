## 0.1.131 — 2026-09-02 — baseline refresh: `align.wast`/`align64.wast`/`simd_align.wast` close (`wasm-wast-parser` 0.1.105)

`wasm-wast-parser` 0.1.105 fixed `parse_memarg` to reject `align=N` text-
format immediates that aren't a power of two (it previously computed
`n.trailing_zeros()` unconditionally, silently misreading `align=0` and
`align=7` instead of rejecting them) -- see that crate's own CHANGELOG
for the full root-cause account. This crate's own code is unchanged;
only `tests/fixtures/testsuite-status.json` moves, regenerated via
`--write-baseline` and diffed programmatically against the prior
baseline across all 257 files. Confirmed exactly three files changed, all
strictly `not_yet_supported` -> `pass` with zero new `fail`:
- `align.wast`: `assert_malformed` 2/48 (46 NYS) -> 48/48.
- `align64.wast`: `assert_malformed` 0/46 (46 NYS) -> 46/46.
- `simd_align.wast` (same shared root cause): `assert_malformed` 12/34
  (22 NYS) -> 34/34.

Aggregate `assert_malformed` moves from 1714/1940 (226 NYS) to 1828/1940
(112 NYS); every other directive kind and every other one of the 257
files is byte-for-byte unchanged from the pre-fix baseline.

