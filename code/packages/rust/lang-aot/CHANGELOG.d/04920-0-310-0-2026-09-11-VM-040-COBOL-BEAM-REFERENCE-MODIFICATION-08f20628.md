## 0.310.0 — 2026-09-11 — VM-040 COBOL BEAM reference-modification MOVE/trap probe

Reprioritized the post-string-ops/refmod queue (remaining COBOL BEAM rows,
VM-041, VM-060b, VM-058) by re-running the backlog's prioritization policy
against current source: `git fetch origin && git merge origin/main` reported
"Already up to date" — this worktree already sat at `dd87a55d80` (PR #14784)
with zero commits behind `origin/main` (`git rev-list --count
dd87a55d80..origin/main` = 4, all four unrelated: toolkit/task-app/engram-
app/lessons-doc commits, none touching `lang-*`/`iir-*`/`cobol-*`/BEAM
paths, confirmed via a path-scoped `git log` returning nothing). `gh pr list
--state open` showed six open PRs (a Compose draft, three npm/yarn dependabot
bumps, one GitHub Actions dependabot bump, one stale paint-vm-canvas
dependabot bump) — none touching this backlog. A fresh brace-balanced parse
of `PROGRAMS` (scoped to the static array) reconfirmed 455 total entries / 58
`Cobol60` rows, with exactly 24 already declaring `Beam` and 34 not — the
backlog's "~34 undeclared" estimate was exactly right. `VM-041`/`VM-060b`
still have no scoped design entry beyond the ranked-backlog one-liner. COBOL
BEAM rows win again on the same rung-4/bounded-pattern reasoning as every
prior round.

Continued the established `.skip(N).take(4)`-over-`Cobol60`-filter pattern
(rows 24–27, immediately after the twenty-four already covered) on real
Erlang: a constant reference-modification MOVE that pads and truncates into
differently-sized receivers, a runtime MOVE that refits using a live computed
slice length, and two computed reference modifications that must fail closed
(a runtime `end` past the item's width; a runtime `start` of zero after the
1-based → 0-based conversion). The first two passed on the first probe. The
two trap rows exposed a real `iir-to-beam` defect (see that crate's
CHANGELOG 0.9.2): `str_slice` lowered straight to `lists:sublist/3`, which
does not itself implement the documented bounds-check contract every other
backend's `str_slice` already enforces — an in-range `start` with an
out-of-range `end` silently truncated (`sublist("ABCDE", 4, 5)` returned
`"DE"`, not an error) instead of trapping, confirmed by a scratch probe test
against real `erl` before any repair. Fixed by making the bounds check
explicit in `str_slice`'s lowering (`erlang:length/1` + `is_ge` branches,
raising `erlang:error(badarg)` on violation) rather than relying on
`sublist`'s own, looser guard clauses. This also required adding
`Expect::Trap` handling to `run_beam` in `lang_matrix.rs` itself — no prior
COBOL BEAM row (or any other language's BEAM row) had ever exercised a
trapping construct, so the runner had no way to represent one; every other
backend's runner already had this `Err(_)/nonzero-exit if Expect::Trap =>
Trapped` arm, BEAM was the one missing it. `run_beam` also now anchors `erl`'s
working directory in the same disposable temp dir as the `.beam` file, so a
legitimately-trapping cell's `erl_crash.dump` does not land in the repo
checkout.

Promoted all four rows to declare `Beam` (24 → 28 of 58 COBOL rows; 430 → 434
declared cells) and added `portable_text_stdout_cobol_beam_refmod_move_and_trap`,
mirroring the existing COBOL BEAM probe tests. Two new direct `iir-to-beam`
regressions pin the bounds-check contract itself:
`test_73_real_erl_str_slice_traps_end_past_length` (the exact silent-
truncation case) and `test_74_real_erl_str_slice_end_equal_to_length_is_in_bounds`
(the boundary control, proving the check is `end > len` not `end >= len`).
Updated `feature_coverage_doc_counts_match_programs_source`'s expected
COBOL-60 tuple (58, 430) → (58, 434) and confirmed it fails against the
pre-fix figure before fixing it, and updated `LANG-VM-FEATURE-COVERAGE.md`'s
COBOL-60 row and grand-total prose (1559 → 1563 declared cells) to match.
The full `non_algol_matrix_every_proven_cell_agrees` capstone confirmed the
corrected total against a live run: 210 programs, 1353 cells exercised (was
1349), 210 skipped, zero failures, 511.43s.

