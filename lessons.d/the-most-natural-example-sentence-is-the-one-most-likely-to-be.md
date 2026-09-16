---
category: Repo policy / workflow reminders
---

# The most natural example sentence is the one most likely to be in script debt, because natural means idiomatic and idiomatic means high-frequency words the curriculum has not reached

Hindi chapter 102 needed an example of a clock span. The obvious one wrote
itself:

> **नौ बजे से पाँच बजे तक** — *from nine to five*

It is the working day. It is what a person would actually say. And **नौ**
contains **ौ**, the au-mātrā, which no writing lesson has drawn — so two
lessons went into script debt and `scriptClosureViolations` moved **30 → 32**.

**Every gate passed.** The regression was visible only in
`git diff core/gentle-ramp-snapshots/hindi.d/metrics/`.

**Why the good example is the dangerous one.** The glyph check was run *before*
authoring, on the chapter's **planned vocabulary**: `tak`, `somvār`,
`śukravār`, `ṭren`, `subah`, `śām`. All clean. **नौ** was never on that list,
because it is not a word the chapter teaches — it arrived inside an
illustration, at writing time, precisely because it was the most idiomatic
filler available.

That is the trap in general form:

| what gets checked | what does not |
|---|---|
| the words the chapter **introduces** | the words the chapter **illustrates with** |

And illustration vocabulary is drawn from exactly the pool most likely to be in
debt: the commonest, most natural words, which are common enough that a
beginner curriculum has usually not reached them by the chapter that needs
them.

**The fix, and the two habits.** Changed **नौ** to **दस**, which is clean. One
word. Then: run the glyph check on the finished draft, not the plan — the plan
lists what you teach, the draft contains what you wrote, and only the second is
what ships. And diff the snapshot metrics directory after regenerating, because
this class of regression passes every gate; if the only thing you read is gate
output, you will ship it.

**The accident was also a measurement.** It named the next alphabet item for
free: **ौ is the largest single glyph debt left in the track at four lessons**,
its earliest use is chapter 16, and *nau* is one of the words it blocks. A
finding you trip over while doing something else is still a finding — write it
into the backlog before you fix it away.
