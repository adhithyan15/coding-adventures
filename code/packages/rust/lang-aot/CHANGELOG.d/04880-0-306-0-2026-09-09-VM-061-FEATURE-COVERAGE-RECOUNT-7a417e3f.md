## 0.306.0 — 2026-09-09 — VM-061 feature-coverage recount

`LANG-VM-FEATURE-COVERAGE.md`'s per-frontend row/cell table was measured
stale in the prior slice (VM-D030): a fresh
`non_algol_matrix_every_proven_cell_agrees` run reported 1548 declared
non-ALGOL cells against the doc's stale 1478. A brace-balanced parse of every
`Prog` in `lang_matrix.rs`'s `PROGRAMS` corpus (not a hand count or a regex)
found the real gap was narrower than the prior slice's "~66 cells" estimate:
six of seven non-ALGOL frontends (Nib, Brainfuck, Dartmouth BASIC, Oct,
FLOW-MATIC, COBOL-60) already matched source exactly. Only Twig's doc number
(343) was wrong — `49 rows × 7 standard backends`, silently excluding its 20
Beam-declaring rows' extra cell each, while Nib/Oct/FLOW-MATIC/COBOL-60's
doc numbers already counted every declared backend including Beam. Corrected
Twig's row to 363, which makes the table's non-ALGOL rows sum to exactly
1548.

Added `feature_coverage_doc_counts_match_programs_source` to
`tests/lang_matrix.rs`: it asserts the exact (rows, total declared cells)
pair for each of the seven non-ALGOL frontends with rows in `PROGRAMS` today,
plus a zero-rows check for `McCarthyLisp`/`Macsyma` (both use dedicated
capstone files instead). Confirmed the test actually catches drift before
committing the fix: it failed with the expected assertion message against
the pre-fix Twig figure (343), then passed once the doc was corrected (363).
A future slice that adds or removes a `Prog` for any of these languages must
now update both this test and `LANG-VM-FEATURE-COVERAGE.md` together, or a
normal `cargo test -p lang-aot --test lang_matrix` run fails.

ALGOL 60's own doc row was observed to also be stale against the live corpus
(233/1631 vs. 245 rows today), but is left untouched: ALGOL is owned by a
separate, actively developing campaign, and this fix's scope (VM-061) never
extended a mandate to correct its count. No production `lang-aot`, frontend
or backend crate changed; this is a doc-and-test-only slice.

