## 0.1.98 — 2026-08-26 — W26 follow-up: real table64 operations, 9 files vendored

Vendors the 9 real corpus files `code/specs/W26-wasm-table64-first-slice.md`
named as the explicitly-deferred "real table64 operations" follow-up:
`call_indirect64.wast`, `table_copy64.wast`, `table_fill64.wast`,
`table_get64.wast`, `table_grow64.wast`, `table_init64.wast`,
`table_set64.wast`, `table_size64.wast`, `table_copy_mixed.wast` — now
unblocked by `wasm-execution` 0.9.74/`wasm-validator` 0.2.72's
`table.get`/`table.set`/`table.grow`/`table.size`/`table.fill`/
`table.copy`/`table.init`/`call_indirect` `is64` operand-width support,
`wasm-runtime` 0.6.14's matching active-element-segment `is64` fix, and
`wasm-wast-parser` 0.1.83's inline-elem-shorthand offset fix.

### Real, measured per-file numbers (all ZERO real `fail`)

| file | module | assert_return | assert_trap | assert_invalid |
|---|---|---|---|---|
| `call_indirect64.wast` | 1/1 | 1/1 | — | — |
| `table_get64.wast` | 1/1 | 5/5 | 4/4 | — |
| `table_set64.wast` | 1/1 | 10/10 | 8/8 | — |
| `table_grow64.wast` | 1/1 | 15/15 | 6/6 | — |
| `table_size64.wast` | 1/1 | 36/36 | — | — |
| `table_fill64.wast` | 1/1 | 64/64 | 6/6 | 9/9 |
| `table_copy64.wast` | 41/41 (+11 nys) | 334/334 (+109 nys) | 760/760 (+446 nys) | — |
| `table_init64.wast` | 26/26 (+18 nys) | 120/120 (+1 nys) | 158/158 (+476 nys) | 67/67 |
| `table_copy_mixed.wast` | 1/1 | — | — | 3/3 |

`table_copy64.wast`'s and `table_init64.wast`'s numbers are near-identical
to their already-vendored is32 siblings `table_copy.wast`/`table_init.wast`
— both new files are largely mechanical i32→i64 transforms of already-
passing corpus, confirming the is64 operand-width implementation is
correct rather than coincidentally passing. The `not_yet_supported` (nys)
counts are pre-existing, unrelated capability gaps (`spectest`-imported
modules this crate has no host for by design; `table_init64.wast`'s
trailing module additionally uses WasmGC `array`/`arrayref`, which this
crate doesn't support at all).

Full JSON diff of `testsuite-status.json` against the pre-change baseline
confirmed zero existing file's tally changed — only these 9 new files
moved.

### Two real, previously-latent bugs found and fixed while vendoring

Neither was reachable before this batch (no `is64` table existed in any
previously-vendored file):

- `wasm-runtime::instantiate()`'s active element-segment application read
  every segment's offset expression as `i32` unconditionally, even
  though the sibling active data-segment branch was already `is64`-aware
  (W25) — trapped instantiation for any active segment on an `is64`
  table. Found via `call_indirect64.wast`.
- `wasm-wast-parser`'s `(table i64 ... funcref (elem ...))` inline-elem
  shorthand hardcoded its generated segment's offset to `i32.const 0`
  regardless of `is64` — same symptom, same file.

