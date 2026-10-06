---
category: Repo policy / workflow reminders
---

# A generated vocabulary tranche must check romanization glyphs, slug collisions and review length before the suite

**What went wrong.** The first Hindi A2 vocabulary tranche (chapters 168-192)
was generated as one `hlgen.py` spec, and the full suite caught three separate
mistakes:

1. **Glyph coverage.** Four romanizations used *ḳh* (U+1E33) for ख़. The main
   book font has no ḳ, so `glyph-coverage.test.ts` failed. The Hindi track
   writes ख़ as *kh* (*khālī*), so the romanization was also off-convention.
2. **Slug collision.** *marnā* ("to die") and *mārnā* ("to hit") both slugged
   to `marna`, so the concept `HI-VERB-MARNA` was realized twice.
3. **Review length.** `add_reviews` puts two reviews in the last chapter,
   covering the whole spec. With 25 chapters, each review covered about 60
   words and ran past the 300-second budget.

**Fix.** Romanize ख़ as *kh*. When an ASCII slug collides, spell the long
vowels out (`maarnaa`). Split the tranche into three specs of 8-9 chapters,
each with its own pair of reviews, as the A1 tranches already did.

**Next time.** Before generating, check that:
- every romanization character is one the track already uses;
- the ASCII slugs are unique;
- a spec has at most about 10 chapters, so its reviews fit the budget.
