### Spanish A1 qualities rung — a new spine node, twelve lessons, and twenty-two adjectives brought home (chapters 397-399)

The second of three authoring tranches on the run to `APTO`, and the first in the series to mint
a spine node rather than fill one. `HL23` §12.2 decided this rung; `HL23` §13 records what it cost.

```
  397  QUALITIES  barato, caro, gratis, mayor, menor
  398  QUALITIES  importante, interesante, favorito, fácil, difícil
  399  QUALITIES  bonito, guapo
```

#### The bundle proof came first, and predicted the outcome to the cent

Baseline reproduces the published post-tranche-7 state exactly — mock 1 Grupo 1 **26,33**, mock 2
**20,33**, 52 objective items failed — which is what licenses the projection. Granting the seven
exam-derived qualities alone projected **27,33** and **24,33**. The re-sit after authoring returns
**27,33** and **24,33**. That is the fourth consecutive slice where the proof called the result
before a word of prose was written.

The proof also re-confirms both `HL23` §12 decisions with the qualities authored rather than
merely granted: all 68 remaining lexemes reach **31,33 / 32,33**, `APTO` on both groups; dropping
the qualities returns mock 2's Grupo 1 to **25,33**, and dropping the nine confusability-flagged
candidates returns it to **28,33**, against a 30,00 bar. Neither decision is optional.

#### The node

`SPINE-DESCRIBE-QUALITIES`, A1, LEXICON — *"I can say what something is like."* Prerequisite
`SPINE-NAME-EVERYDAY-THINGS`, because you must be able to name a thing before you can describe it.

**Its `concepts` list is empty, and it is the first node in the corpus for which that is true.**
That is deliberate. A node's `concepts` list is where it makes a claim on all 23 tracks, and this
rung is justified by the Spanish A1 syllabus — inventory points `A1-NG6-03`, `A1-NG6-08` and
`A1-NG6-10` — so it asks nothing of any other track. The lessons carry namespaced `ES-QUALITY-*`
tags, exactly as tranche 7's did.

Minting a mid-ladder node costs **575 shard renames**, because shard filenames are positional and
every node after the insertion point renumbers. Appending after the C2 nodes would have avoided
them and passed `check:shards`, at the price of a permanent lie about the curriculum's shape.

#### Twenty-two adjectives were already filed under "the cardinal numbers one through five"

`HL23` §12.2 refused, in writing, the option of filing adjectives under `SPINE-COUNT-ONE-TO-FIVE`,
and recorded the refusal so the next tranche would not rediscover it. **It had already happened.**
Chapters 365, 371, 379 and 380 are adjectives and nothing else — `alto`, `gordo`, `alegre`, `feo`,
`necesario`, `dulce` and fifteen more — sitting under a `canDo` about counting to five.

All four chapters are relocated onto the new rung. Each is a one-line segment retarget with no
lesson moves and no splits, and the move is score-neutral because both nodes are A1. It makes
**both** nodes' `canDo` true: the numbers node stops claiming `alegre` and `necesario`.

Chapters 344, 351 and 352 are mixed and are left for a follow-up, because a mixed chapter needs a
segment split and therefore an extension split. `ES-PATH-359-01` — the units of measure — was
examined and left on purpose.

#### Four inventory points close, one of them for free

`A1-NG6-03` (attractiveness), `A1-NG6-08` (interest) and `A1-NG6-10` (ease and difficulty) are
closed by authoring the source's own exponents rather than by pointing them at words that happened
to be nearby. `A1-NG6-09` (capacity with *saber*) needed **no authoring at all**: its note claimed
the corpus never introduces `saber`, which stopped being true when chapter 389 authored it two
slices ago. Unmapped points **48 → 44**; A1 coverage **225 → 229 of 273**, 82% → **84%**.

#### `feo` was already taught, and arithmetic caught it, not the screen

Thirteen lessons were written; twelve shipped. `feo` was already in the corpus as `ES-C380-feo`
under the tag `ES-COUNT-UGLY` — which is how the mis-filing above came to light. The duplicate was
caught because the headword count came out **685** where twelve new lessons predicted 686.

The screen missed it for two separable reasons, both now recorded in `lessons.md`: it was run over
the 76 exam-derived candidates and never pointed at the six exponents this slice added itself; and
re-running it after authoring cannot help, because the screen then reports the tranche to itself —
every new word comes back "already owned", owned by the tranche's own files. A corpus-wide
duplicate-headword invariant is now in the pre-commit sweep.

One new confusability flag appeared that no earlier screen could have seen: **`llevar`/`llegar`**,
manufactured by tranche 7 authoring `llegar`.

#### Etymology was verified in a dedicated pass, before any prose

The pass killed claims that would otherwise have shipped. `favorito`'s `-ito` is an **Italian past
participle**, not the Spanish diminutive it is identical to — a learner fresh off `casita` will
read it wrong, so the lesson teaches the contrast against `bonito`, where the same ending *is* the
diminutive. `feo` is not related to English *foul* (Germanic) nor to *federal* (a second, unrelated
Latin *foedus*). `guapo` is left explicitly unsettled: the DLE prints *vappa* as a flat single line
and Corominas argues against it, so the lesson teaches the disagreement. `mayor` is not the source
of the month *mayo*. `barato`'s Greek *prattein* story is folk etymology and is named as such.
`difficult` entered English as a back-formation from *difficulty*.

The four confusability-flagged words in this tranche teach their own minimal pairs, per §12.3:
`caro` against `cara` and `cero`, `menor` against `menos`.

#### Counts

`≤` pre-A1 stays **304** (floor 300, untouched — every lesson here is A1); `≤` A1 **673 → 685**
headwords; verbs `≤` A1 **68**, unchanged, because no verb was authored. Thirty-two lessons now sit
on the qualities rung: twenty relocated, twelve new.

#### Re-sat

```
  Grupo 1   mock 1  26,33 -> 27,33      mock 2  20,33 -> 24,33
  Grupo 2   mock 1  22,25 -> 24,25      mock 2  15,33 -> 15,33
  objective items failed  52 -> 45 of 100
```

`NO APTO` on both, which is the predicted shape. Mock 2's Grupo 1 moves **+4,00**, the largest
single-slice movement on that paper in the series — which is what §12.2 meant by "no quantity of
noun authoring passes mock 2". Tranches B and C close the rest.
