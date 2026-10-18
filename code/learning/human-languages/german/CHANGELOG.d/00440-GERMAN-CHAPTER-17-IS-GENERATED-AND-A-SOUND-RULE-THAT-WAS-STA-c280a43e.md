## German chapter 17 is generated, and a sound rule that was stated twice and owned nowhere gets an atom

`ch17-food.tex` is now generated as **chapter 17, *Bread, Water, Wine***.
German's hand-written chapters: **3 -> 2**. **No renumber** — seven lessons into
the ten free sequence slots between chapters 16 and 18.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **7** (sounds ×2, cousinweb, grammarlens, culture ×2, etymology) |
| `grep -l '^chapter: 17$'` | **2 lessons** |
| the `.tex` | **2 sections**, one three-column table, 2 cousinweb, 2 culture, 1 grammarlens, 1 etymology, 2 sounds |
| taught-form census | *Brot*, *Wasser*, *Wein* — **3 forms, all headwords nowhere** |

**6 atoms against a ceiling of 12.** The chapter reads bigger than it costs
because **four** of the things the `.tex` presents as new are already owned:

| already owned | from | what it pays for here |
|---|---|---|
| `GE-LEX-DER-DIE-DAS-02` | ch. 1 | the "third gender — *das*" grammar lens |
| `GE-GRAMMAR-GROSSSCHREIBUNG-01` | ch. 4 | the capitalisation culture box |
| `GE-SOUND-EI-AS-EYE-01` | ch. 6 | *Wein* rhyming with English *vine* |
| `GE-GRAMMAR-NATIVE-VS-LOAN-01` | ch. 9 | the whole thesis, for the **fifth** time |

Chapter 1 already teaches all three articles **by name**, so the `.tex`'s "German
has a third: *das*" is a payoff, not a discovery. What is new is that *das* at
last has nouns to sit on.

### A rule stated twice and owned nowhere

Chapter 15's `GE-C09-winter` says "German *w* is a *v*" in prose and mints
nothing. Chapter 16's `GE-C10-schw` then says "German *w* is a **v**, which you
have from *Winter*" — a dependency that is real, load-bearing and **undeclared**.

So this chapter does **not** mint the atom where the `.tex` happens to teach it.
Two upstream edits instead:

* `GE-C09-winter` now introduces `GE-SOUND-W-AS-V-01`, which is where the reader
  actually first learns it. Chapter 15 goes 10 → 11 atoms, under the ceiling.
* `GE-C10-schw` now **requires** it, so what it already said in prose is declared.

Chapter 17 then spends it on *Wasser* and *Wein* — its third use. Same shape as
the `GE-LEX-JAHR-01` move in the chapter-13 PR: **an atom belongs where the
reader first meets the fact, not where a hand-written chapter wrote it down.**

### The chapter's own test, handed to the reader

The `.tex` asserts that *Wein* is a loan. The lessons let the reader **check** it:

| pair | how far apart |
|---|---|
| *Wasser* / water | a whole consonant — *ss* against *t* |
| *Mutter* / mother | two shifts of distance |
| *Wein* / wine / *vīnum* | **no distance at all** |

Inherited words drift, because a sound law runs through them. Three words that
match this closely are not three cousins who happen to look alike — they are one
word, borrowed, arriving too late for the law to touch it. That turns "*Wein* is
a loan" from something to believe into something to notice.

### Counters, re-measured against the merged tree

| counter | before | after |
|---|---|---|
| German hand-written chapters | 3 | **2** |
| `handwritten_parity.py` german | 22 | **15** blocks at risk |
| German lessons (schema-v2) | 262 | **269** |
| atoms taught | 433 | **440** |
| atom-measurement-blind lessons | 6 | **4** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 26 | **27** |
| atoms never revisited | 81 | **81** |
| forward references | 29 | **27** |

Atoms rise by seven for six new ones, because `GE-SOUND-W-AS-V-01` is the
seventh and it lands in chapter 15. Forward references fall by two because the
schema-v1 `GE-C11-brot` named *Wasser* and *Wein* before either was taught.

