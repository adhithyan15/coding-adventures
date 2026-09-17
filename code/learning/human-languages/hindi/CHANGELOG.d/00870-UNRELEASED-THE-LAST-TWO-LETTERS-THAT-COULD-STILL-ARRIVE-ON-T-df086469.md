## Unreleased — the last two letters that could still arrive on time

**No inventory point closes.** Script only — and this is the end of a
possibility rather than the end of a queue.

| metric | before → after |
|---|---|
| **never-taught glyphs** | **3 → 2** |
| **script-closure violations** | **26 → 24** |
| writing-practice lessons | 109 → 111 |
| atoms taught | 518 → 520 |
| `forward-language` | **unchanged** (22) |
| `payoff-surprise` | **unchanged** |
| `atomsNeverRevisited` | **unchanged** (33) |

### Why these two and no others

`HL-C384` established the shape of the problem. The Hindi script track has two
runs: chapters 1–2 teach writing by **copying whole words**, and the `HI-S*`
letter run — where the alphabet is actually walked — does not begin until
**chapter 6**. So every letter inside *namaste*, *khushī*, *ṭhīk*, *svāgat*,
*phir* and *kṛpayā* reached the page before any lesson existed that could draw
it.

**घ and ढ are the only two left whose first word comes after chapter 6.** Both
are first used at sequence 630; chapters 14, 15 and 16 each carry only one
script lesson where the pattern allows two, so the second slot was free and two
of those slots fall before 630.

| lesson | glyph | chapter | seq | pen lifts |
|---|---|---|---|---|
| `HI-S136-letter-gha` | घ | 14 | 612 | 2 |
| `HI-S137-letter-dha-retroflex` | ढ | 15 | 622 | **1** |

**ढ has the shortest build in the book.** Everything below the headline is a
single unbroken run — stem, shoulder, outer bowl and closed inner loop without
the pen leaving the paper — and then the shirorekhā goes on top. Nothing else
is drawn in two strokes.

After these, **every remaining character is one the reader has already been
seeing without being shown**, and each of those lessons will have to say so.
Both lessons state this plainly rather than pretending the order was always
intended: `HI-S136` says only two characters are still ahead of their first
word, and `HI-S137` says it is the last.

### A correction found before it cost anything

The obvious payoff for both looked like `HI-C16-mahine` — the months lesson at
sequence 630 that the closure measure names.

**It is the wrong target.** घ and ढ appear there only inside the twelve
Hindu-calendar month names printed as a cultural aside — *āṣāḍh* carries ढ,
*māgh* carries घ. The lesson's own headword is the Gregorian months, which
contain neither. It is a closure violation caused by a footnote, not a place a
learner reads those letters.

The load-bearing first uses are elsewhere, and those are where the atoms went:

| glyph | payoff | why |
|---|---|---|
| घ | `HI-C18-ghanta` — **घंटा** | the first word the reader actually produces with it |
| ढ | `HI-C34-padhna` — **पढ़ना** | likewise, and that lesson was **itself** in closure debt for ढ |

A letter lesson's own atom is terminal debt unless a later lesson requires,
practises and assesses it. Wiring both into the lessons that teach their words
kept `atomsNeverRevisited` flat on the first measurement — no second attempt
needed, unlike the au pair.

### Neither letter names an example word

`before.mjs` found **no** घ word taught before chapter 14 and **no** ढ word
before chapter 15 — which is precisely what "arriving before its words" means.
So neither lesson illustrates itself, and in particular neither reaches forward
to **घंटा** or **पढ़ना**.

That restraint is the lesson of chapter 103, where three of four new
`forward-language` entries came from a letter lesson grabbing the most natural
example without checking when it arrives. This time the metric did not move.

### What is left

By the metric: **इ** and the **visarga**. Honestly also **ग**, **ञ**, **ट**,
**ठ**, **फ**, the **nuqta** and the **vocalic-r sign** — seven credited as
taught that nobody has drawn, every one of them first used in chapters 1–5.

They can only be taught **late**, the way `HI-S132-letter-ja` was — first used
in chapter 2, drawn in chapter 30, with the lesson saying so. The **nuqta**
should go first: it is `HI-A1-SCR-16`, the sharpest finding in the exam
inventory, and the corpus's own headwords are full of *shukriyā*, *zarūr*,
*darvāzā*, *mez*, *sabzī* and *safed*.
