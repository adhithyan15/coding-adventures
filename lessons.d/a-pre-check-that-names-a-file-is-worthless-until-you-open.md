---
category: Repo policy / workflow reminders
---

# A pre-check that names a file is worthless until you open the file

Tranche 4d's review produced a rule: before writing a lesson, grep the corpus
for every word it will teach, because the lesson that *owns* a rule sits early
and outside any later lesson's prerequisite closure.

Chapter 465 applied that rule. The grep ran, and it printed:

```
cuaderno         prior mentions:  1  ES-C451-carne.md
```

Four of the sixteen words had hits that got followed up, and each one changed
the lesson that was then written. `cuaderno` had one hit, it was the smallest
number on the list, and it did not get opened. `ES-C451-carne` turned out to
be fourteen chapters back and to contain:

- `roots: [quaternum-latin]` — the slug the new lesson should have joined
- "Four sheets of paper folded together, which is how a small notebook was
  made"
- "The same *quaternum* gave Spanish **el cuaderno**" — naming the headword
- a wrap-up: "What number is hiding in the word? (**Four** — *quaternum*.)"

The new lesson ran the same reveal as a discovery, asked the same wrap-up
question, minted a different root slug, and got the fact wrong on the way
past — it said one sheet folded once made four pages, when the four counts
sheets gathered into a fold. Its own `gloss:` field said "four sheets folded
once" and agreed with `carne` against its own body.

So the check did its job and the finding was discarded. The failure was
entirely in the follow-up, and the shape of it is worth naming: **a low hit
count reads as reassurance when it is the opposite.** Seventy-eight hits for
`visible` were obviously English prose and safe to dismiss in bulk. One hit is
one file, which is cheap to open, and a single mention of a headword is far
more likely to be a lesson that *taught* it than a passing use.

Rules that follow:

- A non-zero hit count is a **file to open**, not a number to weigh. Zero means
  proceed; anything else means read before writing.
- Rank the follow-up by hit count **ascending**, not descending. Many hits are
  usually incidental prose; one or two hits on a content word are usually a
  lesson that owns it.
- When a prior lesson turns up, copy its `roots:` slug rather than deriving one.
  `quaternum-latin` versus `quattuor-latin` splits a cousins family silently,
  because the join is exact string equality.
- Check the new lesson's own `gloss:` against its own body. Here the two
  disagreed, in one file, and that alone would have caught it.
