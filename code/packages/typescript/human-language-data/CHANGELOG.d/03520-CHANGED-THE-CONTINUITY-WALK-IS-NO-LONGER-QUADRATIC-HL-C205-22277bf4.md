### Changed - the continuity walk is no longer quadratic (HL-C205)

- Index forward-reference candidates by their leading word-run, so each lesson is asked only about words its own text can reach instead of about every word its track teaches.
- Replace the per-word `(?<![\p{L}\p{M}-])...` regex — ~330µs to build and first run, ~2,700 of them per report — with one shared character class and an `indexOf` boundary walk.
- `measureContinuity` 2,065ms to 218ms on the 2,771-lesson corpus; `report --format json` 3.94s to 1.79s; the full test suite's CPU time 93.8s to 79.7s.
- Report output is byte-identical in both formats, verified against a differential check of 379,834 real-corpus pairs and 60,000 adversarial Unicode cases.
- Skip the rest of a word-adjacent run when an occurrence is rejected, so a body of near-misses cannot drive the matcher quadratic — 3.9s to 2.3ms on the case a security review constructed, which is faster than the regex it replaced.
- Pin the boundary rule with eight tests covering glued words, hyphens, combining marks, astral-plane neighbours, multi-word headwords and non-Latin script, plus a ninth that fails at 8.2s without the run skip and passes at 0.3s with it.

