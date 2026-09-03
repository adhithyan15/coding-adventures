## HL-C312 — a tense chapter costs one lesson per person slot, and the .tex prints three

HL-C287 recorded that a paradigm is the cost of a chapter and that no grep
reports it. German chapter 15 is the same finding with the numbers moved, and it
is worth recording separately because the shape it took here is the one a
compound tense always takes.

The four instruments said, in order:

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **5 blocks** |
| `grep -l '^chapter: 15$' lessons/*.md` | **3 lessons** |
| the `.tex` | **2 sections** |
| taught-word census | **11 German forms** |

The chapter became **twenty-seven lessons across three chapters**, and every
instrument above was reading the wrong axis.

### The `.tex` shows three cells because three is enough to see the pattern

A hand-written chapter prints a paradigm at whatever size makes the point. Ch. 15
printed the *Perfekt* with three rows — *ich habe gesagt*, *du hast gesagt*, *wir
haben gesagt* — because three rows are enough for a reader to infer the fourth.
Under `maxNewGrammarCellsPerLesson: 1` that inference is exactly what is
forbidden: a cell the reader was left to guess is a cell nobody taught. So the
three rows become four lessons, not three, because the chapter's own closing
practice line (*Er machte das … Er hat das gemacht*) uses a fourth person the
table never showed.

**A paradigm table in a hand-written chapter is a lower bound on its cell count,
never the count.** Read what the chapter *does* with the paradigm, not what it
prints of it.

### The running example was never taught

The chapter builds every participle it teaches on *sagen*, and *sagen* is not a
headword anywhere in the German track. The hand-written form could do that
because prose can use a word it has not introduced; a generated chapter cannot,
and it should not want to. Chapter 15 opens with *sagen* now, which is one lesson
the `.tex` gives no sign of needing.

The general form: **a hand-written chapter's running example is a lesson you owe,
and it is invisible in every count because it is not new material — it is
material the chapter assumed.** Worth checking on every migration: take the verbs
and nouns the `.tex` inflects and confirm each one has a lesson.

### Two forward references that did not happen

Both were avoidable by phrasing rather than by cutting a claim:

* the closing "Next:" line of the *Präteritum* chapter would have named *sein*,
  which chapter 18 teaches. It names the job instead.
* the survivor list would have named *war*, which chapter 19 teaches as part of
  the headword `bin, ist, war` — a comma-separated headword splits, so bare
  *war* is a taught word even though no lesson is called that. It says "the past
  of the verb for 'to be,' which arrives two chapters from now."

German's forward-reference count stayed at **40** across a 27-lesson addition.
The count moving is not by itself a defect; a count that moves for a reason you
did not intend is.

### The rule-statement gate fires on ordinary prose

`GE-C15-du-hast-gesagt` wrote "that is the part of this tense that **never has**
to be relearned" — a sentence about the learner, not about German. `info-dump.ts`
matched `never has` and charged the corpus a rule statement, pushing the pinned
ceiling from 30 to 31. Rewritten, not re-pinned. The pattern is deliberately
narrow and it is still worth reading `RULE_PATTERNS` before writing a lesson:
`always|never` followed by `takes|uses|has|is|comes|goes|means|ends|begins|
appears` fires whatever the subject of the sentence is.
