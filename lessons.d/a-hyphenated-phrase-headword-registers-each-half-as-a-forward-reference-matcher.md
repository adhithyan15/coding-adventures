---
category: Testing & coverage
---

# A hyphenated phrase headword registers each half as a forward-reference matcher, so excusez-moi turned every earlier moi into a borrowing

The French B1 core added a `phrase` lesson with the headword `excusez-moi,
pour aller à… ?`. The `fwdcheck words` probe for `excusez-moi` reported
USED-EARLIER with three hits, and they were read past because none of them
contained *excusez*. They contained *moi*. The continuity pin went from 52 to
55: FR-C39-lui-elle, FR-C139-non-plus and the core's own FR-C260-je-pense-que
were now "using *moi* before FR-C262-excusez-moi teaches it".

`taughtWords` in `src/continuity.ts` splits a headword on `/`, `,` **and on
any dash, hyphen included** (the module comment says so: right for a range
like "once — quince", wrong for "dix-sept"). So `excusez-moi` registered two
matchers, `excusez` and `moi`. `moi` had been taught long before, but only
inside the multi-word headword `moi et toi`, which owns no single token, so
this phrase lesson became `moi`'s first owner and every earlier bare *moi*
became a forward reference. Note the asymmetry: the `excusez` half could never
match anything, because in running text it is followed by a hyphen, and a
hyphen counts as word-adjacent.

The fix that kept the measurement honest was to make the lesson what it was:
*excusez-moi* had already been mentioned in chapter 24, and the lesson's new
content is the direction-asking frame, so it became `type: grammar`, which
registers no matcher at all. Swapping in a non-breaking hyphen to dodge the
splitter would have been renaming a lesson to work around a measurement.

What to do differently: before giving a hyphenated item a `word` or `phrase`
lesson, split it on the hyphen yourself and probe each half against the
earlier lessons, and read every hit a probe reports for the half it matched,
not for the whole string you asked about. French has many such items (*excusez-moi*,
*aller-retour*, *peut-être*); *aller-retour* is fine only because both halves,
*aller* and *retour*, already had owners of their own.
