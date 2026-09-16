## HL-C381 — ten Devanagari letters are still untaught, and they are what blocks the Hindi future tense

**UPDATED after chapter 96.** Three of the ten are now taught —
`HI-S127-letter-u` (उ), `HI-S128-letter-uu` (ऊ) and `HI-S129-letter-o` (ओ), at
chapter 22, immediately before the first word in the track that needs one. That
took never-taught glyphs **10 → 7** and closure violations **35 → 31**, and it
unblocked the future tense exactly as this entry predicted: `HI-A1-V-11` and
`HI-A1-T-06` both closed in chapter 96. **Seven remain**, and the ordering
advice below still holds for them.

Teaching **थ** for chapter 95 took Hindi's script-closure violations 38 → 35 and
made the past tense writable. The same audit named the rest of the debt. Ten
glyphs are shown somewhere in the track and taught nowhere:

| glyph | lessons showing it | sourced stroke data present |
|---|---|---|
| छ | 16 | yes |  *(still open)*
| ौ | 13 | no — it is a mātrā, and no mark carries a source |  *(still open)*
| उ | 7 | yes — **TAUGHT, ch96** |
| औ | 5 | yes |  *(still open)*
| झ | 4 | yes |
| ढ | 4 | yes |
| इ | 2 | yes |
| ऊ | 2 | yes — **TAUGHT, ch96** |
| ः | 2 | no |
| ओ | 1 | yes — **TAUGHT, ch96** |

**This is a work queue, not a report.** Each row is a letter lesson of about 150
seconds against a template that already exists twenty-six times over, and eight
of the ten have their stroke order sitting sourced and unused in
`data/scripts/devanagari.json`. The two that do not are a vowel sign and the
visarga; the four existing `HI-S1NN-vowel-sign-*` lessons show how a mātrā is
taught without a stroke-order citation.

**What the letters unblock is the point.** Three uncovered inventory points are
straightforwardly about them — `HI-A1-SCR-12` (the remaining independent
vowels), `HI-A1-SCR-14` (the remaining mātrās) and `HI-A1-SCR-15` (the full
consonant series). But the expensive blockage is elsewhere:

- **ऊ blocks the future tense.** `HI-A1-V-11` wants *-ūṅgā / -egā* as a
  productive pattern, and its note says both mock papers depend on it in the
  first scored part. The third-person **आएगा** is writable today; the
  first-person **आऊँगा** and **जाऊँगा** are not, because **ऊ** has never been
  taught. A future-tense chapter can be written around that gap only by teaching
  the form the learner needs least.
- **उ and ढ block the daily routine.** `HI-A1-F-59` says the habitual tense is
  taught but *jānā*, *uṭhnā*, *baje*, *se* and *par* are not. **पर** is now
  taught; **उठना** needs **उ** and **पढ़ना** needs **ढ**.
- **ौ blocks और.** The commonest conjunction in the language appears in five
  lessons and the reader has never been given the mātrā in it.

**Suggested order**, cheapest-unblocking-first: **ऊ** (opens the future), then
**उ** and **ढ** (open the routine), then **ौ** and **छ** (largest existing debt,
16 and 13 lessons), then **औ इ ओ झ**, then **ः** last — it is Sanskritic, twice,
and unblocks nothing at A1.

**The general rule this came from** is recorded in
`lessons.d/one-untaught-letter-can-block-an-entire-tense-and-the-fix-is.md`:
when a track cannot say something, check the alphabet before the curriculum, and
place the letter lesson before the earliest word that needs it rather than
before the chapter that prompted it. Closure is measured in reading order, so a
letter taught at chapter 20 clears every violation after it.

**Placement has a second constraint nobody knew about**, found while landing
chapter 96 and recorded in
`lessons.d/three-letter-lessons-will-sink-a-small-chapter-s-payoff-below-the.md`:
a letter run dropped into a content chapter dilutes that chapter's payoff
coverage. Chapter 20 has five atoms and a payoff assessing three; adding three
letters took it to 3/8 and tripped `payoff-surprise`. Chapter 22 has four atoms
and a payoff assessing all four, so it absorbed the same three at 0.57. **Check
the host chapter's atom count as well as its position** before choosing where a
letter run goes.
