## Chapter 88 — counting past twenty

`ML-A1-NUM-04` closes. Coverage **192/243 → 193/243 (79%)**.

The note was plain: *"The count stops at twenty. Prices, ages over twenty and
years are all out of reach."*

### The pattern was already on the page

**ഇരുപത്** had been explained since the numbers chapter as *iru* + *pathŭ*,
**two-tens**. Every ten from thirty up is that same word with a different digit
on the front:

| | | built on |
|---|---|---|
| **മുപ്പത്** | 30 | three |
| **നാൽപത്** | 40 | four |
| **അമ്പത്** | 50 | five |
| **അറുപത്** | 60 | six |
| **എഴുപത്** | 70 | seven |

**-പത്-** is in all of them.

But the digits appear in **older short forms that survive only in compounds** —
*āṟŭ* shows up as **അറു-**, *ēḻŭ* as **എഴു-**, and **അമ്പത് has worn furthest of
all**, its link to *five* real and no longer visible. So the lesson says what
kind of pattern this is: **one to read with, not to build with.**

### Ninety breaks the run, so a hundred comes first

**തൊണ്ണൂറ്** has no **-പത്-** in it. What it has is **നൂറ്** — a hundred —
sitting at its end.

| | built from |
|---|---|
| *eṇpathŭ* 80 | eight **tens** |
| **തൊണ്ണൂറ്** 90 | a **hundred**, reduced |
| **നൂറ്** 100 | itself |

**Ninety is named from above**, where the other tens are named from below. That
is why the lesson teaches a hundred **first**: learn *toṇṇūṟŭ* after *nūṟŭ* and
it is one irregular word with a visible reason; learn it before, and it is noise.

### The compound rule is a stem change the learner has met

**ഇരുപത്തിയൊന്ന്** — twenty-one. **The ten does not keep its standing-alone
shape**; it takes **-ത്തി-** and the digit follows, in the plain order English
also uses.

That is the third thing in this book to do it: **കേരളം** became **കേരളത്തി**ൽ
before its ending. **A word wears one shape alone and another before something
else** — worth expecting from here rather than learning case by case.

### Eighty is taught by ear, and why

**എൺപത്** needs the chillu **ൺ**. `ML-S131-chillu-nn` teaches that letter at
sequence 144 — but **`data/scripts/malayalam.json` does not list it**, so any
lesson that writes it trips the `uncovered-glyphs` gate. Eighty is simply the
first word the curriculum has wanted that contains it.

Adding the letter properly means **a sourced stroke-order citation** — the other
four chillus are each pinned in `malayalam.evidence.ts` to a named animation with
frame timings — and **a citation cannot be guessed**. It is also script-owner
territory by design.

So eighty is given **in romanization only**, with the lesson saying its written
shape comes when its letter does. **That is this track's own precedent**:
`ML-C07-numbers-6-10` reads *"hear and say six to ten before meeting their
written forms."*

Recorded as backlog item **`HL-C398`**, with the fix and a note to check whether
any other taught letter is missing from its script's inventory — **the failure is
silent until some lesson happens to need the glyph.**

### `ML-A1-NUM-08` updated

Its numeric blocker is gone: a learner can now count to a hundred. What remains
is genuinely the **units** — no measure of length, no measure of weight — which
is a vocabulary chapter on its own rather than something waiting on the numerals.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 192/243 → **193/243 (79%)** |
| the numerals column | 7/9 → **8/9** |
| `forwardReferences` | unchanged |

