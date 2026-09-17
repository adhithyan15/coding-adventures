---
category: Repo policy / workflow reminders
---

# A glyph that only appears inside another letter lesson's example words counts as taught, so the never-taught-glyph metric understates real script debt by a factor of three

`measureScriptClosure` credits a glyph to **any** lesson with `type: writing`
or `delivery: script` whose text contains it. Not its headword — its **body**.

Every letter lesson ends with a list of words the learner already says, so the
reader can find the new shape inside them. Those example words are full of
**other** letters, and each one is silently marked taught by the lesson that was
teaching something else.

`HI-S117-letter-ca` teaches **च**. Its example list contains **एक दो तीन चार
पाँच**. From that line alone, the measure concludes that **ए** has been taught.
Nobody has ever drawn **ए** for the learner.

**The size of the gap.** The Hindi track has 29 script lessons, and their
subjects are exactly:

```
म न अ आ ् ा त स र क े प ह य ी च ँ ष ु ल ख द ध ऋ थ उ ऊ ओ छ
```

The metric reports **6** never-taught glyphs. Ten more have never been the
subject of any lesson and are credited purely by incidental appearance:

| glyph | what it is |
|---|---|
| ए | the independent vowel *e* |
| ग घ ज ञ ट ठ फ | seven ordinary consonants |
| ़ | the **nuqta** |
| ृ | the vocalic-r **sign** (its independent ऋ is taught; the sign is not) |

So the real figure is **16, not 6**. `scriptClosureViolations` understates the
same way, because a lesson using one of these ten is never counted as being in
debt for it.

**It also reconciles a disagreement nobody had resolved.** The exam inventory's
`HI-A1-SCR-15` says fifteen consonants ordinary A1 text needs are untaught, and
`HI-A1-SCR-16` calls the nuqta *"the single sharpest finding in this file"* —
the corpus's own headwords are full of *shukriyā*, *zarūr*, *darvāzā*, *mez*.
The metric said six. **The inventory was right and the metric was wrong**, and
the two had been contradicting each other unnoticed.

**The helper script inherits the bug.** My `taught.mjs` reproduces the measure's
rule, so it answered "ok" for **चाहiye** and for **ट्रेन** — and **ए** and **ट**
are both on the list above. A check built from the same definition as the thing
it is checking cannot find this class of error.

**The fix when asking "has this been taught".** Build the taught set from
**headwords**, not bodies:

```
for (const ch of lesson.realization.headword ?? "") taught.add(ch)
```

That is what a learner would call taught: the lesson whose subject it was. Keep
the body-based set too and treat the difference as **exposure** — a glyph the
reader has seen without instruction, which is useful to know and is not the same
claim.

**The general shape.** A metric that credits a side effect will drift away from
the thing it is supposed to measure, and it drifts *upward*, because side
effects accumulate. When a metric and a hand-written audit disagree, the audit
is the one that looked. Check what the metric counts as a positive before
trusting a number that is going the way you want.
