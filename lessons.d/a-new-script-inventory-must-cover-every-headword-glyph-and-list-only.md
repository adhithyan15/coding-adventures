---
category: Testing & coverage
---

# A new script inventory must cover every headword glyph and list only letters the track reads

**Context:** adding `code/learning/human-languages/data/scripts/bengali.json`,
the first inventory for a track that had none, so `script-ductus` could cite
Bengali pen paths.

**What happens:** a script file is not only read by `script-ductus`.
`human-language-data`'s `loadScripts` picks it up too, and two corpus gates
start measuring the track against it:

- `tests/script-inventory-queue.ts` pins the corpus glyph-gap queue to `[]`.
  Any Bengali code point in any Bengali headword that no row covers becomes a
  gap, and the queue test fails. Partial inventories ("only the cited
  letters") break it at once. `validate.ts` did not count `digits` rows, so
  the eleven Bengali digit lessons broke it even with a `digits` collection.
- `tests/letter-anchoring-ceilings/<track>.json` pins `unreadInventory`. A
  track with no inventory measured 0; listing the whole alphabet adds every
  letter no word reads (Bengali: 20 of 50) and breaks the ceiling.

**Fix:** compute the set of script characters the track's headwords use
(NFD), check that every one is read by some non-writing lesson, and make the
inventory exactly that set: letters in `letters`, signs in `marks`, digits in
`digits` (with `uncoveredGlyphs` taught to count digits). Rows without a cited
order stay recognition-only (`strokeOrder: []`, no `penLifts`).

**Do differently:** before adding a script file, run the integration test and
`tests/letter-anchoring.test.ts` with an empty draft of it; both gates tell you
the required row set before any ductus work.
