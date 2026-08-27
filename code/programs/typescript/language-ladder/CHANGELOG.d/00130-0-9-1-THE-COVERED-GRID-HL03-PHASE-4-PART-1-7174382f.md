## 0.9.1 — the covered grid (HL03 phase 4, part 1)

- `src/quiz.ts` — `coveredGrid(covered, lessons, activeCount)` enumerates every
  (concept × language) cell the learner has studied, each tied to the real
  lesson that answers it. This is the pool the randomised cumulative quiz will
  draw from ("what is 5 in Telugu? 12 in Latin?").
- Built by **reusing the teaching sweep**, not re-deriving the concept→language
  join: a cell exists exactly where a covered concept's sweep has a stop in an
  active language — so the review side can only ever ask about a (concept,
  language) the teaching side actually presents. Deterministic (concepts sorted,
  then chain order, then chapter/id). Plus `conceptsIn` / `languagesIn` and a
  stable `cellKey` for the SRS to track state per item.
- Verified against the real curriculum: COURTESY-THANKS covers all ten chain
  languages; two covered concepts interleave across both concepts and many
  languages. Controls bite — mislabelling a cell fails the grounding test, and
  collapsing the grid to one concept fails the interleave control. Pure, no UI.
- Next (part 2): the SRS-weighted `pickNext` draw over this grid.

