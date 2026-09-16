## Unreleased — 2026-09-15 — BEAM07: host input closes Dartmouth BASIC INPUT and FLOW-MATIC READ-ITEM

Closed VM-060b, the largest remaining non-ALGOL BEAM gap left in
`LANG-VM-NON-ALGOL-BACKLOG.md` after BEAM06: `call_builtin "input_i64"`/
`"input_str"`/`"input_more"` (BASIC `INPUT`/`INPUT A$`, FlowMatic
`READ-ITEM`'s EOF peek) had no BEAM lowering at all, and `run_beam`'s test
runner never piped a program's stdin to the spawned `erl` process in the
first place — every other subprocess backend (native/LLVM/JVM/CLR) already
did via `output_with_stdin`, but BEAM's `Command::new("erl")....output()`
inherited-but-unused stdin. Full research + decision:
`code/specs/BEAM07-beam-host-input.md`.

**Harness fix**: `run_beam` now builds its `erl` command and pipes
`program_stdin(p)` through `output_with_stdin`, exactly like every other
subprocess runner. A pure extension — every other cell's `program_stdin`
returns `b""`, so `write_all(b"")` is a no-op for them.

**IIR lowering** (`iir-to-beam` — see its own `CHANGELOG.md` for the full
accounting): `input_i64`/`input_str` are a consuming read (`io:get_line/1`
then `string:to_integer/1` or `string:trim/3`, both confirmed as genuine
`call_ext`s on real `erl`, never guard BIFs); `input_more` peeks a
one-line lookahead cached in the process dictionary under a private key,
consumed exactly once by the following consuming read via
`erlang:put/2`'s "returns the OLD value" contract.

**Two confirmed pre-existing framework bugs found via real-`erl` access
violations, not assumed** — full detail in `iir-to-beam`'s own changelog
and `BEAM07-beam-host-input.md` §3.3: (1) `erlang:put/2` must lower to
`call_ext`, never `gc_bif2` — the existing `global_store` lowering (and
this slice's own first cut) used `gc_bif2`, which corrupts a SEPARATE
heap-allocated value also live across the call, something no prior op
combination ever exercised until two sequential `input_str`/`input_i64`
reads did. (2) `allocate`'s Y-register slots need explicit
zero-initialization (a real `erlc` always emits `init_yregs`; this backend
never did) — latent for the identical reason. Fixed generally, not just
for the new ops. `global_store`'s own `gc_bif2`-for-`put/2` bug is
confirmed but explicitly NOT fixed here — flagged below for a dedicated
follow-up.

Proven correct on real `erl`, not just "compiles" — "probe before
declaring, promote only proven cells":
`portable_text_stdout_dartmouth_basic_beam_input` (5 rows: numeric `INPUT
X`; two sequential `INPUT`s summed; a branch-selected string chosen by a
runtime `INPUT N`; string `INPUT A$`; two string `INPUT`s concatenated)
and `portable_text_stdout_flow_matic_beam_read_item_and_eof` (4 rows: a
`READ-ITEM`/`IF END OF DATA` loop over two records then EOF; the same loop
over a genuinely empty file; a two-field-per-record loop; two sequential
bare `READ-ITEM`s with no EOF check) — each executed against real `erl`
with its declared stdin piped through the fixed `run_beam` before being
promoted.

`iir-to-beam` 0.15.0 → 0.16.0: new `call_builtin` lowering for
`input_more`/`input_i64`/`input_str`; the `erlang:put/2`-as-`call_ext` and
`allocate`-Y-slot-zero-init fixes; 11 new tests (105 → 116, `test_103`
through `test_113`), all real-`erl` round-trips where execution matters;
`cargo clippy --all-targets -- -D warnings` clean.

`lang-aot` 0.344.0 → 0.345.0: `run_beam`'s stdin pipe; the 5 Dartmouth
BASIC `INPUT` rows and 4 FLOW-MATIC `READ-ITEM`/EOF rows promoted to
declare `Beam`, each gaining exactly one new declared cell.
`feature_coverage_doc_counts_match_programs_source` updated (Dartmouth
BASIC tuple `(51, 402)` → `(51, 407)`, FLOW-MATIC tuple `(8, 60)` →
`(8, 64)`); `LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth BASIC and FLOW-MATIC
rows and grand-total prose (1667 → 1676) updated to match. The full
existing `iir-to-beam` (116 tests) and `lang_matrix` BEAM test groups (all
pre-existing BEAM tests plus the 2 new ones) all pass; a broader full
`lang_matrix` run surfaced no new regressions — the only other failures
(`matrix_every_proven_cell_agrees`, `proven_columns_do_not_silently_skip`,
and eight `algol_nested_procedure_*` tests) are the pre-existing,
separately-tracked ALGOL nested-procedure-captured-array bug on
NativeAOT/JVM (issue #12032), unrelated to BEAM or this slice.

**Dartmouth BASIC now declares 50/51 rows on `Beam`** — only `RND`
(VM-018, a separate unscoped module-global design question) remains
undeclared. **FLOW-MATIC now declares all 8/8 rows on `Beam`** — fully
complete. Combined with Twig (49/49), COBOL-60 (58/58), and Nib/Oct/
Brainfuck each fully proven per their own rows, **this closes every
non-ALGOL BEAM gap in this backlog except `RND`** — the one remaining item
is a genuinely separate, unscoped product/architecture question.

**Discovered but explicitly out of scope**: `global_store`'s existing
`erlang:put/2`-via-`gc_bif2` lowering carries the identical latent bug
fixed for the new host-input ops in this slice (§3.3 of the spec) —
confirmed reproducible with zero of this slice's own code (a
`str_const`-then-unrelated-`global_store`-then-read-back control
reproduced the same access violation), but fixing it safely requires
adding `global_store` to `iir-to-beam`'s `live_across` liveness filter and
re-verifying the entire existing BEAM corpus, which every BASIC/COBOL/Twig
program with a module-level variable goes through. Flagged for a
dedicated follow-up slice rather than bundled into this one — filed as
[#15332](https://github.com/adhithyan15/coding-adventures/issues/15332).
