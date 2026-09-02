## HL-C287 — Marathi teaches the copula and the gendered present twice; who owns the atoms?

Retiring Marathi's four hand-written chapters surfaced a duplication that had
been invisible because the early chapters declared no atoms at all. Two things
are taught in full, twice, in chapters the reader meets nine and five chapters
apart:

| taught first | taught again | atoms the LATER chapter owns |
|---|---|---|
| **आहे** the copula, ch9 `MR-C02-aahe` | ch14 `MR-C07-asne` | `MR-LEX-ASNE-AAHE`, `MR-GRAMMAR-AAHE-LAST` |
| the **gendered present** (`-to`/`-te`), ch12 `MR-C05-bolne` | ch14 `MR-C07-jane` | `MR-GRAMMAR-PRESENT-GENDER` |

Chapter 9 also teaches "the verb goes last" and chapter 12 "no separate *am*",
which is the same ground `MR-GRAMMAR-AAHE-LAST` and `MR-GRAMMAR-PRESENT-GENDER`
cover in chapter 14.

**What the migration did, and why it is not the fix.** The earlier chapters took
new ids alongside — `MR-LEX-AAHE-COPULA`, `MR-GRAMMAR-VERB-FINAL`,
`MR-GRAMMAR-PRESENT-GENDERED-ENDING`. Duplicate introduction is a hard
validation error, so something had to give, and re-pointing a *generated*
chapter has the larger blast radius: chapter 14's lessons would need `requires`
where they now have `introduces`, its block directives would move, and its
payoff would have to be re-anchored. Minting alongside kept the retirement PR to
one concern. It also means the corpus now says, in the atom graph, that Marathi
introduces the copula twice — which is true, and was equally true before, just
unsayable.

**Recommendation: the EARLY chapter should own both, and chapter 14 should
`require` and `practise` them.**

  * Reading order decides it. A reader meets आहे in chapter 9 and uses it in
    every chapter after; an atom introduced in 14 for a word taught in 9 makes
    the reinforcement windows measure from the wrong place, which is why
    `reinforcementWindowMisses` is the noisiest number in the track.
  * Chapter 14's own lessons are not damaged by the change. `MR-C07-asne` is a
    consolidation of the copula across persons — genuinely a *practise* of
    something already met, not a first teaching. Same for `MR-C07-jane`.
  * The counters that would move are already understood: chapter 14's payoff
    `assesses` must be rewritten against the atoms it practises rather than
    introduces, and its representativeness recomputed against the 0.5 floor.

**Cost, so nobody starts it blind.** Four lesson files (`MR-C02-aahe`,
`MR-C05-bolne`, `MR-C07-asne`, `MR-C07-jane`), two `chapters.d` payoffs, the
three duplicate ids retired, and a full regeneration. It is one focused PR, not
a tranche — but it must be done as its own PR, because it moves reinforcement
numbers across the whole track and mixing that into a content change would make
both unreviewable.

**Do not "fix" it by deleting one side's prose.** Both chapters teach their half
for a reason: chapter 9 needs आहे to finish *mājhaṁ nāv … āhe*, and chapter 14
needs the full person paradigm. The duplication is in the ATOM IDS, not in the
teaching, and the repair is ownership, not deletion.
