## 0.328.0 — 2026-09-13 — VM-040 COBOL BEAM UNSTRING/delimiter probe

Reprioritized the post-STRING-SIZE/delimiter queue (remaining COBOL BEAM
rows, VM-041, VM-060b, VM-058) by re-running the backlog's prioritization
policy against current source rather than repeating the prior conclusion by
habit: `git fetch origin && git merge origin/main` reported "Already up to
date" at `32cf9a5efa`, zero commits behind `origin/main`. `gh pr list --state
open` showed no open PR touching any `lang-*`/`iir-*`/`cobol-*`/BEAM path or
this backlog, so nothing was in flight to coordinate with. A fresh
brace-balanced parse of `PROGRAMS` (scoped to the static array via `];` at
column 0) confirmed 58 `Cobol60` rows, exactly 32 already declaring `Beam`
and 26 not — the backlog's "~26 undeclared" estimate was exactly right.
`VM-041`/`VM-060b` still have no scoped design entry beyond the ranked-
backlog one-liner. COBOL BEAM rows win again on the same rung-4/bounded-
pattern reasoning as every prior round (nine times now).

Continued the established `.skip(N).take(4)`-over-`Cobol60`-filter pattern
(rows 32-35, immediately after the thirty-two already covered) on real
Erlang: one more `STRING ... DELIMITED BY delim` row (an item delimiter
stops a sender at its first match even when another follows) and three
`UNSTRING` rows (truncating/padding fields with no remainder carried into
the last receiver; leading/consecutive delimiters producing space-filled
empty receivers; an item delimiter with source exhaustion leaving later
receivers untouched). All four rows compile through
`cobol-iir-compiler::emit_prefix_before_delim`/`emit_unstring`, both of which
read fields via `str_len`/`str_index`/`str_slice`/`str_concat`/`cmp_*` —
every op already lowered for BEAM before this slice (the previous slice
added `str_len`/`str_index`; everything else was already proven by earlier
COBOL BEAM rows). Verified this by reading both compiler routines end to end
before probing, rather than discovering it by trial and error. All four
programs passed real `erl` execution on the first probe — no `iir-to-beam`
defect found, no production code changed.

Promoted all four rows to declare `Beam` (32 → 36 of 58 COBOL rows; 438 → 442
declared cells) and added `portable_text_stdout_cobol_beam_unstring_and_delimiter`,
mirroring the existing COBOL BEAM probe tests. `iir-to-beam`'s own package
suite is unaffected (still 107 tests, all passing) since no `iir-to-beam`
source changed. Focused Clippy on `lang-aot` and `iir-to-beam` with all
targets and warnings denied is clean. Updated
`feature_coverage_doc_counts_match_programs_source`'s expected COBOL-60
tuple (58, 438) → (58, 442) and confirmed it fails against the pre-fix figure
before fixing it, and updated `LANG-VM-FEATURE-COVERAGE.md`'s COBOL-60 row
and grand-total prose (1567 → 1571 declared cells) to match. The full
`non_algol_matrix_every_proven_cell_agrees` capstone confirmed the corrected
total against a live run: 210 programs, 1361 cells exercised (was 1357), 210
skipped, zero failures, 482.89s.

