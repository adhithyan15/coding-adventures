## 0.331.0 — 2026-09-13 — VM-040 COBOL BEAM INSPECT TALLYING/REPLACING probe

Reprioritized the post-UNSTRING/delimiter queue by re-running the backlog's
prioritization policy against current source rather than repeating the prior
conclusion by habit: `git fetch origin && git merge origin/main` reported
"Already up to date". `gh pr list --state open` showed no open PR touching
any `lang-*`/`iir-*`/`cobol-*`/BEAM path or this backlog, so nothing was in
flight to coordinate with.

PR #14999's trailing note re-scoped the remaining 22 undeclared COBOL BEAM
rows into four distinct families rather than one homogeneous tail: 8
pointer/overflow rows (`STRING`/`UNSTRING` `WITH POINTER` and `ON
OVERFLOW`/`NOT ON OVERFLOW`), 8 base `INSPECT TALLYING`/`REPLACING` rows, 5
VM-047c `BEFORE`/`AFTER` region rows, and 1 VM-057 STRING self-move edge
case. Investigated each rather than defaulting to file order:

- Pointer/overflow (`cobol-iir-compiler::emit_string`/`emit_string_pointer_overlay`/
  `emit_unstring`) compiles through `const`/`cmp_lt`/`cmp_gt`/`cmp_le`/`cmp_ge`/
  `sub`/`add`/`mov`/`str_slice`/`str_concat`/`jmp*`/`label` — every op already
  lowered for BEAM, `str_slice`'s bounds check already fixed by VM-D032. No
  gap found, but it is the most structurally novel combination (multiple
  chained `str_slice`/`str_concat` calls through a helper with fully
  run-time-computed bounds) of the four families.
- Base `INSPECT TALLYING`/`REPLACING` (`emit_inspect_tallying`/
  `emit_inspect_replacing`) compiles through `str_len`/`str_index`/`cmp_*`/
  `const`/`add`/`sub`/`mov`/`jmp*`/`label`/`and`/`or`/`str_const` — the exact
  same substrate `str_len`/`str_index` (added two slices ago) and `and`/`or`
  (proven in the boolean/EVALUATE probe) already exercise elsewhere. This is
  the lowest-risk family: no op combination here is novel to BEAM.
- VM-047c `BEFORE`/`AFTER` region rows reuse `emit_inspect_region_window`,
  which is a strict superset of the base family's op vocabulary (same ops,
  one extra delimiter-scan loop) — already fully implemented and tested on
  all seven standard backends by VM-047c itself, just missing the BEAM
  column.
- VM-057's single self-move row was specifically checked for the WASM-shaped
  aliasing hazard that caused the original VM-057/VM-D032 discoveries
  (`STRING S DELIMITED BY SIZE INTO S` lowers to a `str_slice` whose
  destination and source are the SAME BEAM x-register, since `iir-to-beam`
  also maps one x-register per variable name via `reg_map`). Reading
  `iir-to-beam::lower.rs`'s `str_slice` arm shows the source register is
  staged into a scratch register (`s_src`) via `OP_MOVE` BEFORE the
  destination register is ever overwritten with the `sublist` result — the
  aliasing hazard that hit WASM's bump-allocate-then-copy shape does not
  exist here, since BEAM's `str_slice` already reads every operand before
  writing `rd`. This is a negative result, logged so the next slice does not
  re-derive it.

All four families are missing-backend-parity work (rung 4) on
already-implemented, already-tested COBOL features; none is a red cell.
`VM-041`/`VM-060b` still have no scoped design entry beyond the ranked-backlog
one-liner (rung 5, unchanged). Selected the base INSPECT TALLYING/REPLACING
family as the demonstrably lowest-risk of the four — every op it needs has
multiple prior BEAM proofs, with no novel combination — continuing the
established `.skip(N).take(4)`-over-`Cobol60`-filter pattern on its first four
rows (`TALLYING FOR ALL`, `TALLYING FOR CHARACTERS`, `TALLYING FOR LEADING`,
`REPLACING ALL`; `.skip(44).take(4)`, since the 8 undeclared pointer/overflow
rows precede this family in file order and were deliberately not selected
first this round).

All four programs passed real `erl` execution on the first probe — no
`iir-to-beam` defect found, no production code changed, matching the
op-level audit's prediction. Promoted all four rows to declare `Beam` (36 →
40 of 58 COBOL rows; 442 → 446 declared cells) and added
`portable_text_stdout_cobol_beam_inspect_tallying_replacing`, mirroring the
existing COBOL BEAM probe tests. `iir-to-beam`'s own package suite is
unaffected (still 107 tests: 22 unit, 80 integration, 5 doc, all passing)
since no `iir-to-beam` source changed. Focused Clippy on `lang-aot` and
`iir-to-beam` with all targets and warnings denied is clean. Updated
`feature_coverage_doc_counts_match_programs_source`'s expected COBOL-60 tuple
(58, 442) → (58, 446) and `LANG-VM-FEATURE-COVERAGE.md`'s COBOL-60 row and
grand-total prose (1571 → 1575 declared cells) to match. The full
`non_algol_matrix_every_proven_cell_agrees` capstone confirmed the corrected
total against a live run: 210 programs, 1365 cells exercised (was 1361), 210
skipped, zero failures, 496.04s.

Reprioritize the remaining 18 undeclared COBOL BEAM rows (8 pointer/overflow,
4 more base INSPECT TALLYING/REPLACING, 5 VM-047c BEFORE/AFTER region, 1
VM-057 self-move) against VM-041/VM-060b/VM-058 after this merges.

