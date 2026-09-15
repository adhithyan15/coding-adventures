## 0.305.0 — 2026-09-09 — VM-040 COBOL BEAM condition-name/EVALUATE probe

Declared `Beam` for the next four COBOL-60 matrix rows: a level-88
condition-name `IF`, a level-88 multi-value/THRU-range condition-name (the
first COBOL BEAM row folding `cmp_eq` with `and`/`or`), `SET
condition-name TO TRUE`, and a symbolic `>=` relational. New test
`portable_text_stdout_cobol_beam_condition_names_and_evaluate` (the fourth
`.skip(N).take(4)`-over-`Cobol60` probe, at `.skip(12)`) runs all four on
real `erl`; COBOL now declares 16 of its 58 rows on `Beam` (four
literal/output, four control/rounding, four signed/algebra, four
condition-name/EVALUATE), 422 total declared cells.

All four rows passed on the first probe with no new defect: each reuses
`cmp_*`/`const`/branch lowering already proven by earlier COBOL BEAM rows
(IF/ELSE, comparisons), so no `iir-to-beam`/`ir-to-beam` production change
was needed this slice. All 12 Oct, 26 Nib, and the previous 12 COBOL BEAM
rows pass again alongside the four new ones. The full non-ALGOL matrix
(`non_algol_matrix_every_proven_cell_agrees`) passed 210 programs, 1338
cells exercised, 210 skipped (host-wide missing `ilasm`), zero failures.

Discovered VM-D030 while updating the coverage inventory: the actual
declared non-ALGOL cell total (1548, per that same fresh matrix run) is
significantly higher than `LANG-VM-FEATURE-COVERAGE.md`'s previously stated
1478, for frontends other than this slice's own independently re-verified
COBOL-60 row. Corrected the document's grand total to the measured figure
and queued VM-061 to reconcile every row against source.

