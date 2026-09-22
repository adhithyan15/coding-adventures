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

**Thirty lexemes** were added, across **twenty-six of the fifty rows**.

Every one is load-bearing. Most sit in an answer option a candidate has to
weigh — *"tiene que pagar una multa"*, *"tener el título universitario"*,
*"vigilar a los más pequeños"*, *"pague la reparación por adelantado"* — and
seven sit in a STEM, so the question cannot be read at all: `alumno` (m1 22),
`mejor` (m2 5), `practicar` and `infancia` (m1 18), `trescientos` (m1 24),
`quejarse` (m2 17), `costumbre` (m2 19).

### THE NARROWING RULES PRODUCED FALSE CLEARS — twenty of them, over two rounds

This is the part worth keeping, because it bounds how much the number can be
trusted. **The analysis was wrong twice before it settled**, and both times it
was wrong in the same direction: too generous. 6 → 15 → 17 → 32.

The first draft added nine words. Review found three more:

| word | why it was missed | where |
|---|---|---|
| `espacio` | cleared by a **3-character prefix rule** matching the taught `esperar` — unrelated words | mock 2 item 5, option (a) |
| `ahorro` | cleared as a relative of a verb, but `ahorrar` is not taught either | mock 1 item 23, option (c) |
| `mejor` | held back on the claim that it "is a headword" — **false** | mock 2 item 5, stem |

A second round then found **seventeen more**, every one with zero
word-boundary occurrences in the entire Spanish curriculum and every one
sitting in an item that was *passing*: `aburrido`, `practicar`, `infancia`,
`trescientos`, `finalidad`, `variedad`, `jardinero`, `cancelar`, `concreto`,
`equipaje`, `recuperar`, `quejarse`, `costumbre`, `parecido`, `resolver`,
`disponible`, `prometer`.

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
objectiveFailed           6 -> 32
missingObjectiveLexemes   4 -> 34
mock 1 reading           24 -> 14      listening 23 -> 19
mock 2 reading           22 -> 15      listening 25 -> 20
```

The instrument was understating by **more than 5×**. The pinned note in
`spanish-a1-mock-audit.test.ts` says the count may only ever fall; this is the
one rise that is legitimate, because it is the MEASUREMENT being corrected to
be harsher, not the corpus losing ground.

**It also retracts a claim made three PRs running.** Chapter 474 was landed as
"the last authorable A2 vocabulary chapter" and the programme was declared
finished, on the grounds that the four remaining lexemes were already taught
and blocked only by `HL-C418`, `HL-C420` and `HL-C422`. That was true of the
list the instrument produced. The list was wrong. **Twenty-nine words that are
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
reading, and review found **twenty** false clears in it over two rounds — three
in the first, seventeen in the second; the rules that
produced them — a 3-character prefix, a guess at a verb relative — are still
what filtered the other 545 forms. More gaps almost certainly remain, and every
one of them makes the number kinder than the truth.

The demonstration of that is in the table above: `espacio` was cleared because
it shares three letters with `esperar`. Nothing about that rule is sound; it
was a heuristic chosen to cut 309 candidates down to something readable, and
its errors all point the same way.

And nothing stops the rows drifting again. `requires` is still hand-authored,
and the only thing that now checks it against the paper is that somebody ran
this analysis once. A report-only CLI that re-runs the 592 → 47 narrowing on
demand would make it repeatable; the last step would still need a reader.
