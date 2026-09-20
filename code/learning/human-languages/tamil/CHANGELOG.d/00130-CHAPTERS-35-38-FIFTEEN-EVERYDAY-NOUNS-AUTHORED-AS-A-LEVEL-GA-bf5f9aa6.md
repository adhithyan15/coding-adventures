## Chapters 35–38 — fifteen everyday nouns, authored as a level-gate probe

This tranche exists to answer a measurement question as much as a teaching one:
**does authoring track-local pre-A1 vocabulary actually move
`levelGate.tracks[tamil]`?** It does, and the size of the move is the finding.

### What the gate did

| figure | before | after |
|---|---|---|
| distinct headwords at or below pre-A1 | 33 | **48** |
| `vocabulary` shortfall against the 300 target | 267 | **252** |
| pre-A1 atoms revisited fewer than twice | 29 | **0** |
| track-wide distinct headwords | 67 | **82** |
| atoms never revisited (of atoms taught) | 48 of 126 | **37 of 156** |

Fifteen new lessons moved the pre-A1 vocabulary count by **exactly fifteen**,
because `vocabularyOf` counts **distinct headword strings, one per lesson** —
not the words a lesson teaches. `TA-C12-kudumbam` teaches six kinship terms and
counts as one; `TA-C15-thanneer-arisi` teaches three and counts as one. Closing
a 267-headword gap therefore means roughly **267 more lessons at pre-A1**, which
is the same arithmetic HL09 §3 already published (~150 lessons for the first 300
words, at ~2 words per lesson) arriving from the other direction.

The **reinforcement** criterion behaved very differently: it went from 29 to
**zero**, because a payoff can rescue many atoms at once. Reinforcement debt is
cheap to clear by authoring; vocabulary debt is not.

The `atom-budget` blocker is untouched at 1 — `TA-W01-abugida-va-ka` introduces
four atoms against a budget of three, and predates this work.

### The chapters

| chapter | node | lessons |
|---|---|---|
| 35 — Arriving at a House | `SPINE-MEET-GREET` | வீடு, கதவு, அறை, நாற்காலி |
| 36 — What You Are Offered | `SPINE-POLITE-REQUEST-REPAIR` | தேநீர், பால், உணவு, தவறு |
| 37 — Your Town, Your Friend | `SPINE-EXCHANGE-NAMES` | ஊர், நண்பன், இவர், இவர் என் நண்பர் |
| 38 — Keeping Well, and Leaving | `SPINE-CHECK-WELLBEING`, `SPINE-TAKE-LEAVE` | உடம்பு, சுகம், விடை |

Sequences 750–890, all schema v2, two new atoms each — 8, 8, 8 and 6 per
chapter, all inside the 12-atom chapter budget. Attached to pre-A1 spine nodes
through five new `TA-EXT-03*-LANGUAGE-SPECIFIC` extensions on path segments
`TA-PATH-031`–`035`, which is what makes them count at pre-A1 at all.

### Rescue, at two cadences

Every lesson practises the preceding one to three lessons (R1), and each chapter
payoff reaches several chapters back. Between them the tranche cleared **all 29**
pre-A1 atoms that were revisited fewer than twice — the eight Chapter-1 writing
atoms (**வ**, **க**, **ண**, **ற**, the puḷḷi, the no-conjunct rule, the **ி**
sign, nasal-triggered voicing), the dative pair from Chapter 6, the register
atoms from Chapters 8–9 and 19, the kinship atoms from Chapter 12, the body
parts from Chapter 13, and the water/rice atoms from Chapter 15. Each rescue is
a real revisit: **நாற்காலி** genuinely uses **நான்கு**, **தவறு** genuinely
re-reads **மன்னிக்கவும்**, **உடம்பு** genuinely recalls **தலை** and **கை**.

### What the words are, and what they are not

Several of the obvious "first nouns" were **already taught** and are not
repeated here: தண்ணீர் and சாதம் (Chapter 15), பெயர் (Chapter 2), and அப்பா,
அம்மா, அண்ணன், தம்பி, அக்கா, தங்கை (Chapter 12 — Tamil's age-graded kinship is
already in the course). Chapter 37 builds on that rather than restating it:
**நண்பன் / நண்பி / நண்பர்** splits by sex and respect where the sibling words
split by age, and the payoff names the contrast.

The etymology was checked against the Dravidian Etymological Dictionary, the
Madras Tamil Lexicon and Wiktionary before authoring, and four claims changed as
a result:

- **வீடு** "house" and **வீடு** "release" are **one DEDR entry**, headed by
  **விடு** "to let go" — and **விடை**, Chapter 38's payoff, is in that entry
  too. The chapter arc walks in through the house and out through the leave, on
  one root, because the dictionary says so.
- **தேநீர்**'s **தே** came through **Malay *teh***, not straight from Chinese;
  and **Portuguese *chá*** is the *exception* to the maritime-*te* pattern
  (Macau), not an example of the overland *cha* one.
- **கதவு** has **no Telugu cognate** in DEDR, and **அறை** has **no Kannada**
  one. Neither is claimed.
- **சாப்பிடு** is *not* safely *sāppu* "food" + **இடு**, as this course said in
  Chapter 32: no Tamil lexicon records a plain *sāppu* meaning "food," and the
  Dravidian dictionary has **no entry for the verb at all**. `TA-C36-unavu`
  says so plainly rather than repeating the earlier account.

One genuine English link is claimed and it is a new one: **mulligatawny** is
**மிளகுத்தண்ணீர்**, "pepper water," carrying the **தண்ணீர்** taught in
Chapter 15. *Catamaran*, *curry*, *mango* and *pariah* are Tamil's other gifts
to English and none of them comes from a root in this tranche, so none is
claimed.

### Book, narration, drivability

Chapters 35–38 are generated to `book/chapters/`, added to `book.tex`, and the
178-page book compiles with XeLaTeX with **zero** `Missing character` warnings.
All fifteen lessons derive `coreModality: voice` — every one is drivable once
the detachable *"The letters in this word"* section is set aside — and every
table is two or three columns, so the narration exporter linearises all of them
rather than refusing any.

