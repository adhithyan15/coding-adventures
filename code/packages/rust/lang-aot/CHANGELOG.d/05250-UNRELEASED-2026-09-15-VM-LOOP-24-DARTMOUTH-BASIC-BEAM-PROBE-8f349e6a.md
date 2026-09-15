## Unreleased — 2026-09-15 — VM-LOOP-24 Dartmouth BASIC BEAM: probe-first general arithmetic/control flow + math builtins

A probe-first sweep of every not-yet-`Beam`, non-`INPUT` Dartmouth BASIC
`lang_matrix` row (24 total) against real `erl`, using only the
`iir-to-beam` capabilities that existed before this slice. Design + full
probe transcript: `code/specs/BEAM03-float-lowering.md` §9.

12 rows ran completely unchanged (zero new lowering): general `FOR`/`FOR …
STEP` loops, `IF … THEN` + `GOTO`-style jumps, a same-module `DEF FN` call,
multi-item `PRINT` with `;`/`,`, ordinary scalar real arithmetic, BA7
fixed-decimal and significant-digit/`E`-notation formatting, flat and
nested `GOSUB`/`RETURN`, and `SGN` — promoted via the new
`portable_text_stdout_dartmouth_basic_beam_general_arithmetic_and_control_flow`
test.

`iir-to-beam` 0.12.0 adds single-argument `math:*` transcendentals
(`f64_sqrt`/`f64_sin`/`f64_cos`/`f64_ln`/`f64_exp`/`f64_atan`/`f64_tan`, all
`call_ext` against `math:sqrt|sin|cos|log|exp|atan|tan/1` — confirmed via
`erlc -S` to be ordinary library calls, not guard BIFs, mirroring
`f64_pow`'s shape with one operand instead of two) and
`real_to_int_floor` (`erlang:floor/1` via `gc_bif1` — confirmed a genuine
guard BIF, joining `int_to_real`/`real_to_int_trunc`'s existing arm instead
of the `math:*` `call_ext` family). This promotes 5 more rows — `SQR`,
`SIN`/`COS`/`LOG`/`EXP`, `INT`, `ATN`, `TAN` — via the new
`portable_text_stdout_dartmouth_basic_beam_math_builtins` test.

Dartmouth BASIC now declares 39/51 rows on `Beam` (up from 22/51).
`feature_coverage_doc_counts_match_programs_source` updated
(`(51, 379)` → `(51, 396)`); `LANG-VM-FEATURE-COVERAGE.md` updated to match.

Three groups remain explicitly deferred, each a genuine design question
rather than a quick probe (see `LANG-VM-NON-ALGOL-BACKLOG.md`'s
"VM-LOOP-24" section for the reprioritization note):

- 1-D/2-D numeric arrays and `DATA`/`READ`/`RESTORE` (4 rows) compile but
  trap at runtime: `iir-to-beam` represents arrays via Erlang's `atomics`
  module, which stores 64-bit INTEGERS ONLY — `atomics:put/3` raises
  `badarg` on any float value, confirmed directly on real `erl`. Needs a
  real BEAM float-array data-representation decision, not an opcode.
- String arrays and mixed numeric/string `DATA` (2 rows) fail
  `iir-to-beam` validation outright — no `str`-typed array element
  representation exists at all.
- `RND` (1 row) still traps with a module-global `erlang:get/1` read
  returning `undefined` — confirmed to be the same "DEF-FN-and-module-global
  chain" open design question flagged at VM-018, not fixed as a side effect
  of `real_to_int_floor`.

