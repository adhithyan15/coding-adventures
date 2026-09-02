## HL-C305 — choosing off an inventory beats choosing by topic, and the one budget it broke

`core/exam-inventory-tamil-a1.json` shipped with `Iṇaittoḍar (joining clauses)`
at 0 of 7 and a note on every uncovered point saying what was missing. Taking
that column rather than picking a topic moved Tamil A1 coverage from 155/262 to
**174/262** with thirty-one lessons: nineteen points for eight chapters. The
Hindi precedent held exactly — a health chapter designed by topic once measured
at one exam point and was cut, and its replacement, taken straight off the
uncovered list, closed five.

### The finding worth carrying: an append lengthens the track, and that alone raises the reinforcement number

`continuity.ts` judges a reinforcement window only when the track is long enough
to contain it: `if (at + window.from > last) continue`. Adding thirty-one
lessons moved `last` from 358 to 389, which made R4 judgeable for every
PRE-EXISTING atom introduced between positions 279 and 309 and R3 judgeable for
those between 339 and 358. Fifty-five window misses appeared on lessons this
tranche never opened.

That is not a measurement bug. Those atoms genuinely were never retrieved at
that distance; the track was simply too short for anyone to say so. But it means
**a tranche cannot hold the reinforcement number by looking after its own atoms
alone.** The new lessons are the only thing standing at the right distance from
the old ones, so they have to do the retrieving.

The rule that worked, and it is mechanical enough to reuse: give every lesson in
the tranche a second retrieval line naming **one item from a few lessons back,
one from roughly twenty chapters back, and one from roughly eighty lessons
back**, and make the task the chapter's own pattern so the retrieval is real
practice rather than a declaration. Ninety-two window misses were placed that
way and Tamil's finding came back to 1200 exactly, unchanged, with thirty-one
lessons on top of it.

### The budget this broke, and why it was not smuggled

`TA-A1-NUM-08` named a near miss precisely: eight chapter-7 lessons teach the
Tamil digits ௧–௰ and every one declared an empty `introduces` list, so the
material existed and could not be probed. Four atoms closed it and `TA-A1-L-10`
with no content written at all.

Chapter 7 was already sitting exactly on `maxNewAtomsPerChapter` at 12. Four
honest atoms take it to 16, and `atomChapterSpikes` for Tamil moves 0 -> 1. The
policy answer is a chapter split, and the reason it is filed here instead of
done is worth stating: splitting chapter 7 renumbers chapters 8 through 81, every
`chapter:` field in roughly 340 lesson files, every book target and generated
`.tex` filename, every narration shard — and, dangerously, the dozens of prose
cross-references that name chapters in words ("since chapter seven", "in chapter
forty"), which no gate checks and which a rename would silently invalidate.

**The work: split Tamil chapter 7 into two chapters and renumber the track**,
with a mechanical pass over the spelled-out chapter references. Until then the
spike is a reported finding on a real chapter, which is a better state than four
lessons that teach a script nobody can measure.

### A smaller one: `>` alone does not separate two display lines

Two consecutive `> ` lines join into one run-on paragraph in the generated book,
which no text assertion catches. A bare `>` between them does NOT fix it — it
renders as a literal `>` on its own line, and `chinese/lessons/ZH-C04-bu.md`
carries that defect today. The separator that works is a BLANK line between two
blockquote blocks. Thirteen lessons in this tranche were written the wrong way,
caught by looking at the compiled PDF, and fixed.
