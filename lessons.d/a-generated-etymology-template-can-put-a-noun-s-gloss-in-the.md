---
category: Testing & coverage
---

# A generated etymology template can put a noun's gloss in the root's sense slot

The Arabic etymology pass generated about 1,100 native-word blocks from one
template: "built on the native Arabic root X, 'sense' (Lane)". The script filled
'sense' from the lesson's own gloss when it could not find the root's verb
sense, so many blocks said things like "root s-m-k, 'fish'" or "q-ṭ-r, 'a file
of camels'", where Lane gives the root as "to rise, to be high" and "to drip".
Every root existed in Lane and the scripted check (root exists, word under it)
passed, so the error was invisible to it. Two independent reviewers found it in
about 18% of the 1,262 lessons, against 1-6% in tracks whose blocks were
written per word.

A second trap: the first rewording, "an old noun that Lane lists under X",
tripped none of the empty-block detector's origin signals, and 35 blocks went
back to counting as empty. "An old native noun, filed under the root X (Lane)"
reads as an origin.

**What to do:** when a fill script generates prose from a template, verify the
filled slot, not just the citation: for a root-sense slot, check that the gloss
comes from the root article's verb senses, and make the script refuse (leave
for hand-writing) rather than fall back to the noun's gloss. After any bulk
rewording of origin blocks, run `tests/root-ledger.test.ts` before handing off.
