## Chapter 71 — the people in the sentence

`ML-A1-PRON-03` and `ML-A1-PRON-04` close. Malayalam A1 coverage
**164/243 → 166/243 (68%)**.

| metric | before → after |
|---|---|
| exam-point coverage | 164/243 → **166/243 (68%)** |
| atoms taught | 379 → 382 |
| measurable lessons | 302 → 306 |
| `forwardReferences` | unchanged |
| `scriptClosureViolations` | unchanged |
| `durationViolations` | unchanged |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged |
| reinforcement-window misses | 777 → 791 |

### Seventy chapters with nobody in them but the two people in the room

**അവൻ, അവൾ, അവർ and ഞങ്ങൾ appeared in zero lesson files** — not as headwords,
not anywhere in a body. A learner could say *I* and *you* and could not say
*he*, *she*, *they* or *we*.

### നാം looked taught and was not

It appeared as a headword in three lessons — `ML-C67-first`, `ML-C67-third` and
`ML-C68-eleventh` — where it is a **substring** of the ordinals **ഒന്നാം**,
**മൂന്നാം** and **പതിനൊന്നാം**. Every ordinal ending in **-ന്നാം** is a false
positive for the pronoun.

That is the same trap the Hindi campaign recorded when every ordinal matched its
own cardinal. **Check the token, not the substring.**

### The gap sat inside a system already taught

`ML-C41-that` teaches the **ഇ-/അ-** pointing pair and says in as many words that
**അ-** means far — and the corpus only ever used it on **things**. അവൻ, അവൾ and
അവർ carry that same **അ-**, so the column for *people* was predicted by a rule
the reader already had and never filled.

That is the third chapter in a row to find a gap of exactly this shape, after
Tamil's `TA-A1-PRON-03` and Kannada's punctuation. **The pattern gets taught,
the pattern is correct, and half of what it predicts never arrives.**

### Two of the three new words cost nothing new

- **അവർ** reuses `ML-C02`'s own rule that **a plural raises the register** —
  the move that turned നീ into നിങ്ങൾ in chapter two, turning അവൻ into അവർ here.
  Learning it once buys it twice.
- **അവൻ / അവൾ** split on **-ൻ** against **-ൾ**, endings that recur elsewhere.

Only the two we-words are genuinely new, and the question behind them has no
English equivalent: **ഞങ്ങൾ** leaves the listener out, **നാം** takes them in, and
Malayalam makes you choose every time. Tamil draws the same line with nearly the
same sounds, which this track's comparative voice says out loud.

77 exam points remain open. Coverage means the teaching exists, not that a
reader scores.


