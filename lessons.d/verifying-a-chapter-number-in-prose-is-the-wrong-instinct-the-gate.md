# Verifying a chapter number in prose is the wrong instinct — the gate forbids the reference, not the staleness

Authoring the round-two Hindi tranche I wrote seven pointers back at earlier
material by number ("since chapter 37", "back in chapter 46"), and — following the
existing lesson that a claim moved to book scope must be checked — I looked every
one of them up in the corpus first. Two were wrong and I corrected them. I felt
careful. All seven then failed `chapter-references.test.ts`, which pins
cross-chapter prose references per track and holds hindi at 20; the tranche took it
to 27.

The point of HL-C102 is not that the number might be wrong today. It is that a
number **correct when written goes stale the next time a chapter splits**, silently,
pointing the reader into the wrong chapter with nothing failing. Spanish is pinned
at zero because Spanish is the track that actually renumbers. Verifying my numbers
made them accurate and left the debt exactly as toxic.

**Rule: never write "chapter N" as a cross-reference in lesson prose. Name the
thing** — "when the ear was named", "at the end of the welcome chapter", "when you
first counted to five". The test's own docstring says it: *the fix in prose is
never a fresher number.* Check for the gate before assuming the careful version of
a habit is the wanted one; here, being careful about the numbers was a more
polished way of doing the forbidden thing.

**Corollary — a blunt content detector can be right about a sentence that was
already wrong.** The same tranche had one lesson flagged `sight` on the cue "see
the", from *once you can see the seam in the middle*. The lesson teaches hearing a
morpheme boundary and its own drill line three lines below said **hear**. The
detector's comment explicitly warns against rewriting correct prose to appease it —
but this prose was not correct, and the flag found an internal contradiction that
reading had not. Check whether the sentence is right before deciding the detector
is wrong.
