## Unreleased — the A1 exam inventory: 167 of 258, and no conjunction anywhere

`core/exam-inventory-kannada-a1.json` enumerates **258** A1 points and measures
the corpus at **167 covered, 65%** — the second-highest of the nine inventories
now written, behind nothing and level with Telugu. Kannada becomes the ninth
track that can be *measured* at all; fifteen still cannot.

It is not only a gate. `npm run plan` can already say a track needs vocabulary;
it cannot say WHICH words. The 91 `probe: null` points name them, and are
written as a work queue for that reason.

### Derived from the Spanish A1 set, and accounted for in both directions

The point set is derived structurally from `core/exam-inventory-es-a1.json`, the
DELE/PCIC-sourced Spanish A1 inventory. **Spanish is a proxy for LEVEL, not for
grammar**: each Spanish point was read as "what must an A1 learner handle here",
and the Kannada exponent carrying the same load was then chosen. Where Kannada
carries it with machinery Spanish has no name for — a case suffix, a dative
subject, a zero copula — the point is restated around the Kannada machinery and
records its origin.

All **273** Spanish points are attributed: 271 derive into some Kannada point,
and exactly 2 are dropped with a reason (the *al/del* contractions, and the
exclamative *qué*). Four Kannada points have no Spanish question behind them at
all and are flagged `kannadaSpecific` — the human/non-human noun division, the
one-suffix-per-job property of an agglutinating language, the p→h sound law, and
the spoken/literary divide.

An earlier draft had **fifteen** Spanish points both derived *and* dropped,
because restating a point around Kannada machinery feels like not transferring
it. Restating is deriving. `notTransferred` now holds only points that produce
no Kannada point at all, and a test enforces the disjointness.

### The joining column: 2 of 11, and neither is a conjunction

`mattu` (and), `athava` (or), `aadare` (but), `eekendare` (because) and the
quotative `anta`/`endu` occur **zero** times in 268 lessons and zero times in the
generated book. The two covered points are the `-i` participle, which is
Kannada's real clause-chaining machinery and is properly taught in chapter 4,
and chapter 64 — a chapter *named* `JOIN` whose five words all connect turns
rather than clauses. Three function points and one taught verb are idle for want
of words that are each one lesson long.

### The script column is the opposite of Tamil's

Measured, not assumed: **42 characters taught against 69 used in headwords, 61%
closure**. Tamil's inventory reported 52 of 52 taught; an inventory that carried
that expectation over would have written false notes here. `ma` appears in 40
headwords and is never taught.

### Verified

Every atom named in every probe was checked to exist **against the merged tree** —
37 of them did not exist until Kannada chapters 1, 2 and 4 were retired into the
generator earlier the same day, so validating against the working branch would
have been validating against a tree nobody else had. One invented id
(`KA-ETYMON-C38-HRUDAYA-01`, which is `KA-LEX-…`) was caught by that check and
fixed.

The corpus-wide exam-point backlog moves **792 → 883 across 9 inventories**, and
the unmeasurable remainder falls **16 → 15 tracks**. Re-measured with `npm run
plan` against the merged tree, never composed: this branch read 793 before the
chapter retirement landed, and the retirement moved the number on its own.

human-language-data 124 test files / 1745 tests (7 new), all eleven `check:*`
gates, language-ladder 39 files / 442 tests.

