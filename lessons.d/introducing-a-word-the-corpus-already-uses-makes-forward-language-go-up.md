---
category: Repo policy / workflow reminders
---

# Introducing a word the corpus already uses makes forward-language go up, and that is the measurement catching up

Hindi chapter 98 introduced **काम** (*kām*, "work") and **से** (*se*, "from,
by"). `forward-language` went **13 → 18**, and every one of the five new entries
named one of those two lessons as `taughtBy`.

The reflex is to treat a five-point rise as a regression and look for a way to
avoid it. That reflex is right sometimes and wrong here, and telling the two
cases apart is the point of this note.

**The wrong case, which happened one chapter earlier.** The contrast lesson in
chapter 97 was headworded **करता हूँ / कर रहा हूँ**. That claimed to introduce
*kartā hū̃*, a form taught since chapter 5, so five earlier lessons that use it
were suddenly "early". Nothing about the corpus changed; a headword told a
falsehood. Renaming it to a metalinguistic headword put the count back.

**The right case, which happened here.** *Kām* had appeared in nine lessons
since chapter 5 — inside *kām karnā*, "to work" — and had **never been
introduced as a word**. *Se* had appeared in fourteen. Both were real holes, and
the detector could not see them, because `measureContinuity` reports a use as
early only relative to a lesson that claims to teach the word. **With nothing
claiming to teach it, an untaught word is invisible to this metric.**

So the chapter converted invisible debt into visible debt. The number went up
and the corpus got better. Renaming the headwords to dodge the rise would have
put the hole back and hidden it again.

Two things to do differently.

**Diagnose by `taughtBy`, then ask one question: was this word taught
before?** Group the new entries — they almost always share a `taughtBy`, which
is the lesson you just wrote. Then check whether any *earlier* lesson already
introduces the word. If yes, your headword is lying and the fix is the headword.
If no, the entries are real, the debt predates you, and the fix is to accept the
number and say so in the changelog. A thirty-second grep for
`^headword:.*<word>` settles it.

**A rise is not evidence either way on its own.** Both cases here were +5, on
consecutive chapters, from the same metric, and they needed opposite responses.
The size of the jump tells you nothing; only the answer to "was it already
taught" does.
