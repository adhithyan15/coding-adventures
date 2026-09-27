---
category: Repo policy / workflow reminders
---

# A letter-allowed list for new headwords must intersect the lessons' taught letters with the script data file's glyphs

**What went wrong.** The Sanskrit A1 tranche built its allowed-letter set only
from the letters the lessons teach. The track teaches ङ (SA-W65-letter-nga),
but `data/scripts/devanagari.json` has no entry for it. Ten new headwords used
ङ, including कङ्कणम् and अङ्गुलिः. Validation reported
"characters not yet in devanagari.json: ङ" on every one of them. The earlier
pre-A1 tranche had hit the same wall; that entry is in the Sanskrit
CHANGELOG.d, 00320.

**The fix.** The ten headwords, and the review lessons that name them, now use
the anusvara spelling (कंकणम्, अंगुलिः, संगणकम्, …). This follows the
precedent of मंगलवासरः. The romanizations keep *ṅ*, which is how that spelling
is read aloud.

**Do differently.** When you build a candidate checker's allowed-letter set,
take the letters the track teaches and intersect them with the script file's
glyph keys, then run the validator (`npx vitest run tests/integration.test.ts
-t "zero validation errors"`) before regenerating. A letter can be taught in a
lesson before the script data catches up.
