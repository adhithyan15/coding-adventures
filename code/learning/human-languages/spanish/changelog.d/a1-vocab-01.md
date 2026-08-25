## Unreleased — A1: the first thirty words (chapters 340-345)

### Added

- **Thirty A1 vocabulary lessons across six chapters**, the first tranche
  authored against the A1 gate rather than pre-A1. Spanish entered at **379**
  distinct headwords taught at or below A1, against the HL09 §3.1 floor of
  **600**; it leaves at **409**. The remaining shortfall is **191**. Spanish
  stays at **pre-A1 attained** — this tranche moves the A1 vocabulary
  criterion and regresses nothing.

  All six extensions are declared `stage: "A1"`, so every one of the thirty
  words counts at A1 and not merely below it.

  - **Chapter 340, *Near, Far, and Which Side*** — `cerca`, `lejos`,
    `la derecha`, `la izquierda`, `enfrente`. Distance first, then the two
    sides, then the thing that faces you.
  - **Chapter 341, *Above, Below, In Front, Behind*** — `arriba`, `abajo`,
    `delante`, `detrás`, `dentro`. The vertical axis, the front-and-back axis,
    and the inside.
  - **Chapter 342, *The Clock, Taken Apart*** — `el minuto`, `el cuarto`,
    `el punto`, `el horario`, `la madrugada`.
  - **Chapter 343, *Naming a Date, and the Stretches Around It*** — `la fecha`,
    `el calendario`, `próximo`, `la época`, `el siglo`.
  - **Chapter 344, *Zero, Halves and Dozens*** — `cero`, `el número`,
    `la mitad`, `la docena`, `doble`.
  - **Chapter 345, *Pinning Down Which One*** — `mismo`, `cada`, `la parte`,
    `la persona`, `el objeto`.

- **Four A1 spine nodes now carry track-local vocabulary extensions.**
  `SPINE-ASK-LOCATION` takes chapters 340-341, `SPINE-TIME-OF-DAY` takes
  342-343, `SPINE-COUNT-ONE-TO-FIVE` takes 344 and `SPINE-DEFINITE-REFERENCE`
  takes 345 — two chapters each on the two nodes with ten words' worth of
  genuine A1 function, one each on the other two. The four nodes declare only
  about twelve concepts between them, which cannot host thirty words directly;
  the extension mechanism is the same one chapters 303-334 used at pre-A1.

- **Payoffs on words the learner already half-owns.** `el cuarto` opens out of
  the `cuatro` the track has counted since chapter 8; `dentro` opens out of
  `la entrada`; `la mitad` is assembled in front of the learner from the
  `-dad` of *verdad*/*ciudad*/*edad* and the `medius` of
  *mediodía*/*medianoche*; `delante` reuses the `ante` of *antes*; and
  `el horario` and `el calendario` share the `-ario` ending across two
  chapters.

- **Wiring** — six `chapters.json` entries, six `ES-PATH-340-01`…
  `ES-PATH-345-01` curriculum paths with one `extensions` entry per chapter,
  each path appended to its spine node's `segments`, and six
  `core/book-generation.json` targets anchored inside `targets`. Modality,
  narration, gentle-ramp snapshots, book chapters and track progress
  regenerated in that order.

### Notes

- **Six allocated words were substituted after re-verification, and only one of
  the six was caught by a delimiter rule.** Because `vocabularyOf` compares
  whole headword strings, a near-duplicate never fails the build — it silently
  **inflates** the gate, which is the failure mode worth spending effort on.

  - `el lado` — **compound.** Already taught inside `por un lado… por otro`.
    The separator there is the Unicode ellipsis `…`, which is a *fifth*
    delimiter beyond the four the authoring rules name (`·` `/` `,` `+`, plus
    ` y ` ` o ` ` vs `). Splitting on `...` alone leaves the atom as `lado…`,
    which reads as free. Replaced with `enfrente`.
  - `el par` — **morphology.** `la pareja` is already taught and is the same
    root; `para` is taught as well. No delimiter reaches this. Replaced with
    `doble`.
  - `pasado` — **morphology.** It is the past participle of `pasar`, which is
    taught, alongside `pasaporte`. Replaced with `la época`.
  - `cualquier` — **compound.** It is `cual` + `quiera`, and `cual` is taught,
    but it is written as one orthographic word so no split exposes it.
    Replaced with `el objeto`.
  - `la media` — **morphology.** Same morpheme as the taught `mediodía` and
    `medianoche`, neither of which decomposes under any delimiter rule.
    Dropped; `la mitad` carries the halving instead, and says the *medius*
    connection out loud.
  - `fuera` — **morphology.** A homograph of the imperfect subjunctive of
    *ser*/*ir*, whose stem the track already teaches as `fueron`. Dropped in
    favour of `dentro`, which needs no such disclaimer.

  Two further candidates were blocked outright and never entered the pool:
  `el mes` (the track teaches `los meses`) and `cuántos` (already inside
  `¿Cuántos años tienes?`).

- **`la izquierda` is the one word here with no English cognate, and that is
  the lesson.** It is Basque *ezkerra*, not Latin — one of a small handful of
  pre-Roman survivals in Spanish. What it displaced was Latin *sinistra*, and
  English kept exactly the word Spanish threw away, as **sinister**. The
  English tie is to the discarded word rather than to the headword, which is
  stated plainly rather than papered over.

- **Thirty words, and the at-or-below-A1 count rose by exactly thirty**
  (379 → 409, total 483 → 513). An exact match is the check that no
  near-duplicate slipped through: a word already taught under a different
  string would have raised the total while adding nothing a learner does not
  already have.

- **Arithmetic.** Six chapters of five need every node's word pool to be a
  multiple of five, or slots strand. The pools were sized 10/10/5/5 across the
  four nodes and close exactly. Two verified-free words, `ambos` and `varios`,
  are **carried forward** to the next tranche rather than padding a seventh
  chapter of two.

- All six chapters are ear-only: each chapter's narration opens *"All 5 can be
  done entirely by ear."*

- **Re-verified after chapters 337-339 merged mid-flight.** That tranche also
  split two existing compound headwords (`ahora · hoy` and `vi · di`) into
  singletons, which moves the corpus the freedom check runs against. All thirty
  headwords were re-checked against the new 650-entry corpus and remain free.
