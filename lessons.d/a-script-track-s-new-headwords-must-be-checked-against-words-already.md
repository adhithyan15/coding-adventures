---
category: Testing & coverage
---

# A script track's new headwords must be checked against words already in lesson text and against the script-closure pin, not only the written-letter set

**What happened:** the Tamil A2 word list was checked for three things: every
letter is one the course has taught people to write, no headword is already
taught, and no gloss repeats a taught gloss. Two tests still failed locally.

1. `tests/corpus/tamil/keeps-tamil-s-opening-free-of-future-farewells-and-pronouns-*.case.ts`
   went from 8 forward references to 14. Five new headwords (அதிகாலை, நில்,
   கால், தினமும், தயார்) already appeared in the text of earlier lessons, as
   example words or inside a longer headword, so teaching them later turned
   every earlier use into a word used before it was taught.
2. `integration.test.ts` ("keeps script inventories source-backed and
   closure-pinned") rejected ஸ in கிறிஸ்துமஸ் and முட்டைக்கோஸ். ஸ is in the
   written-letter set, but it is not in the script inventory that the closure
   pin allows.

**The fix:** the 27 candidates whose word already occurs as a token in lesson
text (or inside another candidate's multi-word headword) were dropped before
the chapters were built. The two ஸ words were swapped in place for unused words
from the same category, so the chapters already committed did not move.

**What to do differently:** for a script track, filter the candidate list
before building chapters:

- against every Tamil (or other script) token already in lesson bodies;
- against the tokens inside other candidates' multi-word headwords;
- against the script inventory's closure pin, not only the written-letter set.
