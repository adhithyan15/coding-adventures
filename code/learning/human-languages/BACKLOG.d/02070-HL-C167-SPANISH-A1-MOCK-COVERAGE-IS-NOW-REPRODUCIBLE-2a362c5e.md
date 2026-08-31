## HL-C167 — Spanish A1 mock coverage is now reproducible

The chapter-406 audit exposed a process defect before the next vocabulary
bundle: the scripts behind `sitting-2026-08-26.md` were scratch artifacts, so
later loops could reconstruct the published result only approximately. The
book-bounded scoring policy is now executable and checked in.

`npm run report:spanish-a1-mock-audit` derives the A1 lesson set from the live
curriculum, extracts every answer-key `requires` cell, and scores whole items.
The committed `spanish/mocks/a1/book-bounded-audit.json` records the exact
credit policy, all failed items, the residual lexemes, and their frequencies.
`npm run check:spanish-a1-mock-audit` rejects a stale projection, while focused
tests pin both mock totals and the intentionally generous citation-form policy.

The reproducible post-chapter-406 result is **22/25 reading and 19/25
listening on mock 1**, plus **21/25 reading and 16/25 listening on mock 2**.
There are **22 failed objective items** and **30 distinct missing lexemes**.

The next bounded vocabulary tranche should use this report's whole-item
bundles. The single blockers remain valuable, but coherent multiword items
must be ranked by the number of complete items they close rather than by raw
lexeme frequency.
