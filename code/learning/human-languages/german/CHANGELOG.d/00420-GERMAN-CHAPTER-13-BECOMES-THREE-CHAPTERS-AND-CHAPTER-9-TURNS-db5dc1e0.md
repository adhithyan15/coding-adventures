## German chapter 13 becomes three chapters, and chapter 9 turns out to have already paid for one of them

`ch13-months-seasons.tex` is now generated, as **chapter 13, *The Roman
Calendar***, **chapter 14, *When the Calendar Stopped Naming***, and
**chapter 15, *The Seasons Go Native***. German's hand-written chapters:
**5 -> 4**, and German is still the only track that has any. Old chapters 14-40
renumber to 16-42.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **5** (culture x2, etymology x2, morphologybox x1) |
| `grep -l '^chapter: 13$'` | **2 lessons** |
| the `.tex` | **2 sections**, a **twelve-row** table and a four-row one |
| taught-form census | 22 German forms, **20 of them headwords nowhere** |

**24 atoms against a ceiling of 12**, so three chapters at **8, 6 and 10**, cut
at the seams the `.tex` argues for itself.

### The chapter that was already paid for

Chapter 9's `GE-C06-monate-latein` already teaches `GE-GRAMMAR-NATIVE-VS-LOAN-01`,
already makes the Roman-year claim, and already prints the
*septem/octo/novem/decem* against *sieben/acht/neun/zehn* table. The `.tex`'s
whole "September–Dezember = 7–10" etymology box is therefore **argued upstream**.
Chapter 14 **spends** it rather than restating it, which is why the six numbered
and imperial months cost **six atoms and not eight** — and why splitting at the
named/numbered seam would have produced a four-atom chapter with no argument
left in it. The cut runs Jan–Jun / Jul–Dez instead, and chapter 14's thesis is
the calendar running out of ideas: two men who could order a month renamed did
so, one after the other, and then it is only arithmetic.

To make that spendable at all, `GE-CULTURE-ROMAN-YEAR-BEGAN-IN-MARCH-01` is now
declared in `introduces.knowledge` as well as `introduces_culture_claims`. It was
a culture claim only, and a culture claim cannot be `require`d. The corpus
already had the dual-declaration pattern — `GE-C17-kopf-haupt` declares
`GE-CULTURE-HEAD-CONTAINERS-05` both ways so `GE-C17-hand` can spend it. Chapter
9 goes 6 -> 7 atoms.

### `GE-LEX-JAHR-01` moves back eight chapters, deliberately

Chapter 15 glosses *Jahreszeit* as **year-time**, and glossing a compound with a
word the reader has never been taught is the defect this migration keeps finding.
So *das Jahr* is taught here, where the compound needs it, and old chapter 23 —
now 25 — **spends** it and introduces ***die Jahre*** instead, which is the form
the age sentence actually wanted and which its own gloss already said so. Its
warm-up used to read "a noun you have already met inside a longer word"; that
sentence was true only while *Jahreszeit* came first, and it is gone.

The plural is deliberately **not** previewed in chapter 15. An earlier draft said
"the plural is *die Jahre*" there, which made a forward reference to chapter 25's
new headword — caught by measurement, not by reading.

### Two forward references I wrote and then removed

Drafting produced exactly the defect this programme exists to remove:
`GE-C09-maerz` explained the Roman year by naming *September*, and `GE-C09-juni`
noted that *Juni* and *Juli* sound alike. Both words are taught in the *next*
chapter. Both sentences now make their point without the word, and the corpus
forward-reference count is unchanged at **31** rather than **33**.

### Counters, re-measured against the merged tree

| counter | before | after |
|---|---|---|
| German hand-written chapters | 5 | **4** |
| `handwritten_parity.py` german | 34 | **29** blocks at risk |
| German lessons (schema-v2) | 222 | **249** |
| atoms taught | 396 | **421** |
| atom-measurement-blind lessons | 10 | **8** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 25 | **25** |
| atoms never revisited | 81 | **81** |
| forward references | 31 | **31** |
| corpus info-dump findings | 118 | **118** |

The three new chapters land at **8, 6 and 10** atoms against a German band whose
median is 9 and whose floor is 3, so none of them is an outlier in either
direction.

