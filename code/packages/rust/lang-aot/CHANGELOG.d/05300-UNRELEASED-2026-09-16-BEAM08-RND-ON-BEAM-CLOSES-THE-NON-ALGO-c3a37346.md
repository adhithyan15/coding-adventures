## Unreleased — 2026-09-16 — BEAM08: RND on BEAM closes the non-ALGOL BEAM matrix (VM-018)

Closed VM-018, the LAST remaining non-ALGOL BEAM gap in
`LANG-VM-NON-ALGOL-BACKLOG.md`: Dartmouth BASIC's `RND` trapped on real
`erl` with `{badarith,[{erlang,'*',[undefined,48271],...}]}` — a
module-global read returning `undefined` instead of the seed `main` was
supposed to have stored. Full research + decision:
`code/specs/BEAM08-rnd-beam-support.md`.

**Root cause was NOT a new design question.** `RND`'s frontend-emitted
`__basic_rnd` helper shares its Park–Miller state through the exact same
`global_store`/`global_load` module-global substrate every other
module-level BASIC/COBOL/Twig variable already uses — no VM-018-specific
IIR op, no bespoke substrate. The trap was entirely caused by
`global_store`'s PRE-EXISTING `gc_bif2`-for-`erlang:put/2` bug, discovered
and deliberately deferred in BEAM07 as issue #15332: `put/2` is not a
guard-safe BIF, so `gc_bif2`'s `Live` count cannot correctly protect the
call, and `main`'s seed store silently failed to stick.

**Fixed issue #15332 as a genuine prerequisite** (confirmed concretely,
not assumed: converting `global_store` alone, with ZERO changes to
`dartmouth-basic-iir-compiler`'s existing RND lowering, makes the exact
promoted row produce the correct Park–Miller sequence) — see
`iir-to-beam`'s own changelog for the full accounting. Converted
`global_store` from `gc_bif2` to `call_ext`, mirroring BEAM07's own fix
for its `input_more`/`input_i64`/`input_str` `put/2` calls, and added
`global_store` to the `live_across` liveness filter per the issue's own
scope note. Closes [#15332](https://github.com/adhithyan15/coding-adventures/issues/15332).

Proven correct on real `erl`, not just "compiles" — "probe before
declaring, promote only proven cells": `portable_text_stdout_dartmouth_
basic_beam_rnd` executes the exact promoted row (`FNR(-1)` reseed,
`RND(1)` advance, `FNR(0)` repeat, `RND(1)` advance again) and confirms
stdout `22\n85032\n85032\n601352` — byte-for-byte the same sequence every
other standard backend (NativeAOT, LLVM, WASM, JVM, CLR, VM, JIT) already
proves.

`iir-to-beam` 0.16.0 → 0.17.0: `global_store`'s `call_ext` conversion +
`live_across` fix; `test_114_global_store_emits_call_ext_not_gc_bif2`
(instruction-shape) and `test_115_real_erl_global_store_survives_live_
across_call` (the disposable control test from issue #15332's own
reproduction, made permanent — a co-live `str_const` heap value now
survives the call intact, and the stored global reads back correctly).
118 tests total (up from 116); `cargo clippy --all-targets -- -D
warnings` clean.

`lang-aot` 0.345.0 → 0.346.0: the `RND` row promoted to declare `Beam` —
its 8th and final backend. `feature_coverage_doc_counts_match_programs_
source` updated (Dartmouth BASIC tuple `(51, 407)` → `(51, 408)`);
`LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth BASIC row and grand-total prose
updated to match. Full `iir-to-beam` suite (118 tests) and the full
`lang_matrix.rs` BEAM test group (34 tests, spanning every language that
uses `global_store`) re-run clean — zero regressions from widening
`global_store`'s blast radius. A full `non_algol_matrix_every_proven_
cell_agrees` capstone rerun also passed clean (210 programs, 1467 cells
exercised).

**Dartmouth BASIC now declares all 51/51 rows on `Beam`.** Combined with
Twig (49/49), Nib (26/26), Oct (12/12), COBOL-60 (58/58), FLOW-MATIC
(8/8), and Brainfuck (3/6, intentional — real stdin-as-tape host support
is a separate, unscoped item), **this closes every non-ALGOL BEAM gap in
`LANG-VM-NON-ALGOL-BACKLOG.md`** — the multi-month non-ALGOL BEAM
completion track this backlog has been driving since BEAM01.
