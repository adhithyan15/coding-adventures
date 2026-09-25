---
category: Testing & coverage
---

# A new headword turns every earlier lesson that already writes that word into a forward reference

Kannada's second and third vocabulary tranches (chapters 90-106) passed every
data check. The full suite then failed `keeps Kannada's opening free of future
farewells and pronouns`: forward references had gone from 15 to 18.

A forward reference is any earlier lesson whose text writes a word that a
LATER lesson teaches as its headword (`measureContinuity` in
`src/continuity.ts`). Adding a headword therefore changes the verdict on text
that is already in the track:

- ಚಳಿ, "chilly", was already written in chapter 14's seasons lesson.
- ಹೀಗೆ, "like this", was already written in chapter 49.
- The new chapter-91 birthday lesson wrote ಹಬ್ಬ and ಶುಭಾಶಯಗಳು, which the same
  tranche only teaches in chapter 105.

The fix swapped ಚಳಿ and ಹೀಗೆ for words no earlier lesson writes: ತಲೆಸುತ್ತು,
"dizziness", and ತಪ್ಪು, "wrong". The birthday lesson now romanizes *habba*
instead of printing the Kannada.

**What to do differently:** before appending a vocabulary tranche to a long
track, grep every candidate headword against the text of the earlier lessons,
not only against their headwords. A word already printed in an early lesson is
better taught earlier (or swapped) than taught late. Also check that no new
lesson's example sentence uses a headword the same tranche teaches later.
