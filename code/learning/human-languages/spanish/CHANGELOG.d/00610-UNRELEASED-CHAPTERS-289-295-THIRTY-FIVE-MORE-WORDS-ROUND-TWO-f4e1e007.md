## Unreleased — Chapters 289-295: thirty-five more words, round two

The first tranche took Spanish to 83 of the 300 headwords the pre-A1 vocabulary
floor asks for. That was still the lowest of the seven tracks in the ramp — behind
Tamil and Malayalam at 84, Hindi at 85, Sanskrit at 104, and Telugu and Kannada at
116 and 117. These seven chapters answer with thirty-five more, on the same HL14
rule: **one new headword per lesson**, reuse unlimited and deliberate.

```
289 MEET-GREET             el amanecer, el cielo, la nube, la estrella, la sombra
290 COURTESY-THANK         el trigo, el arroz, la cebolla, el tomate, la miel
291 RESPOND-BASIC          la semilla, la raíz, el tronco, la rama, el bosque
292 EXCHANGE-NAMES         el pueblo, la colina, el valle, el lago, la montaña
293 CHECK-WELLBEING        el cuello, el hombro, la espalda, la rodilla, el tobillo
294 POLITE-REQUEST-REPAIR  el cuchillo, la taza, la olla, la manta, la escoba
295 TAKE-LEAVE             el sendero, el puente, la orilla, la isla, la niebla
```

The vocabulary shortfall falls 217 to 182 — a drop of exactly thirty-five, the
lesson count — and the pre-A1 reinforcement blocker again does not move, holding
at 24. All seven pre-A1 nodes are reused a second time; node reuse remains the
mechanism, since Spanish realised all seven long ago.

**The ramp got gentler again.** R1 falls 0.2843 to 0.2818. Every lesson chains to
the one before, each chapter's second lesson practises the previous chapter's
final atom, and each fifth lesson is a payoff practising all five — so no new atom
misses its R1 window. `forwardReferences` holds at its 500 ceiling and
`ruleStatements` at 30: both had zero headroom, and the tranche took none.

**Screening cost more than writing.** 146 candidates were screened against all 514
existing lessons in both directions and inside headwords; 111 survived and 35 were
chosen. Every rejection but one came from the headword-and-prose check — `hoja`,
`río`, `sal`, `vela` and `manzana` were already taught, and `nido` was caught whole
inside `bienvenido`, `remo` inside `queremos`, and `ario` inside `armario`.

**Threads that pay across chapters.** The diminutive `-illo`/`-illa` is introduced
in an onion and then found again in a seed, a knee, an ankle, a knife and an edge —
six unrelated words, one piece of machinery. The `e-` step before `s` plus a
consonant pays three times (`estrella`, `espalda`, `escoba`), and the short-`o`-to-
`ue` break three times (`pueblo`, `cuello`, `puente`). The run opens on `la nube`,
setting Latin *nebula* aside unclaimed, and closes on `la niebla` collecting it.

**Parts of speech are still not named.** The first tranche had to reword thirteen
lessons to drop "verb" and "noun"; this one was written without them from the
start, so the metalanguage pins do not move.

**The book crossed a thousand pages, and the contents page was not ready for it.**
These seven chapters pushed Spanish from 1244 to 1272 pages, and fourteen contents
entries acquired a four-digit page number for the first time. `book.cls` packs
every page number into a box of exactly `\@pnumwidth`, which it sets to 1.55em;
Latin Modern's digits are 0.5em each, so four of them need 2.00em and overflow by
0.45em — 4.92747pt at the contents' 10.95pt, which is the width TeX reported,
identically, fourteen times. `\@pnumwidth` is now 2.8em, enough for five digits,
matching the number the chapter-number column already uses since HL-C109.

Widening that box narrowed the room left for chapter titles, and chapter 285's
title — the longest in the book — wrapped for the first time. `book.cls` sets
contents entries justified, so its first line stretched to badness 1038, just past
the default `\hbadness` of 1000, and traded the overfull boxes for an underfull
one. `\l@chapter`'s `\rightskip` and `\parfillskip` now carry `plus 1fil` and
`plus -1fil`: wrapped titles set ragged right and can never be reported underfull,
and the two infinities cancel on the final line so the page number does not move.
Neither warning baseline was touched; both counts are zero because the boxes fit.

Spanish is the only track anywhere near this. Every other book is between 133 and
355 pages, so the other twenty-one preambles keep `book.cls`'s defaults until they
need otherwise.

