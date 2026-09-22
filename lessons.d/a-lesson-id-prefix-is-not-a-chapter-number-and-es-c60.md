---
category: Repo policy / workflow reminders
---

# A lesson id prefix is not a chapter number, and ES-C60 is chapter 229

Chapter 468's draft glossed *el grado* as *"the same word as the scale you have
had since chapter sixty"*, reasoning from the id `ES-C60-repaso-grado`.

That lesson's front matter says `chapter: 229`. Its siblings are 226, 227, 228.
`chapters.d/0060.json` is a different chapter entirely — *"Synthesis ---
Describing Things"*. The `ES-C60-` prefix is a **legacy module id**, retained
across a renumbering, and it tracks nothing about where the lesson now sits.

The drift is roughly 170 chapters at that point in the corpus and it is not
constant, so no mental offset fixes it.

The corpus itself gets this right and shows how: `ES-C60-repaso-grado` writes
*"-ante, from chapter fifty-two"*, and `ES-C09-estudiante` — note the `C09`
prefix — really is `chapter: 52`. Prose that says "chapter N" means the
`chapter:` **field**, which somebody looked up.

Rules that follow:

- **Never read a chapter number off an id.** `ES-C<nn>-` is a name, not a
  coordinate. The only source for "chapter N" is `grep '^chapter:' <file>`.
- The same applies to "since", "earlier", "twenty chapters back" and any
  distance claim: those need `sequence:` or `chapter:`, not id arithmetic.
- When citing a lesson in prose, prefer naming it by **headword** (*"under
  *el patio*"*) over numbering it. The headword cannot drift; the number can,
  and a wrong number is a falsifiable claim printed in a book.
- If a number really is wanted, read both fields and say which one you mean.

Related but distinct: `lessons.d/` already records that the `roots:` ledger is
opt-in metadata and a lower bound. Both failures have the same shape — trusting
a machine-facing field to answer a question about the teaching. The prose and
the declared fields are different sources, and only one of them is what the
learner reads.
