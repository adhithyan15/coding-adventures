### Added - the irregular and stem-changing overlays (HL-C91)

HL10 section 5.1 sized the Spanish verb system at roughly 630 cells. HL-C82
shipped the 231 regular ones; this adds the **402 overlays**, for **633 total**.
The original estimate holds.

    stem-change  e-ie 36 · o-ue 32 · e-i 20 · u-ue 4
    strong preterite 90 · short stem 144 · irregular subjunctive 36
    irregular imperfect 18 · irregular participle 12 · go-club 10

**They are a separate list, and that is pedagogy rather than tidiness.** A
learner never meets "the irregular verbs" as a category. They meet the regular
row, and then the one verb that breaks it, in frequency order, one cell at a
time. So every overlay's prerequisite is the regular cell it deviates from:
`tengo` hangs off `ES-CELL-IND-PRES-1SG-CONJ2`. The DAG gains depth, nothing
becomes reachable earlier, and an irregular is always taught against a pattern
the learner already holds.

Three shapes are pinned by test because losing them would flatten the model back
into "this verb is irregular":

- **The boot.** A stem change covers the singular and the third plural only; the
  two plural persons keep the regular stem. Four cells per verb, not six.
- **One weld, twice.** A shortened future stem serves the conditional as well, so
  a verb like *tener* owns twelve cells but one thing to learn.
- **Three imperfects.** The entire language has three irregular imperfects, which
  is why HL10 places that tense immediately after the preterite as a rest.

The regular inventory is byte-identical to before -- verified against `HEAD`, not
asserted -- so every HL-C82 pin still measures what it measured.

`BUILD` already gates generator drift with `--check`; the generator now also
refuses any overlay whose `deviatesFrom` is not a real regular cell, and any
duplicate overlay id.

