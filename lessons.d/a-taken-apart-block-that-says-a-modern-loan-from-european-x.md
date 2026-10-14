---
category: Testing & coverage
---

# A taken-apart block that says a modern loan from European X trips no origin signal, so the empty-block pin rises

Filling the Arabic etymology gaps (HL24 section 9), the first generated batch
brought `wordLessonsWithEmptyEtymologyBlock` for Arabic from 19 to 22, not
down. Three new blocks named a perfectly good origin in words no signal reads:
"a modern loan from the European word, Italian *banco*", "shaped from European
*television* to fit an Arabic pattern", and "an old pointing word of place ...
it has no verbal root". `a-loan` wants the bare phrase "a loan" (an adjective
in between defeats it), "European" is not in `SOURCE_LANGUAGES`, and Italian
is not a `DONOR_LANGUAGES` name, so "Italian *banco*" is not a verbless
ancestor either.

Fix: say "borrowed from", "built from", "traced to", or "not settled", or cite
"(Lane)" / "(EtymDB)" in the block. Each of those is a listed signal.

Do differently: after generating blocks, run the gap measurement for the track
(`measureEtymologyCoverage` over its lessons) before anything else, and read
the `emptyEtymologyBlock` list; a rise there is a wording problem, never a pin
to raise.
