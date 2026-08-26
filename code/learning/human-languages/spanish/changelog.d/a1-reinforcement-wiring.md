## Unreleased — A1 reinforcement, the half that needed no prose

Spanish's `reinforcement` blocker at A1 stood at **83** atoms revisited fewer
than twice (HL09 §3.1 criterion 4). It is now **60**. Twenty-three atoms closed,
**no lesson added, and not one word of new prose written** — because in every
case the corpus was already revisiting the atom and had never said so.

**This does not close the criterion.** `reinforcement(-60)` remains Spanish's
sole A1 blocker, and Spanish stays `attained: pre-A1`, `inProgressAt: A1`.

### The paired atom that lost its twin

Every object-pronoun lesson introduces **two** atoms for one teaching moment:
`ES-C42-lo` introduces `ES-LEX-LO-OBJECT` *and* `ES-GRAMMAR-DIRECT-OBJECT-LO`,
and `la`, `te`, `nos`, `os`, `le` all do the same. Every downstream review then
declares the `ES-GRAMMAR-*` twin and **none of them declares the `ES-LEX-*`
twin** — while printing the word itself in its own recap table:

```
| it (masculine) | **lo** | *lo tengo* — I have it |
| you            | **te** | *te quiero* — I love you |
```

Reviews are written about rules, so the atom naming the rule gets cited and the
atom naming the word gets dropped. This is the `roots: []` finding from the verb
tranche one field over: a declared ledger is what an author wrote down, not what
the lesson spends.

| atom | now practised by | the line that already did it |
|---|---|---|
| `ES-LEX-LO-OBJECT` | `ES-C42-repaso-objeto`, `ES-C42-me-objeto` | the recap row for *lo tengo*; "the same job *lo* does for a book" |
| `ES-LEX-LA-OBJECT` | `ES-C42-repaso-objeto` | the recap row for *la hago* |
| `ES-LEX-TE-QUIERO` | `ES-C42-repaso-objeto`, `ES-C45-nos` | the recap row for *te quiero*; "And I love you. (*Te quiero*.)" |
| `ES-LEX-NOS-OBJECT` | `ES-C45-repaso-ocho` | the eight-cell grid's *nos* column |
| `ES-LEX-OS-OBJECT` | `ES-C45-repaso-ocho`, `ES-C46-les` | the *os (Spain)* row, in both grids |
| `ES-LEX-LE-OBJECT` | `ES-C46-cual-pide` | "*Hablo a Ana* → ***Le** hablo.*" |
| `ES-LEX-NOSOTROS` | `ES-C45-os` | "**vosotros** (*vos* + *otros*), built exactly like *nosotros*" |
| `ES-GRAMMAR-OBJECT-SETS-OVERLAP` | `ES-C46-sintesis-dar-y-decir` | the *lo/la/los/las* against *le/les* contrast, in full |
| `ES-LEX-O` | `ES-C61-repaso-juntores` | "*Siete u ocho.* — *O* steps aside as ***u***" |
| `ES-LEX-MALO` | `ES-C60-repaso-grado` | "***Mal*** … is *malo* with its ending bitten off" |
| `ES-LEX-CON` | `ES-C64-repaso-tonicos` | "*¿Vienes conmigo?* — the two that fuse" |
| `ES-GRAMMAR-CONTRACTION-OBLIGATORY` | `ES-C61-o` | "*a el* into *al*, and now *o* into *u*" |
| `ES-GRAMMAR-DEMONSTRATIVE-BEFORE-NOUN` | `ES-C59-ese` | "*Este libro* is the one in my hand." |
| `ES-GRAMMAR-DEMONSTRATIVE-AQUEL` | `ES-C65-ahi-alli` | "three pointing words: *este*, *ese*, *aquel*" |
| `ES-GRAMMAR-DEMONSTRATIVE-NEUTER` | `ES-C61-quien` | "*¿Qué es esto?* asks about a **thing**." |
| `ES-GRAMMAR-QUIEN-PEOPLE-ONLY` | `ES-C62-veo-a-maria` | "Remember what happened to *quién*" |
| `ES-GRAMMAR-GERUND-MEANING` | `ES-C62-comiendo`, `ES-C62-estoy-hablando` | "*Hablar* gave *hablando*"; "***Estoy hablando.***" |
| `ES-GRAMMAR-GERUND-TWO-ENDINGS` | `ES-C62-estoy-hablando` | the *Estoy **comiendo*** row |
| `ES-GRAMMAR-PERSONAL-A-WHY` | `ES-C62-repaso-gerundio` | "*Veo a María*, but *veo la casa.*" |
| `ES-GRAMMAR-PRETERITE-AR-COMPLETE` | `ES-C63-comisteis`, `ES-C63-repaso-paradigmas` | "*Hablasteis*. Now the other two families."; "The form the book owed you longest." |
| `ES-ORTHOGRAPHY-MI-ACCENT` | `ES-C64-conmigo` | "*Para mí*, *para ti*." |
| `ES-GRAMMAR-VOCATIVE-NOT-OBJECT` | `ES-C64-repaso-tonicos` | "*María, ven.* — a comma, and that is the whole grammar" |
| `ES-GRAMMAR-INDEFINITE-PLURAL-AGAINST-BARE` | `ES-C67-uno-otro` | "*Un libro* — one book. *Unos libros* — some books." |
| `ES-GRAMMAR-POSSESSIVE-GENDER-AGREEMENT-SINGULAR` | `ES-C65-vuestro` | "*Nuestra casa* — our house." |
| `ES-GRAMMAR-IR-PRESENT-3PL` | `ES-C55-comieron` | the *comer / comen / comieron* paradigm row |

### About a third of the screen's proposals were rejected, on purpose

The screen indexes every later lesson's block-level prose and prints the line
that would justify the claim. A match is a prompt to go read the line, not a
verdict, and reading them threw a third out as **the detector matching rather
than the teaching**:

- *ocho* in "Tengo ocho años" does not revisit `ES-CULTURE-ROMAN-MONTH-NAMES`.
- *libro* in "Un libro — one book" does not revisit `ES-GRAMMAR-ORDINAL-APOCOPE`.
- *grande* in "muy grande" does not revisit `ES-GRAMMAR-EXCLAMATIVE-QUE`.
- *hablo* in a six-form paradigm chant does not revisit
  `ES-GRAMMAR-PROGRESSIVE-IS-OPTIONAL`.
- *comida* being pluralised does not revisit `ES-GRAMMAR-NOUN-FROM-VERB-IDA`,
  which is about deriving the noun from the verb.

Accepting those would have reported more atoms closed and taught nothing, and it
is worse than leaving them open, because a wired atom leaves the report. They
stay open and are named here so the next tranche knows they are owed prose.

### Closure is the ceiling on this technique

An atom may only be practised by a lesson that already holds it in its
transitive `prerequisites` closure — `schema-v2-practice-before-introduction`.
Spanish's chains break at chapter joints (`ES-C346-escuela` reaches back to
`ES-C334-edad`, skipping chapter 345 entirely), so "a later lesson says the
word" is never sufficient on its own. Every edit here was closure-checked first.

### What is left, and why it needs lessons rather than wiring

The residue of 60 has one shape: **each was revisited exactly once, by its own
chapter's `repaso`, and never again** — HL09 §7.1's "eight self-contained
mini-courses", measured atom by atom. A widened sweep found no further honest
wiring anywhere in the track.

The existing `repaso` chain cannot absorb them either. Those lessons compute at
265–297 seconds against the hard 300-second limit, giving roughly 37 prompts of
total headroom against about 90 atom-slots needed. That is HL09 §7.2's stated
condition for dedicated `review` lessons, so the next tranche authors them.

### The unmeasured tail, measured: exactly two

Reinforcement windows are only judged where the track is long enough to contain
them, so a track's final lessons are never judged. Instrumenting that directly
rather than reasoning about it gives Spanish's tail as exactly **two** atoms:
`ES-LEX-BORRAR` (position 996, closes its only measurable window on one revisit)
and `ES-LEX-GRITAR` (position 997, the last lesson, so no window exists at all).

Both become blockers the moment any lesson lands after them. They are recorded
open rather than papered over — adding a lesson purely to make a window
measurable would be gaming the schedule, not satisfying it.

### Also

- The `-83` figure was checked against the possibility that the 32 new verb
  lessons had grown it, and they had not. Those lessons chain
  `practises.knowledge` correctly and sit close enough to the track end that few
  windows are measurable.
- **No headword moved.** No lesson is added by this change, so at-or-below pre-A1
  stays at **304** (floor 300) and at-or-below A1 stays at **617 / 40**, each
  re-measured from a from-scratch `dist/`.
