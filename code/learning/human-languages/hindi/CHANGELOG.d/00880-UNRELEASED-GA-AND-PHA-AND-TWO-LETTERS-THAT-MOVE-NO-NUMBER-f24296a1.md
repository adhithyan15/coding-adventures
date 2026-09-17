## Unreleased — ga and pha, and two letters that move no number

**No inventory point closes, and no reported metric changes.** That second part
is the point of this entry.

| metric | before → after |
|---|---|
| atoms taught | 520 → 522 |
| measurable lessons | 465 → 467 |
| writing-practice lessons | 111 → 113 |
| **never-taught glyphs** | **2 → 2** |
| **script-closure violations** | **24 → 24** |
| `forward-language` | unchanged (22) |
| `payoff-surprise` | unchanged |
| `atomsNeverRevisited` | unchanged (33) |

Two characters that nobody had ever drawn are now drawn, and **the two metrics
that are supposed to track exactly that sat still**. `measureScriptClosure` had
already credited both — **ग** to whichever early lesson printed *svāgat*, **फ**
to whichever printed *phir* — so by its reckoning there was nothing to fix and
nothing got fixed.

`HL-C383` recorded that defect. This is the first change that demonstrates it
from the other side: not a number that was wrong, but a real improvement the
number cannot see. **By the honest count, undrawn goes 7 → 5.**

### Late, and the lessons say so

| lesson | glyph | chapter | first used | gap |
|---|---|---|---|---|
| `HI-S138-letter-ga` | ग | 25 | ch3, *svāgat* | **22 chapters** |
| `HI-S139-letter-pha` | फ | 26 | ch4, *phir* | **22 chapters** |

These are the first of the eight that `HL-C384` showed **can never arrive on
time**, because the `HI-S*` letter run does not begin until chapter 6 and all
eight are used before it.

`HI-S138` states the reason rather than glossing over it: the opening teaches
by **copying whole words** — a real sentence in two chapters, which is worth a
great deal — and the cost is that the letters inside those words went unnamed.
The reader has been recognising **ग** by its company. Now they get the letter.

### फ is the fourth breath pair

| plain | breathy |
|---|---|
| त | थ |
| च | छ |
| ज | झ |
| **प** | **फ** |

Four pairs now, and the distinction was learned once — each pair since has cost
only a shape.

### Both illustrate themselves from words already taught

`before.mjs` gave **स्वागत**, **मिलेंगे** and **मंगलवार** for ग, and **फिर**
for फ, all taught long before the host chapters. Unlike chapter 103, neither
lesson reached forward for a more convenient example, and `forward-language`
did not move.

**फ's examples avoid the nuqta forms deliberately.** *māf*, *safed* and
*farvarī* all carry the dot, and the dot is not taught; **फिर** is plain. An
early draft of the gloss called फ "the last carrier the dot will need", which
is both a forward reference and a "not yet" promise — the liability that has
gone stale five times in this campaign. Cut before commit.

### Why these two, and why now

They are the **nuqta's missing carriers**. The nuqta is `HI-A1-SCR-16`, the
sharpest finding in the exam inventory, and it attaches to seven consonants —
of which **ग** and **फ** had never been drawn and **ड** is drawn only at
chapter 59. A complete nuqta lesson needs all seven, so it must follow chapter
59 and it needed these two first.

That is the next item.

### Payoffs

A letter's own atom is terminal debt unless a later lesson requires, practises
and assesses it:

| glyph | payoff |
|---|---|
| ग | `HI-C54-cow` — **गाय** |
| फ | `HI-C45-fruit` — **फल** |

Both chosen for a headword the letter actually opens, both with duration
headroom checked first. `atomsNeverRevisited` held flat on the first
measurement, as it did for the previous pair.
