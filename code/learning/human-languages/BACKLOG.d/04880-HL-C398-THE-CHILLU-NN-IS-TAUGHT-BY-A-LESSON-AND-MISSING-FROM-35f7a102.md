## HL-C398 — the chillu NN is taught by a lesson and missing from the Malayalam script inventory

Found while writing Malayalam chapter 88, which needed **എൺപത്** (eighty) and
could not print it.

### The gap

`ML-S131-chillu-nn` teaches **ൺ** (U+0D7A) at sequence **144** — very early, long
before any chapter that would use it. But `data/scripts/malayalam.json` does not
list it:

| chillu | in the inventory |
|---|---|
| **ൽ** U+0D7D | yes |
| **ൻ** U+0D7B | yes |
| **ൾ** U+0D7E | yes |
| **ർ** U+0D7C | yes |
| **ൺ** U+0D7A | **no** |

So the validator raises `uncovered-glyphs` for **any content lesson that writes
the letter**, and `tests/integration.test.ts` turns the corpus-wide glyph-gap
queue into a hard failure. The learner has met the letter; the data file has not.

### Why chapter 88 did not simply fix it

Each of the four listed chillus is pinned by `tests/script-inventories/malayalam.evidence.ts`
to a **specific sourced stroke-order animation** — a named Wikimedia file, a
frame count, and start and end timings, matched by regex:

> `/Sriveenkat.*Ml ൾ order\.gif.*chillu LL.*00:03\.0.?00:09\.3.*Wikimedia Commons.*14 July 2023/i`

Adding **ൺ** to the same standard means producing that evidence for it. **A
citation cannot be guessed**, and the inventory file says so itself: *"Stroke
order is a separate, source-gated effort: only source-verified entries carry
it"*, and *"When `penLifts` is absent it means NOT VERIFIED — never none."*

This is also script-owner territory by design. `tests/script-inventories/README.md`
and HL24 exist precisely so that *"unrelated script authors must not share an
executable edit surface"*, and a Malayalam curriculum chapter is an unrelated
author for this purpose.

### What chapter 88 did instead

**Taught eighty by ear.** The tens from thirty to seventy are given in script;
**eṇpathŭ** is given in romanization only, with the lesson saying that its
written shape comes when its letter does.

That is not a workaround invented for the occasion — it is **this track's own
precedent**. `ML-C07-numbers-6-10`'s gloss reads *"hear and say six to ten before
meeting their written forms"*, and its headword is romanization.

### The fix

Source a stroke-order animation for **ൺ** to the standard the other four meet,
add the `finalConsonants` entry, and extend `malayalam.evidence.ts` with the
matching assertion. Then chapter 88's tens table can carry **എൺപത്** in script,
and the romanization-only row and its explanatory sentence come out.

Worth checking at the same time whether any other taught letter is missing from
its script's inventory. The failure mode is silent until some lesson happens to
need the glyph, which is how this one surfaced — **eighty is simply the first
word the curriculum has wanted that contains it.**

### Second occurrence: chapter 110, the telephone

**ഫോൺ** hit the same wall, and the cost is no longer hypothetical. Closing
`ML-A1-LEX-38` — five Spanish points, the largest single payoff left in that
inventory — meant naming the telephone, and the word cannot be printed for the
same reason eighty could not.

The draft that discovered it had gone further and bought a **new letter** for the
job: `ML-S148-letter-pha` taught **ഫ**, which appears in no headword anywhere in
the corpus. Validate then failed on the *chillu*, not on **ഫ** — and once the
word could not be printed, a letter bought to write it had no purpose, so that
lesson was deleted rather than kept for appearance.

So the tally for this one missing inventory entry is now: one number taught by
ear, one everyday noun taught by ear, and one script lesson written and thrown
away. **Two words in, the pattern is clear — this will keep costing until the
citation is sourced**, and each occurrence costs more than the last, because the
words that need it are getting more ordinary.
