## German chapters 1 and 2 leave the hand-written set

The book's two opening chapters were hand-written LaTeX. Nothing built them from
the lessons, so every lesson-level gate — the five-minute ceiling, the atom
budgets, the ramp report — was true of the corpus and irrelevant to the pages a
reader actually opened. Both are now generated.

### What moved

- `core/book-generation.d/handwritten.d/german-0001.json` and `german-0002.json`
  are now `targets.d/` entries. German's hand-written chapters: **16 -> 14**.
  `handwritten_parity.py german`: **78 -> 77** blocks at risk.
- Nineteen legacy lessons migrated to schema v2. Generated books refuse anything
  else (`book.ts`: "generated books require schema version 2"), which is the real
  reason chapter 1 could not be flipped even though its measured prose gap was
  already zero. German's atom-measurement-blind lessons: **65 -> 46**.
- Both chapter payoffs are now atom-scored instead of authored boilerplate, and
  the chapter openings carry the hand-written introductions rather than the
  generator's "Complete GE-C0N-practice" stub. Corpus payoffs below the
  representativeness floor: **86 -> 85**.

### Prose carried rather than dropped

A block-count parity check cannot see a sentence lost inside a block that
survived, so each chapter was diffed word-for-word against its hand-written
original. Eight pieces of chapter 1 were living only in the LaTeX and have been
re-homed in the owning lesson:

- the plurals *die Tage*, *die Morgen*, *die Abende*, *die Nächte*, and the rule
  that every German plural takes *die* whatever the gender — now the new lesson
  **GE-C01-die-plural**, which also teaches the umlaut that *Nächte* writes down;
- *Tag* rhyming with English *tock* rather than *tag*, and the final *-g*
  hardening to *k*;
- the *Nacht* **ch** as the same throat-scrape as Spanish *j* and Arabic *kh*;
- the full *ich wünsche Ihnen einen guten Tag* behind the frozen *Guten Tag*;
- first-syllable stress on *Morgen* and *Abend*, the *-en* sinking to *-un*, the
  crisp final *t* of *gut*, the vowel-like murmur of *der*, and French *le/la*
  beside Spanish *el/la*.

Chapter 2's one measured gap was its second `cousinweb`: the etymology of *mich*,
which the hand-written chapter taught inside the *freut mich* section under a
heading the renderer has no type for. It is now **GE-C02-mich**, a lesson of its
own placed *before* *freut mich*, so the phrase no longer uses a word the book
has not taught.

### Splits, and the rule behind them

Three lessons were created, all by splitting rather than by inventing content:

| new lesson | lifted from | why |
|---|---|---|
| `GE-C01-die-plural` | the four day-part grammar lenses | a transferable rule, and four plurals with nowhere else to live |
| `GE-C02-heissen-endungen` | `GE-C02-heissen` | *-e / -t / -en* is "the same machine on every German verb", not a fact about *heißen* |
| `GE-C02-mich` | `GE-C02-freut-mich` | a second headword inside a one-headword lesson |

The rule applied throughout: one lexical, one sound and one etymology atom per
word lesson; a grammar lens that explains *this word* assesses the lexical atom,
while a grammar lens that teaches a transferable rule gets its own atom and, if
that makes four, its own lesson.

`GE-C02-du-sie` also moved from sequence 80 to 68, ahead of *heißen*, so the
conjugation lesson can name *du* and *Sie* without borrowing them from later in
the chapter.

### One defect the parity gate could not see

Reading the generated chapter against its hand-written original — the step a
block count cannot do for you — caught a rendering fault. The High German
Consonant Shift table in `GE-C01-gut` and the day-roots list in `GE-C01-tag`
were written *inside* a markdown blockquote, and the book renderer does not nest
block structure inside a quote. Both collapsed into one unreadable run-on line
with stray `>` markers left in the LaTeX:

```
| English | German | |---|---| | good | gut | | day | Tag | ...
```

Un-nesting them, prose untouched, gives the generator what it can render: a real
`tabularx` for the four shift pairs and an `itemize` for the two day-roots. No
other German lesson nests a list or table in a blockquote, and no other
generated German chapter carries the artifact.

### What this reveals, and does not fix

Making these chapters measurable makes them measured. Chapter 1 introduces 31
atoms across 14 lessons and chapter 2 introduces 23 across 10, against a
twelve-atom chapter budget, so German's `atom-step` queue moves 8 -> 10 and the
corpus line from 27 chapters above 12 to 29. That debt was always there; schema
v1 simply hid it from the gate. Paying it means splitting the chapters
themselves, which renumbers every later German chapter and belongs in its own
change — recorded in `BACKLOG.d` as HL-C242.

### Verification

`node dist/cli.js validate` clean; the full vitest suite green apart from the
pre-existing `figure*.test.ts` (missing paint-vm dependency) and
`script-closure.test.ts` (477 vs a >500 pin, on non-Latin tracks German is not
part of). Every `check:*` gate exits 0. The German book compiles under XeLaTeX
to 211 pages with zero overfull, underfull or missing-character warnings,
holding its all-zero entry in `core/latex-warning-baseline.json`.

