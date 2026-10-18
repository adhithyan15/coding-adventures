---
category: Testing & coverage
---

# Saying a word is a compound does not tell the etymology detector where it comes from

`hasEmptyEtymologyBlock` / `blockStatesOrigin` in
`src/etymology-coverage.ts` look for origin signals. They find "comes
from", "built from/on", "borrowed from", "inherited", a CDIAL or DEDR
number, or "not settled". Several natural phrasings match none of them:
- "is a compound of";
- a bare "(MW)" citation;
- "comes, MW says, from", where an interjection splits the verb from
  its preposition.

A block written that way still counts as empty, or as no visible
origin, although it states one.

The Sanskrit fill agents hit all three. Their verification scripts
caught it because they ran the detector on every lesson they wrote. A
reviewer's rewording of C38-soil broke the signal in the same way, and
was caught the same way.

**What to do:** after writing or rewording any etymology block, run
`hasEtymologyHook`, `hasVisibleEtymology` and `!hasEmptyEtymologyBlock`
(dist/etymology-coverage.js) over every lesson touched. Prefer "is built
from", "comes from" or a cited entry number to "is a compound of".
