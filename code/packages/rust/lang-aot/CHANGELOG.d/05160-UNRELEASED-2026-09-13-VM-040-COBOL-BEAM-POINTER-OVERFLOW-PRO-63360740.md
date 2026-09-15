## Unreleased — 2026-09-13 — VM-040 COBOL BEAM pointer/overflow probe

`git fetch origin && git merge origin/main` fast-forwarded cleanly (one
unrelated SwiftUI border-edges commit, `092349e3ff`); `gh pr list --state
open` showed no PR touching any `lang-*`/`iir-*`/`cobol-*`/BEAM path or this
backlog, so nothing was in flight to coordinate with.

Re-derived the prior slice's premise from source rather than trusting it: a
brace-balanced scan of `PROGRAMS` restricted to `Language::Cobol60` confirmed
58 rows, 40 declaring `Beam`, 18 not — and the 8 pointer/overflow rows
(`STRING`/`UNSTRING` `WITH POINTER` and `ON OVERFLOW`/`NOT ON OVERFLOW`) are
exactly indices 36-43, immediately preceding the base `INSPECT TALLYING`/
`REPLACING` family the prior slice promoted. `VM-041`/`VM-060b`/`VM-058`
still have no scoped design progress beyond their ranked-backlog one-liners.

Read `cobol-iir-compiler::emit_string`/`emit_unstring`/
`emit_string_pointer_overlay` end to end before writing any test, per this
loop's "probe before declaring" discipline. The shared overlay helper is
genuinely structurally novel: it chains THREE `str_slice` calls (the
receiver's untouched head, the content actually placed, and the receiver's
untouched tail) and TWO `str_concat` calls to stitch the result back
together, with the overlay's start position, remaining room, and the amount
actually copied all computed at run time from a live `PIC 9` pointer item —
the first COBOL BEAM row to chain multiple `str_slice`/`str_concat` calls
through one helper over fully run-time-computed bounds, rather than a single
call with a run-time bound (VM-D032's case). Traced the register flow by
hand: the receiver register is read twice (for the head and tail slices)
before it is ever written (by the final `str_concat`), so no VM-057-shaped
destination-aliases-source hazard exists here — the receiver is never both
source and destination of the SAME op, unlike VM-057's self-move row.

Added `Beam` to all 8 pointer/overflow rows and
`portable_text_stdout_cobol_beam_pointer_overflow`, running the full family
in one slice (not the usual four-at-a-time) since it is a natural
self-contained unit and was the backlog's own top candidate to expose a real
`iir-to-beam` defect. All eight programs — four `STRING ... WITH POINTER`
cases and four `UNSTRING ... WITH POINTER` cases, exercising both `ON
OVERFLOW` and `NOT ON OVERFLOW` — passed real `erl` execution on the first
probe. No `iir-to-beam` defect was found and no production code changed;
this is a clean-pass outcome, not a repair. `iir-to-beam`'s own package
suite is unaffected (still 107 tests: 22 unit, 80 integration, 5 doc, all
passing) since no `iir-to-beam` source changed. Focused Clippy on `lang-aot`
and `iir-to-beam` with all targets and warnings denied is clean.
Forty-eight of 58 COBOL rows now declare BEAM (454 total declared cells, up
from 446). `feature_coverage_doc_counts_match_programs_source` was confirmed
to actually exercise the check (it failed with the expected 454/446
mismatch against the pre-fix doc before the doc was corrected), then updated
(COBOL-60 tuple `(58, 446)` → `(58, 454)`) and `LANG-VM-FEATURE-COVERAGE.md`'s
COBOL-60 row and grand-total prose (1575 → 1583 declared cells) to match. The
full `non_algol_matrix_every_proven_cell_agrees` capstone confirmed the
corrected total against a live run: 210 programs, 1373 cells exercised (was
1365), 210 skipped, zero failures, in 490.62s.

Reprioritize the remaining 10 undeclared COBOL BEAM rows (4 more base
INSPECT TALLYING/REPLACING, 5 VM-047c BEFORE/AFTER region, 1 VM-057
self-move) against VM-041/VM-060b/VM-058 after this merges.

