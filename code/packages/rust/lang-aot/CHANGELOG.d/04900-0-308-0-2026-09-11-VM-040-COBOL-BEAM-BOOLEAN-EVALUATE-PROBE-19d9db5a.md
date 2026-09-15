## 0.308.0 — 2026-09-11 — VM-040 COBOL BEAM boolean/EVALUATE probe

Reprioritized the post-VM-042 queue (COBOL BEAM rows, VM-041, VM-060b,
VM-058) by re-running the backlog's prioritization policy against current
source rather than repeating the prior conclusion by habit: `git log` since
VM-042 merged showed no commit touching `iir-to-beam`, `cobol-iir-compiler`,
`cobol-runtime` or `lang_matrix.rs`'s COBOL rows, so nothing changed the
picture VM-042's own trailing note described. A brace-balanced parse of
`PROGRAMS` in `lang_matrix.rs`, restricted to the actual static array (not
every `Prog {}` literal in the file — a naive whole-file parse over-counts
by two: a per-iteration differential-test helper reusing the `Prog` struct
with `backends: &[]`, and a boundary-assertion `Prog` local to a single
`#[test]` function), confirmed the "~42 undeclared COBOL BEAM rows" figure
precisely: exactly 42 of 58 `Cobol60` rows did not declare `Beam`, matching
every non-ALGOL row/cell count already pinned by VM-061's own test. VM-058
stays rung 5 (nothing found here changes that re-examination); VM-041 and
VM-060b remain unscoped design work. COBOL BEAM rows remain the bounded,
already-proven-pattern rung-4 pick.

Continued the established `.skip(N).take(4)`-over-`Cobol60`-filter pattern
(rows 16–19, immediately after the twelve already covered across four prior
probe batches) on real Erlang: a compound `(N > 1 OR N > 9) AND N < 8`
condition, a `NOT (N < 3 OR N > 9)` negated group (the first COBOL BEAM row
to emit `xor`), an `EVALUATE` case statement, and an `EVALUATE` with a
multi-value/THRU-range `WHEN`. All four passed on the first probe, reusing
`cmp_*`/`and`/`or`/`xor`/branch lowering already proven by earlier COBOL BEAM
rows — no `iir-to-beam`/`ir-to-beam` defect found, no production code
changed. Promoted all four rows to declare `Beam` (16 → 20 of 58 COBOL rows;
422 → 426 declared cells) and added
`portable_text_stdout_cobol_beam_boolean_and_evaluate`, mirroring the
existing COBOL BEAM probe tests. Updated
`feature_coverage_doc_counts_match_programs_source`'s expected COBOL-60
tuple (58, 422) → (58, 426) and confirmed it fails against the pre-fix
figure before fixing it, and updated `LANG-VM-FEATURE-COVERAGE.md`'s
COBOL-60 row and grand-total prose (1551 → 1555 declared cells) to match.

