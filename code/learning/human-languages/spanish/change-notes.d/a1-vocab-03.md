## Unreleased — A1 vocabulary, tranche 3: thirty-five words (chapters 353-359)

### Added

- **Thirty-five A1 vocabulary lessons across seven chapters**, the third tranche
  on the run to the HL09 §3.1 A1 floor of 600 distinct headwords at or below A1.
  Spanish moves **444 → 479**, measured through the gate's own code path both at
  the branch point and again after rebasing onto current `main`.

  | Ch. | Spine node | What it buys |
  | --- | --- | --- |
  | 353 | `SPINE-ASK-LOCATION` | el hospital, el banco, la farmacia, la biblioteca, el museo |
  | 354 | `SPINE-ASK-LOCATION` | el hotel, el restaurante, el cine, el parque, el jardin |
  | 355 | `SPINE-ASK-LOCATION` | la pared, el techo, el piso, el rincon, el fondo |
  | 356 | `SPINE-TIME-OF-DAY` | desde, la cita, la etapa, la ocasion, de repente |
  | 357 | `SPINE-TIME-OF-DAY` | la Navidad, la Pascua, la vispera, el regalo, la tarjeta |
  | 358 | `SPINE-DEFINITE-REFERENCE` | el color, el tamano, la forma, la clase, la marca |
  | 359 | `SPINE-COUNT-ONE-TO-FIVE` | el metro, el gramo, el kilo, el litro, el peso |

  Sequences 6570-6910, one new atom per lesson, one headword per lesson. All four
  A1 spine nodes are used, and the arithmetic closes exactly: seven chapters of
  five is thirty-five, with no chapter borrowing a slot from another node.

- **This is the first Spanish tranche authored against the HL21 shards** rather
  than against the monoliths. Seven new `chapters.d/<NNNN>.json`, seven new
  `curriculum.d/path/`, seven new `curriculum.d/extensions/`, and one appended
  `segments` line in each of the four A1 `curriculum.d/spine/` nodes. The three
  monoliths — `chapters.json`, `curriculum.json`, `core/book-generation.json` —
  were **regenerated with `npm run unshard`**, never hand-edited, per HL21 §3.

### The duplicate hunt, and the eight candidates it killed

`vocabularyOf` compares whole strings, so a near-duplicate does not fail the
gate — it **inflates** it, which is the dangerous direction. Every candidate was
checked against the full corpus of 715 existing headwords after splitting them on
the compound delimiters, stripping articles, cutting `+ …` grammar annotations
and ellipses, and then by eye for shared morphemes. Eight were dropped:

- **`el lado`** — inside the taught `por un lado… por otro`, which the ellipsis
  hides.
- **`mil`** and **`el millón`** — `mil` is inside the taught `mil gracias`, and
  `millón` is built on it.
- **`el aeropuerto`** — contains the taught `el puerto`.
- **`la panadería`** — contains the taught `el pan`.
- **`la cifra`** — Arabic *ṣifr*, which is **exactly** the taught `cero`. Two
  spellings of one word, and no delimiter rule reaches it.
- **`faltar`** — Latin *fallere*, the same root and the same visible stem as the
  taught `falso`.
- **`las vacaciones`** — dropped on a check that turned out to be a **false**
  alarm worth recording: *vacāre* is not *vacca*, so it is genuinely unrelated to
  the taught `la vaca`. It shares the visible stem `vac-` with it anyway, which
  is the confusion this course exists to prevent rather than create, so it was
  set aside on pedagogy rather than on etymology.
- **`tanto`** — the one that needed the root ledger to catch. `tam-latin` is
  already spent, twice, on `también` (*tan bien*) and `tampoco` (*tan poco*).
  `tanto` **is** that *tan*, apocope and all. `el tamaño` survives the same check
  because *tam magnum* is a distinct lexeme, not a bound form of a taught word,
  and it now reuses `tam-latin` as reinforcement instead of colliding with it.

`la pascua`, `la víspera`, `el regalo`, `la clase`, `el peso` and `la ocasión`
were each cleared against a look-alike (`pasar`, `visto`, `regular`, `claro`,
`eso`, `casi`) by checking the morpheme rather than the string.

### Etymology worth the lesson

Three threads run the length of the tranche rather than sitting in one lesson.

**One Latin word, arriving twice.** `el fondo` is *fundus* borrowed from books;
`hondo` is the same *fundus* come down through mouths, with the *f* turned to
*h*. `la forma` is *forma* borrowed; `hermoso` is *formōsus* inherited. Both
lessons close on the change the course taught back at chapter 57, and `la clase`
completes the set from the other side — it kept its `cl-` precisely because it
never went through a mouth.

**Roots already in the learner's pocket.** `la Navidad` is *nāscī*, and so are
`nada` (*rēs nāta*, "a thing born") and `nadie` — a word for Christmas and two
words for nothing and nobody, off one verb. `el tamaño` is the *tan* of
`también` and `tampoco`. `el kilo` is set explicitly **against** the `mil` of
`mil gracias`: Greek *khīlioi* and Latin *mille* are two unrelated thousands.

**Two words that fell together.** `el metro` is Greek *métron*, a measure — and
the underground `metro` is not, being short for *metropolitano*, from *mētēr*, a
mother. Same syllable, no relation. `el rincón` makes the opposite point: it is
Arabic *rukn*, and it did not displace a Latin word, it supplied a distinction
Latin did not have — the corner **inside** a room, against `la esquina` out on
the street.

### Modality, budgets and pins

All seven chapters narrate as **"All 5 can be done entirely by ear."** Three
lessons were rewritten to earn that, and all three were the known false-positive
shape rather than real visual dependency: `ES-C356-desde` said "out from under
*the table*" as an English example sentence, `ES-C356-ocasión` said "once you
*see the* falling", and `ES-C357-tarjeta` said a coat of arms is a shield you
"*look at*". Figurative prose, matched by a deliberately blunt substring
detector. Rephrased rather than argued with.

**No ceiling was raised.** The `info-dump` `ruleStatements` ceiling of 30 was hit
once during drafting, by `ES-C354-hotel`'s pronunciation note reading "the *h* is
silent, as it always is here". It now shows the instance — "the same way it did
in *el hospital*" — and the corpus measurement is back at 30.

`forwardReferences` moves **295 → 308**. All thirteen are *earlier* lessons that
had already used one of these words in passing and now, at last, have somewhere
to point: `ES-C57-f-a-h` and `ES-C09-oso` on *forma*, four lessons on *hotel*,
two on *hospital*, two on *peso*, and one each on *biblioteca* and *desde*. Zero
of the thirteen are inside this tranche — the one that was, `ES-C353-hospital`
naming *hotel* a chapter before it is taught, was rewritten to promise the word
instead of spending it.

Atom spikes: zero, at both the lesson and the chapter budget. Banned words
(HL10 §7.4): zero added. `targets` in `core/book-generation.json` stays 23
contiguous runs for 23 languages, which is the invariant HL21 §5.3 pins.
