## HL-C421-0bced6ad — The mock requires rows cover the passage but not the question stems and options

**Status: OPEN.** Found while pre-checking Spanish chapter 471 against A2 mock
1 item 31, by reading the item's options rather than only its `requires` row.

This is a finding about the **metric**, not about a chapter, and it matters
more than any single chapter does. The book-bounded audit is the number the
whole vocabulary programme steers by. If it can call an item passed while a
candidate could not actually answer it, the number overstates readiness — and
the stated goal is passing a real exam, not reaching a number.

### What the audit actually reads

`src/spanish-a1-mock-audit-cli.ts` does not derive requirements from the paper.
It parses them out of the answer-key table:

```
requires: row[2].split(",").map(clean)
```

So `requires` is **hand-authored metadata**, and the audit is exactly as
complete as those rows are. An item passes when every lexeme in its row is
taught — nothing checks the paper text itself.

### The demonstration, verified exactly

A2 mock 1, item 31. The answer key reads:

```
| 31 | b | A2-NE18-06 | curso, grupo, avanzado, perderse, principiante, sencillo |
```

All six are words of the **audio passage** — a woman explaining she moved
cooking classes because the Monday group was too advanced. The question is:

> **31.** La mujer cambió de grupo porque…
> - a) el horario no le venía bien.
> - b) **el nivel era demasiado alto para ella.**
> - c) el profesor no le gustaba.

`horario` is taught (`ES-C342-horario`). `profesor` is taught
(`ES-C09-estudiante`). **`nivel` is not taught at all**: a substring grep for
`nivel` across `spanish/lessons/` and `spanish/units/` returns **zero** hits,
so it is absent in every inflected form, not merely as a headword.

It is in the **correct answer**. A candidate who understood every word of the
dialogue still has to read option (b) to choose it. Teaching the six words in
the row does not make this item answerable.

### What is NOT claimed

The scale of this is **unmeasured**, and two attempts to measure it were both
too crude to report:

- Comparing raw word-forms in the A2 papers against every word-form in the
  corpus gave 706 "missing" words. That is badly overstated: most are
  inflections of taught lemmas (`amables` from the taught `amable`), exam
  apparatus (`audición`, `comprensión`, `afirmación`), or proper names.
- Filtering those by a shared five-character prefix gave 61. Still wrong, in
  the other direction of error: stem-changing and irregular verbs defeat
  prefixes, so `pidió`, `sabía`, `miró`, `dejé` and `venía` were all counted
  as gaps although `pedir`, `saber`, `mirar`, `dejar` and `venir` are taught.

The honest position is one verified instance plus a mechanism that predicts
more. Quantifying it needs the audit's own taught-forms set, which is
lemma-aware, rather than a grep.

### Proposed work, not taken

1. Extend the audit to check stem and option text as well as the passage, or
   add a separate check that every content word in a stem or option appears in
   the taught set.
2. Re-audit the existing `requires` rows against the paper text once that
   check exists, and repair the rows it flags.

**Deliberately not done here.** Either change moves the headline number, and
the number is the programme's steering signal — it wants its own branch, its
own review, and a statement of how much of the apparent gap was real. A
vocabulary chapter is the wrong place to change the instrument it is measured
by.

### MEASURED, and the rows repaired — 2026-09-22

**Status: the scale is no longer unmeasured.** Done on its own branch, as this
entry asked, and with the statement of how much of the apparent gap was real.

The two earlier attempts failed because they compared word FORMS against the
corpus. This one uses the audit's own taught set — the same
`lessonsUpToLevel("A2")` headword expansion the gate uses, copied rather than
re-derived — and then narrows by hand rather than by another heuristic:

**This table describes the FIRST pass only.** Its bottom line was wrong by a
factor of five; it is kept because the shape of the narrowing is what later
rounds had to correct.

| step | distinct forms |
|---|---:|
| in the two papers' stems and options | 592 |
| not a taught form (the overstated figure, as before) | 309 |
| after removing closed-class function words, exam apparatus and morphological relatives of taught lemmas | **47** |
| of those, not a headword | 21 |
| of those, **zero word-boundary hits anywhere in the corpus** | **9** |

The 47 were read individually; 15 of them are proper names (*Alberto*,
*Zaragoza*, *Nuria*…) and most of the rest are inflections the filter missed,
notably stem-changing verbs — `pide`/`pidió` from *pedir*, `sirve` from
*servir*, `va`/`vaya` from *ir*. That is the same trap this entry recorded, and
it is why the last narrowing is by reading rather than by rule.

**Forty-seven lexemes** were added, across **forty-three of the hundred rows** the gate reads (fifty per mock).

Every one is load-bearing. Most sit in an answer option a candidate has to
weigh — *"tiene que pagar una multa"*, *"tener el título universitario"*,
*"vigilar a los más pequeños"*, *"pague la reparación por adelantado"* — and
seven sit in a STEM, so the question cannot be read at all: `alumno` (m1 22),
`mejor` (m2 5), `practicar` and `infancia` (m1 18), `trescientos` (m1 24),
`quejarse` (m2 17), `costumbre` (m2 19).

### THE SEARCH DID NOT CONVERGE — thirty-eight corrections over three rounds

This is the part worth keeping, because it bounds how much the number can be
trusted. **The analysis was wrong three times, and never settled.** 6 → 15 → 17 → 32 → 48.
Each round of adversarial review found more untaught words sitting in items
that were passing — 3, then 17, then 18 — and *every one of the thirty-eight
corrections ran in the same direction*: the narrowing had been too generous.

A method wrong three times running in one direction is not nearly right. It is
**biased**, and the bias flatters the corpus. **48 is where the search stopped,
not where the truth is.**

The first draft added nine words. Review found three more:

| word | why it was missed | where |
|---|---|---|
| `espacio` | cleared by a **3-character prefix rule** matching the taught `esperar` — unrelated words | mock 2 item 5, option (a) |
| `ahorro` | cleared as a relative of a verb, but `ahorrar` is not taught either | mock 1 item 23, option (c) |
| `mejor` | held back on the claim that it "is a headword" — **false** | mock 2 item 5, stem |

A second round then found **seventeen more**, none of them a headword at or
below A2 and every one sitting in an item that was *passing*: `aburrido`,
`practicar`, `infancia`, `trescientos`, `finalidad`, `variedad`, `jardinero`,
`cancelar`, `concreto`, `equipaje`, `recuperar`, `quejarse`, `costumbre`,
`parecido`, `resolver`, `disponible`, `prometer`.

(An earlier draft said all seventeen had *zero occurrences in the entire
curriculum*. That is false for two: `aburrido` appears in `CHANGELOG.md` and
`roadmap.md` prose — where it is itself named as untaught — and `resolver` in a
`grammar-cells.json` overlay no lesson references. The absolute wording was
wrong; the substance was not.)

A **third** round then found eighteen more, and these are the ones that should
worry a reader most, because several sit in the **keyed** option — the correct
answer turns on a word the course never teaches:

| where | words |
|---|---|
| stem, so the question is unreadable | `profesional` (m1 14), `inscribirse` (m1 38), `descartar` (m1 42), `iniciativa`/`ganar` (m2 14), `comprender` (m2 16), `calzado` (m2 41) |
| the keyed option | `sitio` (m1 12, m2 6), `prever`, `sustituir`, `error` (m1 enunciados), `obligatorio` (m2 9), `inscribirse` (m2 12), `acudir` (m2 49), `comunicar`/`interrumpir` (m2 50) |
| a distractor to weigh | `material` (m2 12), `rápido` (m1 30) |

Three of that round's candidates were investigated and **cleared**, and they
are recorded because they show the standard: `disculparse` (`la disculpa` is
taught), `informar` (`el informe` is in a taught headword) and `pedir` (taught
inside a parenthesised headword). `coger` was left out as well — it appears in
a message body rather than a stem or option, so it is outside what this pass
measures.

Mock 1 item 36 is the pattern in one line. Its row reads *sección, cerrado,
inventario, horario, habitual* — five passage words — and the option the
candidate must choose reads *"comprar en una sección **concreta**"*.

The `mejor` claim is the one that matters most, because it was wrong in a way
that is easy to repeat. The only headword containing `mejor` is
`pasar a mejor vida`, and that lesson sits on `SPINE-READ-CULTURAL-WEIGHT`,
which derives to **C2** — outside this gate's own taught set. So `mejor`
belongs with `creer` and `explicar`: taught, but above the ceiling. Excluding
it while counting those four was internally inconsistent.

`afirmar` was also held back, as exam apparatus. It **is** apparatus in the two
instruction lines, but mock 1 item 41 is a scored statement — *"Afirma que sin
el curso no le darán el puesto"* — and that row already lists `curso`, `puesto`
and `dar` from the same stem. The row does read the stem; singling out one word
was inconsistent.

**Two exclusions do stand**, for a reason the book itself supplies: `escolar`
from the taught `la escuela`, and `comedor` — which `ES-C297-tenedor` does not
merely make derivable but **glosses outright**, *"A comedor is where the eating
is done"*, at chapter 297, **pre-A1**. An earlier draft of this entry credited
chapter 431 with the `-dor` ending; 431 does teach it, but 297 got there first
and with this very word.

One more correction of the same kind: the first draft listed `mejorar` for mock
1 item 23, but the paper reads *"la **mejora** de las notas"* — a deverbal
noun. Listing the infinitive lemmatised across a part-of-speech boundary and
would have let a future `mejorar` headword flip the item to passing while the
option stayed unreadable. The row lists `mejora`.

### What it cost the headline, and what that retracts

```
objectiveFailed           6 -> 48
missingObjectiveLexemes   4 -> 51
mock 1 reading           24 -> 12      listening 23 -> 13
mock 2 reading           22 -> 10      listening 25 -> 17
```

The instrument was understating by **eight times over**. The pinned note in
`spanish-a1-mock-audit.test.ts` says the count may only ever fall; this is the
one rise that is legitimate, because it is the MEASUREMENT being corrected to
be harsher, not the corpus losing ground.

**It also retracts a claim made three PRs running.** Chapter 474 was landed as
"the last authorable A2 vocabulary chapter" and the programme was declared
finished, on the grounds that the four remaining lexemes were already taught
and blocked only by `HL-C418`, `HL-C420` and `HL-C422`. That was true of the
list the instrument produced. The list was wrong. **Forty-six words that are
not headwords at or below A2 were invisible the entire time**, because nothing
ever put option text on it — and the programme was not out of words, it was out
of words *the passage needed*.

Three of the twenty-nine are glossed inside lesson bodies that are themselves
inside the A2 set — `alquiler` in `ES-C441-contrato`, `espacio` in
`ES-C57-es-inicial`, `mejora` in `ES-C466-visible`, which is the very lesson
supplying `visible` to the same row. Counting them missing is still right,
because this gate is headword-only by construction and `glossed-not-taught.ts`
treats body presence as a review queue rather than a teaching claim. But the
wording has to say *"not a headword at or below A2"* rather than *"never taught
anywhere"*, which an earlier draft of this entry claimed and which is false for
those three. `HL-C422` is where the headword-only rule itself is argued.

### What this does NOT fix

**The list is a FLOOR, not a census.** The narrowing from 47 was done by
reading, and review found **thirty-eight** false clears in it over three rounds — 3, 17, 18 — and never stopped finding them; the rules that
produced them — a 3-character prefix, a guess at a verb relative — are still
what filtered the other 545 forms. More gaps almost certainly remain, and every
one of them makes the number kinder than the truth.

The demonstration of that is in the table above: `espacio` was cleared because
it shares three letters with `esperar`. Nothing about that rule is sound; it
was a heuristic chosen to cut 309 candidates down to something readable, and
its errors all point the same way.

### The report-only CLI now exists, and it says 48 is still a floor

`npm run report:mock-stem-coverage` is the repeatable version of this pass,
with its vocabulary declared in `core/spanish-mock-stem-vocabulary.json` rather
than living in a script. It sorts every form of every stem and option into
buckets and prints the two a reader must adjudicate. **It never clears a word
on a morphological guess** — a form with a plausible taught relative goes to
`derivable`, which is still printed, beside the word it matched.

It found two bugs in the old method immediately:

- **The prefix rule never belonged.** `espacio` was cleared against `esperar`
  on a 3-character prefix. The reporter strips declared *suffixes*: `espacio`
  reduces to `espaci`, `esperar` to `esper`, so they never meet. A test pins it.
- **A `minStemLength` of 4 was too tight**, silently breaking `dice` from the
  taught `decir`. Because matches are printed, a loose rule costs a glance and
  a tight one costs an exam item. It is 3.

And on its first run it caught a flaw in itself: rows are written in *citation*
forms while papers carry *inflected* ones, so `alumnos` was re-flagged in the
very item whose row had just been repaired with `alumno`.

**What it reports today, after all 47 repairs: 39 unaccounted forms in each
mock.** `ayuntamiento`, `cuota`, `decisión`, `participar`, `presentarse`,
`reparación`, `propuesta`, `condiciones`, `universitario`, `huerto` — each in a
stem or an option, with no taught relative, no declared exemption, and no
mention in its item's row.

**Those 78 were deliberately not added by hand.** Doing so would be the fourth
biased pass, and the finding of this entry is that the fourth pass is the
problem. `48` stands as the committed number, with the reporter beside it
saying plainly how far short of the truth it is.

`requires` is still hand-authored, so the rows can still drift — but the drift
is now something a command surfaces rather than something that waits for
somebody to think of checking.
