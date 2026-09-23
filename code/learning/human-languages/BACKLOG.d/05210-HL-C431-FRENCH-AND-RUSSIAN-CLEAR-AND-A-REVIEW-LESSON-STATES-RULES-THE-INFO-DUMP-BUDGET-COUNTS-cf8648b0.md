## HL-C431-cf8648b0 — French and Russian clear, and a review lesson states rules the info-dump budget counts

**Status: CLOSED (2026-09-23) — implemented.** Fourth tranche of HL-C426's
second-pass template.

### What shipped

Six `review` lessons, three per track, no new atoms and no new headwords:

```
french:  reinforcement 14 -> 0
russian: reinforcement 13 -> 0
```

Both needed a **third** lesson for the same structural reason: an atom with no
later revisit at all needs two passes, and in both tracks those atoms sit far
from anywhere a single lesson could reach twice.

- **french** — the three accent marks (`ê ç ï`) are all r=0, taught three pages
  apart in chapter 4. One lesson at sequence 179 gives them a first pass; one at
  1125, thirty chapters on, gives them the second and picks up `ventre`.
- **russian** — `ли` is r=0 at sequence 1140, and the track ends at 1360. Two
  lessons in that last stretch, at 1145 and 1335.

Thirteen of twenty-three tracks now carry no pre-A1 reinforcement debt.

### The French accent lesson is the one worth keeping

The three marks had been taught as one topic called *accents* and are three
unrelated jobs:

| mark | what it changes |
|---|---|
| **ê** circumflex | **nothing you say** — it records an *s* that fell out |
| **ç** cédille | **the sound** — keeps **c** soft before a, o, u |
| **ï** tréma | **the grouping** — read this vowel on its own |

History, sound, grouping. The circumflex is the useful one: French dropped an
*s* and left a hat, English borrowed the older spelling and kept the *s*, so
**forêt/forest**, **hôpital/hospital**, **île/isle**. A hat is a decoder ring,
not a pronunciation instruction — and sorting the three by *what they change* is
what stops them blurring together.

### TWO MORE RULES A REVIEW LESSON TRIPS

Adding to the three in HL-C430. Both fired on the same Russian lesson.

**1. `info-dump` counts rule statements, and one per lesson is the budget.**

```
RU-R02-second-pass-four-letters-and-what-is-left-out
  rule-statement: "the-rule-is: ... your eye already has a rule for those
                   shapes and the rule is"
```

The detector matches phrasings like *the rule is*. A review lesson explaining
**why** something was hard reaches for that construction naturally — and the
corpus ceiling is 32 for the whole 7,500-lesson corpus, so one lesson breaks it.
Rewritten as an observation ("your eye reads those shapes already, and reads
them wrong"), per the precedent every previous entry in that ceiling's comment
set: **soften the phrasing, do not spend the budget.**

**2. "simply" and "just" turn up in a HEADING.** `## Script: two that lie, two
that are simply new`. The banned-word lint reads block prose including headings,
and a review lesson's headings are more discursive than a word lesson's.

### And the French activity ledger is the OPPOSITE of Marwadi's

Worth stating because the two look alike and demand opposite things:

| track | the pin | a review lesson must |
|---|---|---|
| marwadi | EVERY lesson carries exactly ONE activity, all ids listed | **carry** an `hl-activity` |
| french | the complete set of EIGHT ids, `toEqual` | **not carry** one |

The French test is named *"pins French-owned objective activities **without
extending a global ledger**"*. A ninth id breaks it, and extending the pin would
defeat what it is for. A working note written before this tranche had this
backwards and would have shipped a failure; the per-track corpus test settled
it in one read.

### Remaining

gujarati 13 and marathi 14, both of which pin reinforcement-window **positions**
and want that checked as the first step. Then tamil 73, and arabic 78 — which
still wants its own entry, because 68 of its 78 have no later revisit at all and
that is a missing review layer rather than debt.
