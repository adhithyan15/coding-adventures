## Unreleased — 2026-09-15 — BEAM04 Dartmouth BASIC BEAM: ets-backed float array representation

VM-LOOP-24 found 4 Dartmouth BASIC `lang_matrix` rows (1-D array, 2-D array,
`DATA`/`READ`/`RESTORE`, fractional `DATA`) that compile cleanly but trap at
runtime with `{badarg,[{atomics,put,[Ref,Index,FloatValue],...}]}`:
`iir-to-beam` represented every `alloc_array`/`array_set`/`array_get` with
Erlang's `:atomics` module, which can only hold 64-bit INTEGERS. BASIC
arrays are `array<f64>` (BA7-1b routes every scalar numeric value through
the shared f64 track), so every BASIC array write traps. Full research +
decision: `code/specs/BEAM04-float-array-representation.md`.

Two candidates were researched concretely against real `erl`/`erlc -S`, not
guessed at: bit-reinterpreting the f64 as an i64 via BEAM bit-syntax (the
bit pattern round-trips exactly, but needs three entirely new opcode
families this backend has never implemented — `bs_create_bin`,
`bs_start_match4`/`bs_match`, `test bs_get_float2`) versus switching to
`:ets` (needs ZERO new BEAM opcodes — `ets:new/2`/`ets:insert/2`/
`ets:lookup_element/3` are ordinary `call_ext`s, the same shape `math:*`
already uses). `:ets` was chosen — the backlog's own prior assumption that
reusing `:atomics` would "likely" be simpler did not hold up once actually
measured.

`iir-to-beam` 0.13.0 dispatches `alloc_array`/`array_set`/`array_get` with
`type_hint == "array<f64>"`/`"f64"` to `:ets` instead of `:atomics`. Every
other array/tape use (Brainfuck's byte tape, the GOSUB return-address
`array<i64>` stack, the `DATA` pool's kind array) is untouched. This
promotes all 4 rows via the new
`portable_text_stdout_dartmouth_basic_beam_arrays_and_data` test, each
executed against real `erl` before promotion.

Dartmouth BASIC now declares 43/51 rows on `Beam` (up from 39/51).
`feature_coverage_doc_counts_match_programs_source` updated
(`(51, 396)` → `(51, 400)`); `LANG-VM-FEATURE-COVERAGE.md` updated to match.

A known, documented limitation was found and left unfixed, not exercised by
any promoted row: unlike `atomics:new`, `ets:new` does not pre-zero N
cells, so reading a never-`array_set` index traps (`badarg`) instead of
returning `0.0`. Pinned by a dedicated `iir-to-beam` test rather than left
to regress silently.

Still explicitly out of scope: string-typed arrays and mixed numeric/string
`DATA` (2 rows — a separate, larger gap; `iir-to-beam` has no `str`-typed
array element representation at all), `RND` (1 row — VM-018's
DEF-FN-and-module-global design question, unaffected by array
representation), and the 5 `INPUT` rows (VM-060b, unrelated to arrays).
