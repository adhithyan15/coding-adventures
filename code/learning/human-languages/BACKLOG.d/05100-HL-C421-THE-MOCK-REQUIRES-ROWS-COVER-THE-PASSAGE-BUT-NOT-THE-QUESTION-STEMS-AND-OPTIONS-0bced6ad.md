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
