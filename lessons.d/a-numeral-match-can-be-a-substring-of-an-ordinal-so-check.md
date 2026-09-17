---
category: Repo policy / workflow reminders
---

# A numeral match can be a substring of an ordinal, so check the romanization before believing it

Checking whether the Hindi corpus taught the cardinals above twenty, a substring
search for **तीस** (*tīs*, thirty) and **सौ** (*sau*, hundred) returned hits in
three lessons. Read at face value, that says thirty and a hundred are already
taught and the exam point is closer than its note claims.

**Every hit was inside an ordinal.** तीस was the opening of **तीसरा** (*tīsrā*,
*third*). सौ was the opening of **सौवाँ** (*sauvā̃*, *hundredth*). The cardinals
appear nowhere in the track.

The romanization field settles it in one line, because the ordinal suffix is
visible there — *tīsrā* is not *tīs*, *sauvā̃* is not *sau* — while the Devanagari
substring is genuinely identical.

**Why this shape recurs.** Ordinals are built on cardinals in most languages, so
*every* ordinal is a false positive for its own cardinal. The same trap waits for
बीसवाँ against बीस, दसवाँ against दस, and their equivalents in every other track
in this repository.

**What to do.** When searching for a numeral, match against the **romanization**,
or require the Devanagari match to be the whole headword token rather than a
prefix of one. Print what matched before counting it.

This is the same family as the glyph-credit bugs: *the string appeared* is not
*the thing is there*. A search that cannot tell a word from the start of a longer
word will always answer optimistically.

**The consolation prize was better than the search.** Looking at what actually
matched turned up a real finding — **सौवाँ is taught and सौ is not**, so a reader
can say *hundredth* without ever being told what a hundred is. That became the
opening hook for the chapter.
