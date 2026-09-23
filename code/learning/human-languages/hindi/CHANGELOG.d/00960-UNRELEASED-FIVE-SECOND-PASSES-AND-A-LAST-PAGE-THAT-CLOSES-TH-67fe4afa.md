## Unreleased — five second passes, and a last page that closes the reinforcement blocker

Five lessons. None of them teaches anything, and none of them needed a new home.

| metric | before → after |
|---|---|
| pre-A1 atoms revisited fewer than twice | 44 → **0** |
| ladder blockers | 2 → **1** (`vocabulary 199/300`) |
| lessons | 506 → 511 |
| atoms taught | unchanged (545) |
| atoms introduced | **0** |
| `atomsNeverRevisited` | 33 → **23** |
| `forwardReferences` | unchanged (22) |
| reinforcement-window misses | 1298 → **1289** |
| new path segments or extensions | **0** |

Reinforcement debt was the last thing standing between Hindi and a pre-A1 claim
that rests on something other than vocabulary count. It is gone; the vocabulary
shortfall is not, and is the whole of what remains.

### Forty-four atoms, fifty-four retrievals

Ten of the forty-four had **no** later revisit at all, and an atom at zero costs
two lessons rather than one: `practisedAtoms` is a set per lesson, so naming an
atom three times on one page still earns it a single revisit. Ten zeros and
thirty-four ones is `(10 x 2) + 34` = fifty-four retrieval slots against
forty-four atoms, and that gap is what bought the fifth lesson.

### Where the five went

| chapter | what it retrieves | atoms |
|---|---|---|
| 85 | the Devanagari pieces, in two passes | 22 |
| 89 | the opening chapters, run cold | 12 |
| 94 | five pockets nobody returned to | 10 |
| 105 | the ten that had been met exactly once | 10 |

Every one was appended to an **existing** path segment whose `spine_node`
already matched its content — three of them `SPINE-EXCHANGE-NAMES`, which is
where a track that opens on names keeps its pre-A1 material. No new chapter, no
new segment, no new extension.

### The script pages sort by kind, not by when

**The first eleven** are eight consonants, the vowel killer **्**, and the two
hangers **ा** and **ी**. The organising fact is that a bare Devanagari consonant
already contains a short *a*: the killer removes it, the hangers replace it.
**अ** earns its own paragraph, because it is the same sound the consonants
already carry — it is what you write when there is no consonant to put it in.

**The second eleven** arrived late and thinly, and reading them together shows
two things no single chapter could. Three of the five consonants — **ष**, **ठ**,
**ण** — are **retroflex**, a whole row made with the tongue curled back that
English does not distinguish and Hindi leans on. And **औ** beside **ौ** is the
pairing the whole system runs on: a vowel takes a full-size letter when nothing
precedes it and a hanging sign when a consonant does. The nuqta closes the page
by saying what a dot under **ज़** is for — the sound is borrowed.

### The three retrieval pages

- **chapter 89 — the opening, eighty chapters later.** The four exchanges that
  open and close a meeting, named and then produced with nothing on the page.
  The claim worth holding is that **आप counts as plural whoever it points at**,
  which is why **आप कैसे हैं?** takes **कैसे** for one person, and that
  **आपका स्वागत है** does two jobs — answering thanks, and greeting an arrival.
- **chapter 94 — five pockets nobody returned to.** A body word whose ancestry
  genuinely stops, a person word in three registers, two Persian household
  words, the two conditions of a form field, and the two named meals. The
  through-line is that a word taught well can still be lost if no later page
  happens to need it.
- **chapter 105 — the last page.** The second pass over the ten that had been
  met exactly once. It sits at the end on purpose: an atom's second outing wants
  the widest gap the book can give it, and for the chapter recaps of the opening
  that gap is the whole book.

### What it cost

Three of the five failed the duration gate on the first draft — 495s, 502s and
358s against a 300-second error threshold — and the declared `max_seconds` had
nothing to do with it. Effective duration is `max(declared, computed)`, and
computed is derived from the prose: words, prompt lines, repeat cues and pauses,
plus 15%. A retrieval page has a practical budget of about **440 words**.

Fourteen headings were rejected outright. `classifyBlock` maps a level-two
heading to a block type by prefix and schema v2 refuses `unknown`, so
`पेट — the trail that stops` is not a legal heading and
`The word, taken apart: पेट` is.

Curriculum digest `98e3031a`/7550 → `998a8193`/7555 on a **26-line** structural
diff.
