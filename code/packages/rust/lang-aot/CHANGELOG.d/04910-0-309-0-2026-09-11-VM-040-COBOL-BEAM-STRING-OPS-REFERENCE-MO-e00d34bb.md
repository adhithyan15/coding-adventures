## 0.309.0 — 2026-09-11 — VM-040 COBOL BEAM string ops/reference modification probe

Reprioritized the post-boolean/EVALUATE queue (remaining COBOL BEAM rows,
VM-041, VM-060b, VM-058) by re-running the backlog's prioritization policy
against current source: `git fetch origin && git merge origin/main` found
this worktree already at `d68f9bc3cb` (PR #14779) with zero commits ahead on
`origin/main`, and `git log d68f9bc3cb..origin/main` over
`iir-to-beam`/`ir-to-beam`/`cobol-iir-compiler`/`cobol-runtime`/
`lang_matrix.rs` returned nothing — no commit touched any of these paths
since the last slice merged. `gh pr list --state open` showed no LANG-VM PR
in flight (one Qt PR, one Compose draft, four dependabot bumps). A fresh
brace-balanced parse of `PROGRAMS` (scoped to the static array, lines
171–6530, avoiding the two known non-row `Prog {}` false positives)
reconfirmed 455 total entries / 58 `Cobol60` rows, with exactly 20 already
declaring `Beam` and 38 not — the backlog's "~38 undeclared" estimate was
exactly right. VM-058 and VM-041/VM-060b's rung-5/unscoped status are
unchanged since nothing touched their paths either. COBOL BEAM rows win
again on the same rung-4/bounded-pattern reasoning as every prior round.

Continued the established `.skip(N).take(4)`-over-`Cobol60`-filter pattern
(rows 20–23, immediately after the twenty already covered across five prior
probe batches) on real Erlang: an alphanumeric `EVALUATE` subject with a
`THRU`-range `WHEN` (the first COBOL BEAM row to use `str_cmp`'s lexical
ordering to fold a range with `and`, reusing the boolean/EVALUATE slice's
fold and the initial-output slice's `str_cmp`), and three COBOL reference-
modification rows (literal bounds/omitted length, live computed indices, and
computed slices driving both `IF` and `EVALUATE`) built entirely on
`str_slice`, already proven by the earlier COBOL BEAM alphanumeric
MOVE/comparison slice. All four passed on the first probe — no
`iir-to-beam`/`ir-to-beam` defect found, no production code changed.
Promoted all four rows to declare `Beam` (20 → 24 of 58 COBOL rows; 426 → 430
declared cells) and added
`portable_text_stdout_cobol_beam_string_ops_and_refmod`, mirroring the
existing COBOL BEAM probe tests. Updated
`feature_coverage_doc_counts_match_programs_source`'s expected COBOL-60
tuple (58, 426) → (58, 430) and confirmed it fails against the pre-fix
figure before fixing it, and updated `LANG-VM-FEATURE-COVERAGE.md`'s
COBOL-60 row and grand-total prose (1555 → 1559 declared cells) to match.
The full `non_algol_matrix_every_proven_cell_agrees` capstone confirmed the
corrected total against a live run: 210 programs, 1349 cells exercised (was
1345), 210 skipped, zero failures, 493.28s.

