---
category: Repo policy / workflow reminders
---

# Copy every reused target-language form from its direct lesson owner before pinning a new corpus test

The first test for Marwadi's joining tranche hand-typed **तावो** for *tavo*,
because the romanization made the long-vowel spelling look obvious. The direct
owner `MW-C118-tavo` actually teaches **तवो**. The test therefore demanded a
form the learner had never met and would have encouraged the implementation to
silently invent a spelling.

The fix was to read the canonical owner and copy both its exact atom id and
script form before authoring the example. For any reused target-language word,
build the example from the direct lesson owner's `headword`, `romanization`,
and `introduces.knowledge` fields. Do that before pinning learner copy in a
test; transliteration is not a reversible spelling algorithm.
