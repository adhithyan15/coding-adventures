## Unreleased — the au pair, and a debt a lesson had written down in its own body

**No inventory point closes.** This is script debt, and it is the cheapest
remaining item on the alphabet: two lessons, three closure violations cleared,
and the reported never-taught count cut by nearly half.

| metric | before → after |
|---|---|
| **never-taught glyphs** | **5 → 3** |
| **script-closure violations** | **29 → 26** |
| writing-practice lessons | 107 → 109 |
| atoms taught | 516 → 518 |
| `forward-language` | **unchanged** (22) |
| `payoff-surprise` | **unchanged** |
| `atomsNeverRevisited` | **unchanged** (33) |

### The mātrā differs from one you have by a single stroke

| mark | sound | above the bar |
|---|---|---|
| ◌ो | o | one stroke |
| ◌ौ | au | **two** strokes |

**The only difference is the count**, which makes this the easiest mark in the
script to misread and the easiest to fix: when you meet one, stop and count.

`HI-S134-vowel-sign-au` goes at **chapter 33**, which is before the earliest of
the three lessons it was putting in debt — `HI-C34-padhna`, `HI-C40-where` and
`HI-C76-tenth`. Its examples are **नौ**, **मौसम** and **चौदह**, all taught by
chapter 22.

**नौ** is the word chapter 102 had to route around. Its clock span was drafted
as *nau baje se pāṁc baje tak* — *from nine to five* — and changed to **दस**
because the mark was undrawn. It is drawn now.

### The letter this book owed you

`HI-S135-letter-au` goes at **chapter 34**. It has **six pen lifts**, more than
any other character in the book.

Its real significance is what it pays off. `HI-C68-aur` — the lesson that
teaches **और**, the commonest conjunction in the language — carried a section
titled **"The letters in this word — one is still owed to you"**:

> Its opening vowel is the independent *au*, and **this book has not taught that
> letter**. […] this lesson is deliberately **gloss-first** […] and the debt is
> written down rather than hidden. Learn *aur* now by ear — it is far too useful
> to wait for — **and the letter will come.**

**The letter has come**, and thirty-four chapters *earlier* in the reading order
than the lesson that promised it. So the section is now false, and rewritten:
the heading becomes *"the debt is paid"*, the body tells the reader to **read**
the headword rather than take it by ear, and the roman gloss is named as a
crutch they no longer need.

That is a **fifth variety of stale claim**, and the first found outside the exam
inventory: a *lesson body* that recorded a debt honestly, promised it would be
paid, and had no way to notice when it was. The four earlier varieties were all
notes in `core/exam-inventory-hindi-a1.json`. **Any text that says "not yet"
becomes a liability the moment the work lands**, and the check is the same one
the campaign already uses on inventory notes — grep for the thing you just
taught.

### The payoff that kept a metric flat

A letter lesson's own atom is terminal debt unless something later requires and
practises it. **ौ** is revisited by the independent-vowel lesson one chapter on.
**औ** would have been terminal — `atomsNeverRevisited` went 33 → 34 on the first
measurement.

The fix was not to invent a revisit. `HI-C68-aur` now **requires and practises**
`HI-SCRIPT-RECOG-135`, and its rewritten section assesses it, because reading
**और** is exactly what the letter is for. The metric went back to 33 on its own.

The first attempt put the atom in `HI-C78-pehla-paath`, the connected-reading
chapter, which also uses **और** — and that lesson has no duration headroom, so
even a ten-word clause pushed it from 290s to 303s and failed `validate`. It was
reverted rather than trimmed, because distorting a reading lesson to move a
counter is the thing this campaign keeps declining to do.

### Lessons

| id | headword | chapter |
|---|---|---|
| `HI-S134-vowel-sign-au` | ◌ौ | 33 |
| `HI-S135-letter-au` | औ | 34 |

Remaining after this: **इ** and the visarga by the metric's count, plus the
seven the metric still credits without ever having drawn — **ग घ ट ठ फ**, the
**nuqta** and the **vocalic-r sign**. Their first uses are in chapters 1–13,
where the track has least room, which is `HL-C383`'s open problem.
