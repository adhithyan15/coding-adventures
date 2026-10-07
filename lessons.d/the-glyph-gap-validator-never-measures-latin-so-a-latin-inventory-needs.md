---
category: Testing & coverage
---

# The glyph-gap validator never measures Latin, so a Latin inventory needs its own closure census by Unicode script, not by character name

**Context:** adding `data/scripts/latin.json`, the first inventory for the
six Latin-script tracks, so `script-ductus` could cite Latin print letters.

**What happened:** the inventory was scoped from a Python census that kept a
headword character when its Unicode *name* contained "LATIN" (or it was a
combining mark). The full human-language-data suite then failed in the new
`script-inventories/latin.evidence.ts`: the Portuguese headword "1.º / 1.ª"
uses the ordinal indicators U+00BA and U+00AA, which are `Script=Latin` but
are named "MASCULINE/FEMININE ORDINAL INDICATOR". Meanwhile the glyph-gap
queue (`script-inventory-queue.ts`) stayed green, and would have stayed green
for ANY Latin gap: `validate.ts`'s `uncoveredGlyphs` asks `belongsToAny`, whose
script matchers are built from `ALL_SYSTEMS`, which filters out "Latin". So
no Latin headword can ever report an uncovered glyph.

**Fix:** added ª and º as recognition-only rows, and made the inventory's own
evidence module the exact closure gate: every `\p{Script=Latin}` or mark
character of every Latin-track headword (NFD) is listed, and every listed one
is read.

**Do differently:** census a script's characters with `\p{Script=...}` (the
same property the code uses), never with character names. And before relying
on the glyph-gap queue for a new inventory, check that the validator's script
matchers include that script; for Latin they do not.
