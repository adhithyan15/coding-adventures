# A1 mock exams exist, and somebody finally sat them

The assessment contract has named `mocks/a1/mock-1-answer-key.md`,
`mocks/a1/mock-2-answer-key.md` and `mocks/a1/rubric.md` since it was written.
Not one of them existed. Two full mocks now do, and the answer to the question
they were built to ask is recorded.

Spanish's contract referenced **26 distinct artifacts that did not exist** — 21
mock files and 5 task-shape inventories (`a2` through `c2`). This change builds
three of the 21: the A1 rubric and the two A1 answer keys, leaving 23. That debt
is real and is being pinned repo-wide as a ceiling by separate work; it is named
here so this change is not mistaken for closing more of it than it does.

**No curriculum number moves in this change.** No lesson, ledger, spine shard or
pinned count is touched. Headwords stay **617**, verbs stay **40**, pre-A1 stays
**304**. This adds assessment artifacts and a report and nothing else.

## What was added

| file | what it is |
|---|---|
| `mocks/a1/rubric.md` | the real DELE A1 v2020 structure and pass rule, sourced |
| `mocks/a1/mock-1-paper.md`, `mock-2-paper.md` | two full four-paper mocks, all items original |
| `mocks/a1/mock-1-answer-key.md`, `mock-2-answer-key.md` | keys, PCIC point indexing, and a per-item `requires:` line |
| `mocks/a1/sitting-2026-08-26.md` | the scored result |

## The result

Both mocks return **`NO APTO`**, on the awarding body's real rule — DELE scores
**Grupo 1 = lectura + escritas** and **Grupo 2 = auditiva + orales**, each out
of 50, each needing 30 independently.

| | Grupo 1 | Grupo 2 | global |
|---|---|---|---|
| mock 1 | 3,00 / 50 | 11,58 / 50 | NO APTO |
| mock 2 | 0,00 / 50 | 4,00 / 50 | NO APTO |

86 of 100 objective items failed. Granting the entire A2 tier still fails both
groups in both mocks.

## What it found, and why it is not the gap we expected

The alphabet, capitals and punctuation were expected to be decisive. They are
real, but they land only on Prueba 3, where they cost the Writing paper exactly
one band — 8,33 points — and no quantity of vocabulary buys it back.

**The verb inventory is what sinks the exam.** 62 of the 86 failed items involve
a missing high-frequency verb. `gustar`, `hacer`, `tener`, `poder`, `querer` and
`decir` are all taught, but staged **A2** under `SPINE-SAY-WHAT-I-DO`; `porque`
is staged **B1**. The 40 verbs that *are* at or below A1 are dominated by
`borrar`, `colgar`, `empujar`, `girar`, `lanzar`, `quemar`, `saltar` and `volar`
— the tranche that closed criterion 2b in `a1-verbs-02.md`.

So HL09 §3.1's own warning has recurred inside the partition added to prevent
it: **criterion 2b counts verbs and never asks which verbs**, and *a total can
always be reached by the wrong parts*. The grammar ramp is not implicated — 136
grammar atoms at or below A1, full present, preterite, imperfect, progressive
and clitics. The machinery is built; there is nothing to run it on.

The smallest addition that passes is about **136 lexemes** — ~34 verbs and ~100
everyday nouns — and it passes Grupo 1 by 2,3 points. Adding the orthography
points turns that scrape into a margin.

## Confidence

The sitting is mechanical, not editorial. Each item declares the lexemes it
needs; a script marks it correct only if every one is in the set derived from the
corpus by the same lesson → path → spine → stage rule `levels.ts` uses. The
extractor reproduces the pinned 40-verb count exactly before being trusted.
Making the matcher 2,5× more lenient — plurals, multiword decomposition,
paradigm-taught verbs — changed the scores by **zero points**.
