## HL-C286 — size a French chapter by the words it teaches, not the blocks it holds

French chapters 6, 7 and 8 are now generated. **Ten hand-written French chapters
remain.** This entry records the measurement those three needed and the parity
gate could not give, and the per-chapter numbers it produced, so the remaining
ten do not have to rediscover them.

### The parity gap is not an estimate of the work

`handwritten_parity.py --check french` scored these three chapters at **0, 1 and
1** blocks of gap. The authoring they actually needed was **12, 9 and 9
lessons** against the 2, 2 and 2 they had.

The gate is not wrong; it answers a different question. It counts LaTeX
*environments* that would disappear on a flip. The debt is in *words*: **a word
taught in a paragraph inside a surviving `cousinweb` costs zero blocks and a
whole lesson.** Chapter 8's gap of one block hid `il est … heures` — the frame
the entire chapter runs on — plus the `une heure`/`deux heures` agreement and
three time expressions the `.tex` named in a single sentence and then deferred
with the words *"add … later."* None of them was owned by any lesson. None of
them cost a block.

### The measurement that does size it

**Count the French words the `.tex` teaches, against the lessons that own them.**
Extract every `\emph{}` and `\textbf{}` span from the chapter, drop English (a
system dictionary), drop cited Latin (macron-bearing forms and a named list of
etymons), then bucket the rest into *owned by a lesson in this chapter*, *owned
by a lesson elsewhere*, and **owned by no lesson at all**. The third bucket is
the authoring queue.

Two limits, stated so the numbers are read correctly. It is a **candidate list
for a human**, not a count to trust: the `.tex` marks French, cited Latin and
English emphasis with the same `\emph`, so no filter separates them perfectly.
And it **under-reports accented headwords**, because a chapter written with
`\`{e}` macros yields fragments — chapter 10 reports zero owned words while
`FR-C10-parents` plainly owns *père* and *mère*. Read the chapter; use the list
to know what to look for.

### What it says about the ten that remain

Per chapter, the French vocabulary the `.tex` teaches against the lessons that
exist:

| ch | title | lessons today | words the `.tex` teaches |
|---|---|---|---|
| 1 | Greetings | 11 | already dense; parity 0 |
| 2 | Introductions | 9 | `nom`, `s'appeler`, `vouvoyer` unowned |
| 9 | Months and Seasons | 2 | **twelve months + four seasons** |
| 10 | Family | 2 | *père, mère, frère, sœur* — one lesson holds two each |
| 11 | Bread, Water, Wine | 2 | *pain, eau, vin* |
| 12 | Numbers 11–20 | 2 | **ten numbers**, *onze* through *vingt* |
| 13 | Colours | 2 | *noir, blanc, rouge, bleu* |
| 14 | To Have, and Age | 2 | *avoir* across six persons, plus *ans* |
| 15 | The Compound Past | 2 | the participle system, plus *passé simple* |
| 16 | To Be, and its Past | 4 | *être* across six persons, **plus the whole set of verbs that select it** |

Chapters 9, 12 and 16 are the large ones, and each raises the same design
question rather than a drafting one: **`maxNewAtomsPerChapter` is 12**, and a
chapter that honestly teaches sixteen months-and-seasons, or ten numbers plus
their formation, or a six-person paradigm plus a verb list, cannot fit inside it
one-atom-per-word. Chapters 6, 7 and 8 each landed at exactly 12, 12 and 11 by
folding related facts into the atom of the word that carries them — the *six*
and *neuf* liaisons became one `FR-PHON-LIAISON-NUMBERS` atom rather than two,
and chapter 6's calendar lesson introduces no atom at all because its job is to
make four already-taught facts click together. That technique will not stretch
to sixteen months. Deciding whether the budget or the chapter boundary gives is
work for whoever takes chapter 9, and it should be decided before drafting, not
after the ramp report goes red.

### Three defects only the rendered page showed

Compile with `book-cli.js --materialize-compile-inputs` and XeLaTeX, and **look
at the pages**. A text assertion passes on all three of these:

- A dialogue written as consecutive `>` lines **joins into one run-on
  paragraph**. Separating the turns with a blank line between separate
  blockquotes fixes it; a bare `>` line does not — it leaks a literal `>` into
  the output.
- `info-dump` read chapter 8's clock tables as **verb paradigms**, because every
  row began *Il est …* and its French person list contains `il`. The tables are
  clock times. The fix improves them anyway: the repeated frame moved into a
  one-line instruction above the table, leaving rows that carry only what
  differs.
- `FR-C07-vendredi` declared the sound tag `nasal-an` while its own prose said
  *"the opening **en** is nasal."* **Sound tags are validated for registry
  membership and never against what the lesson says**, so a tag that contradicts
  its lesson is green in every gate and only catchable by reading.

### Two things that move in the honest direction

`chapter-references.test.ts` refuses to let cross-chapter prose references grow,
and caught *"the article you met in Chapter 1"* in a first draft. And teaching a
word makes previously-invisible forward references appear, because the gate
cannot see a violation for a word no lesson teaches — so expect those counts to
rise legitimately as authoring proceeds. French's fell instead (51 → 48 across
the three chapters), which took deliberate work: the *"Next: …"* pointer closing
each lesson names the next word **in English**, and chapter 6's gender example
uses *un jour* / *une nuit* from chapter 1 rather than the *café* chapter 28
owns.
