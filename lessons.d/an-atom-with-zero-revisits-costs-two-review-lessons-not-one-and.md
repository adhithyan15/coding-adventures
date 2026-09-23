---
category: Testing & coverage
---

# An atom with ZERO revisits costs two review lessons, not one, and no amount of repetition inside one lesson substitutes

Seven consecutive tranches of a corpus-wide reinforcement programme sized themselves by counting
thin atoms and dividing by the per-lesson ceiling. Fifty Telugu atoms became nine lessons; thirteen
Gujarati atoms became **one**. The rule of thumb — *one retrieval per atom* — held every time, and
it was never the rule. It was a coincidence of which tracks came first.

The criterion reads:

```ts
const thin = relevant.filter((d) => d.revisits < 2 && !isEtymologyAtom(d.atom));
```

**Less than two.** An atom sitting at `revisits: 1` needs one more lesson. An atom at `revisits: 0`
needs **two**, and they have to be two different lessons, because the credit is counted per lesson:
`practisedAtoms` is the union of a lesson's frontmatter `practises.knowledge` and its blocks'
`assesses`, and a set counts a repeated id once. Listing an atom in three blocks of one lesson earns
exactly one revisit.

Every track in the first seven tranches happened to carry only `revisits: 1` atoms — the shape a
chapter recap produces, one pass and nothing after it. The first track with a large zero-revisit
share broke the estimate by a factor of two:

| track | thin atoms | zero-revisit | actual cost in slots |
|---|---|---|---|
| sanskrit | 28 | 17 | **45** |
| german | 37 | 19 | 56 |
| punjabi | 40 | 11 | 51 |
| kannada | 41 | **3** | 44 |
| arabic | 78 | **68** | **146** |

Read the last two rows together. Kannada has **13 more thin atoms than sanskrit and needs fewer
retrievals**; arabic has 78 atoms and costs more than three times sanskrit's 28. The headline count
predicts nothing on its own.

**The operational form** is to size this work in *slots*, never in atoms:

```
slots = (atoms with revisits 0) * 2 + (atoms with revisits 1) * 1
```

and then divide by the observed per-lesson ceiling, which is about ten to thirteen atoms before
duration and prose budgets bite.

**Two other costs travel with the count and are invisible in it.** The first is **chapter spread**: a
review lesson has to sit after every atom it retrieves and live in one coherent chapter, so 41 atoms
strewn over 24 chapters is harder to answer than 28 over 8, whatever the slot arithmetic says — that
is why the smallest atom count is not the cheapest track to take next. The second is that a very high
zero-revisit share is **not debt at all**. Arabic's 68-of-78 across a 129-lesson track means the track
has no review layer, and answering that with a couple of second-pass lessons would be papering over a
structural gap rather than repairing it.

**What made the miss cheap to catch** was measuring `revisits` per atom before authoring instead of
reading the blocker's shortfall number, which reports atoms and not slots. The gate's own message —
`28 atom(s) at or below pre-A1 are revisited fewer than twice` — is accurate and says nothing about
cost. A shortfall is a count of failures, not an estimate of the work to clear them.

Related: [[a-shortfall-is-only-comparable-to-another-shortfall-in-the-same-unit]] — there the error
was comparing shortfalls across criteria measured in different units; here it is reading a single
shortfall as though its unit were effort.
