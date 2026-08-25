## Unreleased — A1 vocabulary, tranche 2: thirty-five words (chapters 346-352)

### Added

- **Thirty-five A1 vocabulary lessons across seven chapters**, the second
  tranche on the run to the HL09 §3.1 A1 vocabulary floor of **600** distinct
  headwords taught at or below A1. Measured on `main` at the moment this branch
  was cut, the track taught **379**; it leaves this tranche at **414**. Stacked
  on tranche 1's thirty (chapters 340-345), the running total is **444**, and
  **156** headwords remain.

  The count rose by exactly thirty-five, which is the check that proves no
  headword in this tranche duplicates one already taught. `vocabularyOf`
  compares full strings, so a near-duplicate would have inflated the gate
  silently rather than failing.

  - **Chapter 346, *Five Places A Town Has*** — `la escuela`, `la iglesia`,
    `el mercado`, `la oficina`, `la estación`. Every one of the five is named
    after something other than its building: leisure, a summoned crowd, a heap
    of goods, a workshop, and a place to stand still.
  - **Chapter 347, *Which Way, And What Is Around You*** — `el norte`,
    `el sur`, `el oeste`, `el centro`, `alrededor`. Three of the four compass
    points came into Spanish from Germanic sailors by way of Old French rather
    than down from Latin, so `norte` and English *north* are one word.
  - **Chapter 348, *A Moment, And Its Two Edges*** — `el principio`, `el fin`,
    `el instante`, `la demora`, `el turno`.
  - **Chapter 349, *The Stretches A Calendar Marks*** — `la jornada`,
    `la década`, `la fiesta`, `el aniversario`, `la agenda`.
  - **Chapter 350, *Mine, Somebody Else's, And Which Kind*** — `propio`,
    `ajeno`, `igual`, `distinto`, `el tipo`.
  - **Chapter 351, *Counting Past Your Fingers*** — `cien`, `la cantidad`,
    `ambos`, `varios`, `el triple`.
  - **Chapter 352, *The Whole Set, And How Much Of It*** — `el conjunto`,
    `entero`, `la mayoría`, `el resto`, `escaso`.

- **Seven track-local A1 extensions**, one per chapter, hung off the four A1
  spine nodes the tranche realizes: `ES-EXT-346-PLACE` and `ES-EXT-347-PLACE`
  on `SPINE-ASK-LOCATION`, `ES-EXT-348-SPAN` and `ES-EXT-349-SPAN` on
  `SPINE-TIME-OF-DAY`, `ES-EXT-350-REF` on `SPINE-DEFINITE-REFERENCE`, and
  `ES-EXT-351-COUNT` and `ES-EXT-352-COUNT` on `SPINE-COUNT-ONE-TO-FIVE`.
  The four nodes declare about a dozen concepts between them, far fewer than
  thirty-five words need, so each chapter carries its own extension rather than
  one extension spanning several segments.

- **Two words carried forward from tranche 1.** `ambos` and `varios` were
  verified free there and left unused when that tranche's pools were sized in
  multiples of five. They are spent here rather than re-derived.

### Notes

- **The tranche is fully drivable.** All seven chapters narrate as "All 5 can
  be done entirely by ear" — no tables, no glyph work, and no sight cue in the
  prose.

- **Etymology, and where there is none to claim.** `el norte`, `el sur` and
  `el oeste` are Germanic borrowings rather than Latin inheritances, and the
  lessons say so plainly instead of manufacturing a Latin ancestor. `el oeste`
  also carries the warning that the fourth point, `el este`, fell into the same
  shape as the pointing word `este` by pure accident: the compass word is
  Germanic, the pointing word is Latin *iste*.

- **Six candidates were dropped before drafting**, each one a full-string miss
  that the count check would not have caught:
  - `el este` — collides with the taught `este` once a leading article is
    stripped.
  - `el futuro` — the taught pattern headword `si + futuro` separates on `+`,
    and the right-hand atom is `futuro`.
  - `el plazo` — same Latin *placēre* as the taught `el placer`.
  - `la temporada` — shares the `tempor-` morpheme with the taught `temprano`.
  - `el mes` and `el año` — the taught atoms `los meses` and
    `¿Cuántos años tienes?` already carry them; no delimiter separates either.
  - `el porcentaje` — one orthographic word built from `por` plus *ciento*,
    the same shape that ruled out `cualquier` in tranche 1.

- **`ruleStatements` stayed at its ceiling of 30.** Two draft sentences tripped
  the info-dump gate — a `varios` sentence that began "always comes before its
  noun" and a `resto` sentence that read "is used with *de*". Both were
  rewritten to show the instance rather than assert the rule, rather than
  raising a ceiling that records debt.

- **Spanish remains at pre-A1.** Thirty-five more headwords does not move the
  attainment claim, and the report still names Spanish as the only track with
  a level attained.
