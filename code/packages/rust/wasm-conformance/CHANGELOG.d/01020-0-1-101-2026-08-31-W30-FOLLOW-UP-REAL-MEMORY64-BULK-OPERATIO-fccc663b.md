## 0.1.101 — 2026-08-31 — W30 follow-up: real memory64 bulk operations, 13 files vendored

Vendors the corpus files `code/specs/W25-wasm-memory64-first-slice.md`
named as the explicitly-deferred "bulk memory ops on a 64-bit memory"
follow-up: `bulk64.wast`, `memory_copy64.wast`, `memory_fill64.wast`,
`memory_init64.wast`, `memory_grow64.wast`, `memory_redundancy64.wast`,
`memory_trap64.wast`, `address64.wast`, `align64.wast`,
`endianness64.wast`, `load64.wast`, `float_memory64.wast` — now unblocked
by `wasm-execution` 0.9.76/`wasm-validator` 0.2.73's `memory.copy`/
`memory.fill`/`memory.init` `is64` operand-width support. Plus one
unrelated census-flagged file, `bulk.wast` (the plain is32 bulk-memory
proposal test file, already fully implemented, simply never fetched).

### Real, measured per-file numbers (all ZERO real `fail`)

| file | module | action | assert_return | assert_trap | assert_invalid | assert_malformed |
|---|---|---|---|---|---|---|
| `bulk64.wast` | 5/5 | 20/20 | 38/38 | 7/7 | — | — |
| `memory_copy64.wast` | 33/33 | 15/15 | 4320/4320 | 18/18 | 64/64 | — |
| `memory_fill64.wast` | 11/11 | 5/5 | 14/14 | 6/6 | 64/64 | — |
| `memory_init64.wast` | 29/29 | 12/12 | 126/126 | 16/16 | 67/67 | — |
| `memory_grow64.wast` | 4/4 | — | 39/39 | 6/6 | — | — |
| `memory_redundancy64.wast` | 1/1 | 3/3 | 4/4 | — | — | — |
| `memory_trap64.wast` | 2/2 | — | 4/4 | 166/166 | — | — |
| `address64.wast` | 4/4 | — | 206/206 | 32/32 | — | — |
| `align64.wast` | 25/25 (+1 nys) | — | 47/47 | 1/1 | 37/37 | 0/46 (nys) |
| `endianness64.wast` | 1/1 | — | 68/68 | — | — | — |
| `load64.wast` | 1/1 | — | 37/37 | — | 46/46 | 13/13 |
| `float_memory64.wast` | 6/6 | 24/24 | 60/60 | — | — | — |
| `bulk.wast` | 10/10 (+3 nys) | 21/21 (+17 nys) | 35/35 (+13 nys) | 9/9 (+9 nys) | — | — |

("nys" = `not_yet_supported`, pre-existing capability gaps unrelated to
this batch, e.g. `bulk.wast`'s handful of directives needing a host this
crate deliberately doesn't implement.) `memory_grow64.wast` needed ZERO
`wasm-execution`/`wasm-validator` code changes at all — `memory.size`/
`memory.grow`'s `is64` awareness was already part of the W25 first slice,
confirmed by re-reading the existing code before assuming otherwise.
`address64.wast`/`align64.wast`/`endianness64.wast`/`load64.wast`/
`float_memory64.wast`/`memory_redundancy64.wast`/`memory_trap64.wast` are
all plain scalar load/store address-computation/alignment/endianness/trap
corpus splits with zero bulk-memory content (verified by reading each
file, not assumed from its name) — already fully covered by the first
slice's `pop_effective_addr` is64/is32 branch, vendored here purely
because nobody had fetched them yet.

Full JSON diff of `testsuite-status.json` against the pre-change baseline
confirmed zero existing file's tally changed — only these 13 new files
moved, and zero new `parse_failures` entries.

### Investigated and deliberately NOT vendored: `extern.wast`

The other census-flagged file. Read in full before deciding: NOT a
simple omission of the MVP-era externref this crate already supports
(W08) — every directive uses real WasmGC surface this crate has none of
(`anyref`, `any.convert_extern`/`extern.convert_any`, `struct.
new_default`, `array.new_default`, `ref.i31` interop, `(elem declare
func ...)` declarative segments), and its `assert_return` expected-value
position uses a `ref.host N` literal `wasm-wast-parser`'s constant-
expression parser doesn't recognize at all. Confirmed live (not
assumed): `FAILED TO PARSE SCRIPT: at byte 1150: expected a *.const
expression, found "ref.host"` — unlike the existing GC-proposal batch
(`ref.wast`, `call_ref.wast`, etc.), which all gracefully degrade to
all-`not_yet_supported` at the directive level, this file fails to parse
as a whole SCRIPT. This crate's own baseline has zero pre-existing
`parse_failures` entries — a real, load-bearing policy (only vendor
files that at least parse as a script) this file would be the first
exception to. Genuinely the same "non-null concrete reference types"
wall three independent prior sessions (W20/W23/W24) already found
blocked, reached from yet another direction — not a small fix, and not
bundled into this batch.

