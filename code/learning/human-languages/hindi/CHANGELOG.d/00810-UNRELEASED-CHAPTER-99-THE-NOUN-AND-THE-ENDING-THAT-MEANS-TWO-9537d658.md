## Unreleased — chapter 99: the noun, and the ending that means two different things

Hindi A1 exam coverage **217/282 (77%) → 221/282 (78%)**, closing four points
on **zero new nouns**.

| point | what was missing |
|---|---|
| `HI-A1-N-07` | plural morphology |
| `HI-A1-N-08` | the oblique form a noun takes before a postposition |
| `HI-A1-POST-06` | *ko* marking a recipient or a definite object |
| `HI-A1-DET-03` | a demonstrative in front of its noun |

### Four points, and not one new word

Every example noun in the chapter — **बच्चा**, **किताब**, **भाषा**, **आदमी** —
was already taught. The chapter is **pure morphology on known vocabulary**,
which is the cheapest shape a tranche can have and the one worth looking for
before authoring anything.

### The plural system is four rows and two questions

| noun | plural |
|---|---|
| बच्चा (m, -आ) | बच्चे |
| आदमी (m, other) | आदमी |
| किताब (f, consonant) | किताबें |
| भाषा (f, -आ) | भाषाएँ |

**Gender first, ending second.** The two feminine rows do the same thing despite
ending differently, which is why asking about the ending alone gets *bhāṣā*
wrong.

One row does **nothing at all**, and that is the row a reading paper will test:
*do ādmī* is two men with no mark on the noun, so **the number lives in the
numeral, the verb and any adjective** rather than in the word.

### The oblique is not a new system

`HI-A1-N-08`'s note says the corpus teaches *me* and the *kā* genitive without
the case change the noun undergoes. True — but the **bend itself** has been
taught four times already:

| kind of word | plain | in front of a postposition |
|---|---|---|
| possessive | मेरा | मेरे पास |
| possessive | उसका | उसके बाद |
| **noun** | **बच्चा** | **बच्चे को** |

A noun is the **fourth and last kind of word** to wear the oblique **-ए**. So
the lesson does not introduce a rule; it finishes one.

### The load-bearing consequence

**-ए now means two different things**, and the chapter is built around telling
them apart:

| what you see | how many | why |
|---|---|---|
| बच्चे हैं | more than one | nothing follows the noun |
| बच्चे को | **one** | a postposition follows |

**The postposition is the whole signal.** A reader who misses it miscounts every
sentence that has one, and **-ए** is everywhere in this language.

### को was inside a word they already say

`HI-A1-POST-06`'s note is exactly right that only the fused *mujhe* was present.
The lesson recovers the postposition by taking **आपको** apart in front of the
reader — *āp* plus *ko* — rather than presenting it as new, and teaches both
jobs the point names. The note's own observation rides into the lesson: this is
the counterpart of the proxy's **a** before a human direct object, so the demand
is not unique to Hindi and only the marker is.

### A fifth partly-stale note

`HI-A1-DET-03` said the deixis chapter *"never puts either in front of a noun."*
It does: `HI-C69-verb-last` prints **यह किताब है** and `HI-C78-pehla-paath`
prints **वह किताब पढ़ता है**. Neither is an atom, so the point was genuinely
uncovered — but the precise complaint is that the shape was **shown and never
taught**, which is a different defect from absence and the one the corrected note
now records.

The lesson adds why it matters more in Hindi than the English parallel suggests.
**Hindi has no article**, so **यह** and **वह** are what a writer reaches for
where English would have used *the* — and **वह आदमी** is often best read as *the
man* rather than *that man*.

### forward-language 18 → 21, and the diagnosis took thirty seconds

Four new entries on the first measurement, and last chapter's `lessons.d` note
is what sorted them:

| entry | verdict |
|---|---|
| `HI-C69-verb-last` shows **यह किताब** | **real** — the DET-03 finding, now visible |
| `HI-C86-aapko-use` shows **को** | **real** — the POST-06 finding |
| `HI-C86-repaso-sarvanam` shows **को** | **real**, same cause |
| `HI-C91-ko` shows **बच्चे को** | **mine**, and avoidable |

The fourth was a self-inflicted ordering slip: my own *ko* lesson used *bacce ko*
as its example **one lesson before** the oblique lesson introduces that phrase.
Changed to *ādmī ko*, which the lesson already used. Three genuine, one fixed.

That is the procedure working as written: group by `taughtBy`, then ask whether
anything earlier already introduces the word. Three of these expose debt the
chapter is precisely about; one was a mistake.

### Numbers

| metric | before → after |
|---|---|
| atoms taught | 484 → 491 |
| measurable lessons | 424 → 432 |
| `forward-language` | 18 → 21 |
| `measurement-blind` | 12 → 13 |
| `payoff-surprise` | **unchanged** |
| script-closure violations | **unchanged** |
| atoms never revisited | **unchanged** |

One thing the chapter had to route around: **अच्छा** contains **छ**, which is
still untaught, so the agreement examples use **लंबा** instead. **छ is now the
largest single glyph debt in the track — sixteen lessons** — and it has sourced
stroke data sitting unused. It is the next alphabet item.

Reinforcement misses rose 1126 → 1143, in step with nine new lessons.

### Verification

`human-language-data` full suite; all twelve `check:*` gates;
`check-book-compile.sh hindi` under XeLaTeX.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
