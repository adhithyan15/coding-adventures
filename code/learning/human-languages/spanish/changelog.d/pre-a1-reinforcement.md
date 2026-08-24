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

### Seventeen atoms closed by new review lessons — chapters 318 and 319

Eleven of the original 24 had no later lesson that honestly touches them, so the
schedule HL00 specified and never built now carries them. Eleven `type: review`
lessons, introducing nothing, each drilling material the learner already owns:

- `ES-C318-repaso-despedirse` (seq 5020) — *hasta pronto*, and *promptus* inside *pronto*.
- `ES-C318-repaso-disculparse` (seq 5030) — *lo siento* against *perdón*, the split following the Latin.
- `ES-C318-repaso-genero` (seq 5040) — *la cabeza*, *el vino*, *la mano*, and the object pronoun as the witness that settles *el agua*.
- `ES-C318-repaso-marcas` (seq 5050) — the **¿** the Academy made official in 1754, and the *h* on *hermano* that was never spoken.
- `ES-C319-repaso-la-superficie` (seq 5060) — a second pass over the facts a surface will not give you.
- `ES-C319-repaso-salir` (seq 5070) — the five words of a leaving, and the five unrelated quarries they came from.
- `ES-C319-despedida` (seq 5080) — *el puerto*, *el faro*, *hasta pronto*.
- `ES-C319-repaso-presentarse` (seq 5090) — an introduction start to finish: both names, the address, the reply.
- `ES-C319-dos-respuestas` (seq 5100) — *mucho gusto* beside *encantado*: a pleasure named, and a spell sung on.
- `ES-C319-repaso-respuestas` (seq 5105) — the five answering words, and the *ali-*, *nati* and *paucus* behind them.
- `ES-C319-otro-y-poco` (seq 5110) — the two that agree with what they measure, and the *p*→*f* road a third time.

### The tail-atom effect, recorded because it will recur

Six more atoms than the original 24 are closed here, and the reason is
structural rather than incidental. A reinforcement window is only judged when
the track is long enough to contain it, so the atoms introduced by a track's
LAST chapter have never been judged at all. Every vocabulary tranche therefore
ends in a short tail of atoms that are invisible until something extends the
track past them.

Adding these review lessons is exactly such an extension, and it surfaced the
tail of three separate tranches in turn:

| tranche | atoms it left | revisits found |
|---|---|---|
| chapter 302 (round 3) | `ES-LEX-C302-AWAY-04`, `-05` | 1 and 0 |
| chapters 303-305 (survival) | `ES-LEX-C305-NAME-04`, `-05` | 1 and 0 |
| chapters 314-317 (body/answers) | `ES-LEX-C317-ANSW-19`, `-20` | 1 and 0 |

Each pair became visible only after this branch grew past it, and each is closed
here rather than left for whoever next appends and finds a number that went up.

The general shape is worth stating: **any tranche that appends to the end of a
track leaves reinforcement debt that only the NEXT appender can see.** Closing
it belongs with whoever extends the track next, not with the tranche that
created it, because until the track grows the debt does not exist as a
measurement. Chapters 306-313 are still in flight; when they land they will
leave a tail of the same shape, and it will be visible to whoever follows them.

### Also

- Chapters 318 and 319 added to `spanish/chapters.json`, `spanish/curriculum.json`
  (eight path segments, eight extensions, five spine-node segment lists),
  `core/book-generation.json` and `spanish/book/book.tex`.
- A `review` lesson is not a `CONTENT_TYPE` realizing its node's concept, so
  `unclassified-curriculum-extension-lesson` requires each one to sit in an
  extension — and an extension may not span path segments. Every new segment
  therefore carries its own.
- Every generated artifact regenerated: lesson modality, narration, figures,
  gentle-ramp snapshots, book chapters and track progress all report clean
  under `--check`.
- Pinned test counters relaxed in the direction they actually move, keeping their
  existing numbers so parallel tranches do not serialize behind them. Several of
  these were relaxed independently by the sibling tranches; where both sides had
  a floor, the higher one is kept.
- `level-gate.test.ts`'s etymology-waiver test rewritten. It read Spanish and
  pinned `reinforcement.shortfall` to 24; Spanish no longer has a reinforcement
  blocker to read. The waiver is now proved by counterfactual — rename the
  etymology atoms out of the `-ETYMON-` convention and some track's shortfall
  must rise — which bites harder than the constant did and pins no corpus figure.
