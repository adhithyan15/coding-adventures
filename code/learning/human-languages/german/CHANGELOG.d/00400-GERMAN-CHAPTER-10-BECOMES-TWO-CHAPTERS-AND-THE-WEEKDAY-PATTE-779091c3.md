## German chapter 10 becomes two chapters, and the weekday pattern gets its exceptions

`ch10-days.tex` is now generated, as **chapter 10, *The Gods of the Week***, and
**chapter 11, *The German Weekend***. German's hand-written chapters: **7 -> 6**,
and German is the only track that has any. Old chapters 11-39 renumber to 12-40.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **4** |
| `grep -l '^chapter: 10$'` | **2 lessons** |
| the `.tex` | **2 sections**, two **four-column** `tabularx` tables |
| taught-form census | 7 day names + *Mond*, *Donner*, *Sonne*, *Sabbat*, *Sonnabend* |

**18 atoms against a ceiling of 12**, so two chapters at the `.tex`'s own
weekday/weekend seam: **11 and 7**. A three-way split (5/6/7) was considered and
rejected — 11 fits, and splitting *below* the ceiling for its own sake is not the
rule. The rule is the ceiling, not the median.

### The chapter's own thesis is a rule with two exceptions

The `.tex` closes by saying it: German names its days for **gods**, like English,
not for planets — **with two edits**. So the pattern gets a lesson
(`GE-C07-tag-muster`), five days apply it, and the two that break it get a lesson
each:

* ***Mittwoch*** has no god and no *Tag*. The medieval Church declined to name a
  day for Wodan, and English kept him where German did not. That is a decision
  you can **date and attribute**, sitting inside an everyday word — most
  etymology in this book is drift with nobody deciding anything.
* ***Samstag*** is not Saturn's day but the **Sabbath**, Hebrew *shabbāt* by way
  of Greek *sábbaton*, which makes German *Samstag* and Spanish *sábado* the same
  word while English alone still points at a Roman planet.

### Five untaught words the chapter was leaning on

*Mond*, *Donner*, *Sonne*, *Sabbat* and *Sonnabend* were headwords nowhere.
*Mond* and *Donner* are now taught **before** the days built on them, so *Montag*
and *Donnerstag* are assembled rather than memorised.

### Six payoffs spent rather than re-argued

| atom | owned by | spent on |
|---|---|---|
| `GE-LEX-TAG-02` | ch1 | the *-tag* in every day name |
| `GE-SOUND-NACHT-CH-01` | ch1 | *Mittwoch*'s raspy *ch* |
| `GE-LEX-ABEND-02` | ch1 | makes *Sonnabend* transparent |
| `GE-SOUND-EI-AS-EYE-01` | ch6 | *Freitag*'s *ei* |
| `GE-GRAMMAR-SOUND-LAW-01` | ch6 | *Donner* from *thunder*, *th* -> *d* |
| `GE-GRAMMAR-NATIVE-VS-LOAN-01` | ch9 | *Samstag*, for the second time |

Three of those six did not exist before chapter 6 was migrated. Working bottom-up
is what makes them available rather than accidental.

### Counters, re-measured against the merged tree

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 7 | **6** |
| corpus hand-written chapters | 7 | **6** |
| `handwritten_parity.py` german | 39 | **35** blocks at risk |
| German lessons (schema-v2) | 197 | **212** |
| atoms taught | 367 | **385** |
| atom-measurement-blind lessons | 14 | **12** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 19 | **24** |
| atoms never revisited | 83 | **82** |
| forward references | 33 | **33** |
| corpus narration refusals | 47 | **45** |
| corpus paradigm tables | 95 | **91** |
| corpus full paradigm grids | 21 | **20** |
| corpus lessons with info-dump findings | 121 | **118** |
| book pages | 367 | **383** |

Every corpus-wide info-dump number **fell**: the two four-column day tables are
gone, and the two recaps that replace them are two-column German/English lists
the narrator can speak. All fifteen lessons are `voice`, and both chapters open
hands-free.

### Five defects caught in the draft, before CI

Four banned words (*not just the slot*, *not just to which day*, *you have just
seen*, *the noun you just met*) and one sight cue (*But look at the pair*). The
banned words mattered more than usual: `banned-words.test.ts` had been ratcheted
to 1076 hours earlier, and shipping these would have pushed it straight back over
a ceiling that had only just come down.

### Verification

125/125 test files, 1801 tests; all eleven `check:*` gates; language-ladder 39/39
files, 442 tests; the German book compiles under XeLaTeX with zero errors, zero
overfull or underfull boxes and zero missing characters (383 pages), and both new
chapters were read on the page.

