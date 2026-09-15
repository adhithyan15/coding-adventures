## 0.1.9 — 2026-08-13 — baseline regenerated after multi-value blocktypes were implemented (WASM04)

No code changes in this crate — `wasm-wast-parser` 0.1.6 and `wasm-execution`
0.6.7 added support for multi-value `block`/`loop`/`if` blocktypes (a
blocktype that's a type-section index, not just the MVP's empty/single-valtype
byte). Baseline regenerated: `assert_return` 13512/13521 (99.9%) →
13775/13780 (99.98%, +263 pass, -4 fail). `module` 98/98 → 102/102 (+4).
Verified via a full per-file diff:

- `block.wast`, `if.wast`, and `loop.wast` — previously failed to parse
  at all — now parse and pass in full (`assert_return` 52/52, 123/123,
  78/78 respectively).
- `fac.wast` also newly parses (unrelated pre-existing gap this happened
  to close too).
- `br.wast` (75/76 → 76/76) and `func.wast` (93/96 → 96/96) each had a
  handful of previously-failing `assert_return` cases fixed by the same
  interpreter change.
- No regressions: every file that changed strictly gained passes; nothing
  that previously passed now fails or is newly unsupported.

See `wasm-wast-parser`'s `0.1.6` and `wasm-execution`'s `0.6.7` changelog
entries for the full bug writeups.

