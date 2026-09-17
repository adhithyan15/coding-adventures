## HL-C375 — a fifth of the exam gap is already taught and merely unwired

Chapter 419 closed two A1 points and the two cost wildly different amounts,
which is the finding.

**A1-NE18-02 (music and dance)** needed authoring: `ES-LEX-MUSICA` existed but
`ES-LEX-BAILAR` did not, so the point needed a lesson. **A1-NE18-05
(photography)** needed nothing at all. `ES-LEX-FOTO` and `ES-LEX-FOTOGRAFIA`
have been introduced by chapter 405 for as long as that chapter has existed.
The point was counted uncovered because its `probe` was `null` — nobody had
wired it. Spanish could already teach photography and the number said it could
not.

A corpus-wide scan puts **451 of the 2,189 uncovered points (21%)** in that
second class: at least one already-introduced atom whose name matches a word in
the point's label. The heuristic is crude and will overcount, but even half of
451 is more points than the last several tranches closed in total, at the cost
of reading a probe rather than authoring a chapter.

| track | gaps | gaps with candidate atoms already taught |
|---|---:|---:|
| russian A1 | 121 | 48 |
| french A2 | 88 | 44 |
| marathi A1 | 139 | 40 |
| japanese A1 | 112 | 31 |
| chinese A1 | 93 | 24 |
| tamil A1 | 87 | 22 |
| hindi A1 | 89 | 21 |
| malayalam A1 | 80 | 21 |
| telugu A1 | 112 | 21 |

This reframes the queue. `completion-plan.ts` ranks `exam-point` above
`vocabulary` because headwords cannot cover a point with no atom — still true.
But it does not distinguish a point that needs a *lesson* from a point that
needs a *line of JSON*, and those differ by roughly two orders of magnitude in
cost. **The next tranche should be a probe-wiring audit, not authoring**, and
the ordering after it should treat "unmapped" as two families rather than one.

The audit has to be done honestly: a candidate atom is a hypothesis, not a
match. `ES-LEX-FOTO` covers "photography"; something like `ES-LEX-ARTE` would
not cover "artistic disciplines" merely by sharing a stem. Each wiring has to be
read against the point's label before it is committed, or the gate stops
measuring whether a reader could pass and starts measuring whether a script
could pattern-match.
