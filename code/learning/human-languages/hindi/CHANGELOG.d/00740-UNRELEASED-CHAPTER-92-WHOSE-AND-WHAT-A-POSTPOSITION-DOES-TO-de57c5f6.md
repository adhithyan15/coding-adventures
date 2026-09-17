## Unreleased — chapter 92: whose, and what a postposition does to it

Hindi A1 exam coverage **199/282 (71%) → 203/282 (72%)**. Four points close in
one chapter, and they close together because they are **one system rather than
four facts**.

| point | what was missing |
|---|---|
| `HI-A1-DET-06` | *uska*, *hamara*, *tumhara* |
| `HI-A1-DET-07` | the reflexive possessive *apna* |
| `HI-A1-V-09` | *mere paas* — Hindi has no verb for *have* |
| `HI-A1-PRON-07` | a pronoun in front of a postposition |

### The chapter turns on a correction, not a new word

Chapter 2 taught **मेरा / मेरी / मेरे** and called *mere* the plural. That is
true and incomplete. *Mere* is **also the oblique** — the form a word takes in
front of a postposition — and the two are spelled identically:

| phrase | which job |
|---|---|
| *mere dost* | plural — my friends |
| *mere pās* | oblique — near me |

Nothing in the spelling separates them. **What comes after does.** Once that
bend is automatic, all four points follow from it, which is why they are one
chapter and not four.

### Hindi has no word for "have", and the note said so bluntly

`HI-A1-V-09`'s note is the sharpest in the inventory: *"Hindi has no verb 'to
have'; possession is `at me there is`… so a learner cannot say they have
anything."*

> मेरे पास एक किताब है।
> *mere pās ek kitāb hai* — "near me there is a book" = **I have a book.**

The lesson makes the agreement explicit rather than leaving it to be inferred:
the **thing owned is the subject**, so *hai* agrees with *kitāb* and not with
the possessor.

### apna is not optional, and both mock papers use it

`HI-A1-DET-07`'s note records that **both** practice papers' personal accounts
reach for it — *apne dost ke sāth*, *apne parivār ke sāth* — while nothing in
the corpus introduced it.

When the owner is the **subject of the same sentence**, Hindi replaces the
ordinary possessive. *Maiṅ merā kām kartā hūṅ* is wrong; *maiṅ apnā kām kartā
hūṅ* is right. One Hindi word covers *my own*, *your own*, *their own* and *our
own* — and it removes an ambiguity English cannot express without extra words:
*uskī kitāb* is somebody else's book, *apnī kitāb* is his own.

### Agreement runs the opposite way from English

The possessive agrees with the **thing owned**, never with the owner:

| | what decides the form |
|---|---|
| English *his* / *her* | the owner's gender |
| Hindi *uskā* / *uskī* | the owned thing's gender |

So *uskī kitāb* tells you the book is feminine and tells you **nothing** about
whose it is. This is not a new rule — it is the **-आ** agreement the track
already has, doing its ordinary job.

### Terminal-lesson debt, budgeted

Six lessons introduce one atom each; two retrieve. Measured against a clean-main
equivalent: `attained` unchanged, reinforcement shortfall **flat at 44**, pre-A1
headwords **flat at 191**.

### Script closure: zero new debt, caught before generation

Three drafts carried Devanagari the corpus shows but never teaches — **उ** (in
*uskā*), **थ** (in *sāth*), and **औ** and **छ** in a reading passage. Each was
romanized out of body prose, and the reading was rewritten to keep its teaching
point (three owners, one bend) using only taught script.

**No exemption list was edited and no ceiling was touched.** Those glyphs remain
pre-existing debt owned by the lessons that first showed them.

The table-width, banned-word, chapter-reference and info-dump checks all ran
**before** generation this time, and none of the four fired.

### Pins

| pin | before → after |
|---|---|
| Hindi lesson count | 380 → 388 |
| `coverage.covered` | 199 → 203 |
| `coverage.unmapped` | 83 → 79 |
| report string | 199/282 → 203/282 |

### Two counts went up, and both stay up

`measurement-blind` **5 → 6** — the one new *síntesis* lesson. **HL-C377** for
the fifth time; the type was again not relabelled to clear it.

`reinforcementWindowMisses` **996 → 1012**, which is not the level-gate
shortfall (flat at 44) but the consequence of the track getting eight lessons
longer.

### Verification

`human-language-data` 145 files / 2083 tests; `language-ladder` `bash BUILD`
39 files / 442 tests; all twelve `check:*` gates; `check-book-compile.sh hindi`
under XeLaTeX.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
