## Unreleased — pre-A1 reinforcement closed

`plan-cli --ceiling C2` reported `reinforcement/spanish/pre-A1` with **24
outstanding**: 24 atoms at or below pre-A1 that no two later lessons practise,
against HL09 §3.1 criterion 4. That count is now **0**, which clears one of the
two remaining blockers between Spanish and the first attained CEFR level in the
corpus.

### Thirteen atoms closed by recording a revisit that was already happening

The preferred fix, and in every case below the lesson's own prose already
retrieves the atom — the `practises.knowledge` entry and the block `assesses`
entry were simply never written down. No new claim is made about any lesson.

| atom | now practised by | the line that already did it |
|---|---|---|
| `ES-LEX-GUSTO` | `ES-C35-gustar` | "Spanish's own **el gusto** — the *gusto* of *mucho gusto*" |
| `ES-GRAMMAR-STRESS-EXCEPTION` | `ES-C06-cafe` | "Why is the accent written? (It overrides the usual vowel-final stress.)" |
| `ES-FORM-MANANA` | `ES-C05-hasta-manana`, `ES-C289-amanecer` | the *ñ* drill; and *mane* heard inside *mañana* |
| `ES-FORM-ESPANOL` | `ES-C06-hablo-espanol` | "[YOU SAY: "Hablo español" — *AH-bloh es-pa-NYOL*]" |
| `ES-ORTHOGRAPHY-OPENING-PUNCTUATION` | `ES-C06-hablo-espanol` | "ask it — "¿Hablas español?" (raise the pitch; ¿ opens it)" |
| `ES-HISTORY-AL-ANDALUS-LOANS` | `ES-C290-arroz` | "*arroz*, *azúcar*, *aceite* — three words carrying an Arabic *al-*" |
| `ES-LEX-HASTA-LUEGO` | `ES-C38-luego` | ""hasta luego" then just "luego" — same word, standing alone" |
| `ES-LEX-HASTA-MANANA` | `ES-C289-amanecer` | "[YOU SAY: *hasta mañana*, then *el amanecer*]" |
| `ES-PRAGMATICS-POLITE-REQUEST` | `ES-C40-comprar` | "[YOU SAY: "Un café, por favor"]" |
| `ES-GRAMMAR-NO-02` | `ES-C61-repaso-juntores` | "*No quiero café ni agua.* — *Ni* carries its own *no*" |
| `ES-GRAMMAR-BARE-LANGUAGE` | `ES-C14-hablar-preterite` | "[YOU SAY: "Hablo español. Hablé español."]" |
| `ES-LEX-HABLO-ESPANOL` | `ES-C14-hablar-preterite` | the same line |
| `ES-LEX-PARENTS-01` | `ES-C37-sentarse` | "Siblings — separate branches of one root, like *father* and *padre*." |

Where the atom was not already in the lesson's transitive prerequisite closure,
`requires.knowledge` gained it too — the validator enforces that an atom cannot
be practised before it is available, and in each case the dependency was real
and merely undeclared.

### Eleven atoms closed by new review lessons — chapters 318 and 319

The remaining eleven had no later lesson that honestly touches them, so the
schedule HL00 specified and never built now carries them. Seven `type: review`
lessons, introducing nothing, each drilling material the learner already owns:

- `ES-C318-repaso-despedirse` (seq 5020) — *hasta pronto*, and *promptus* inside *pronto*.
- `ES-C318-repaso-disculparse` (seq 5030) — *lo siento* against *perdón*, the split following the Latin.
- `ES-C318-repaso-genero` (seq 5040) — *la cabeza*, *el vino*, *la mano*, and the object pronoun as the witness that settles *el agua*.
- `ES-C318-repaso-marcas` (seq 5050) — the **¿** the Academy made official in 1754, and the *h* on *hermano* that was never spoken.
- `ES-C319-repaso-la-superficie` (seq 5060) — a second pass over the four facts a surface will not give you.
- `ES-C319-repaso-salir` (seq 5070) — the five words of a leaving, and the five unrelated quarries they came from.
- `ES-C319-despedida` (seq 5080) — *el puerto*, *el faro*, *hasta pronto*.

The last two exist because of a measurement subtlety worth recording. A
reinforcement window is only judged when the track is long enough to contain it,
so the atoms of the final chapter had never been judged at all. Extending the
track by five lessons made chapter 302's `ES-LEX-C302-AWAY-04` and `-05` visible
for the first time, with 1 and 0 revisits. They are closed here rather than left
for whoever next appends to the track and finds a number that went up.

### Also

- Chapters 318 and 319 added to `spanish/chapters.json`, `spanish/curriculum.json`
  (five path segments, two extensions, four spine-node segment lists),
  `core/book-generation.json` and `spanish/book/book.tex`.
- Every generated artifact regenerated: lesson modality, narration, figures,
  track progress, gentle-ramp snapshots and book chapters all report clean
  under `--check`.
