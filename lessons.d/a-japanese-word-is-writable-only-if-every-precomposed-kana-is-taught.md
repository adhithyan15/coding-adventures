---
category: Repo policy / workflow reminders
---

# A Japanese word is writable only if every precomposed kana is taught and in the script inventory

**What went wrong.** The first draft of the Japanese A1 tranche checked each
candidate word by decomposing it (NFD) and asking only whether the base kana
had a writing lesson, treating the dakuten ゛ and handakuten ゜ as taught marks.
That admitted two kinds of word the corpus refuses:

- **A taught sign missing from the script inventory.** ら has a writing lesson,
  but `data/scripts/japanese.d/letters/` has no row for it. No earlier word
  headword used ら, so nothing had noticed. Twenty new headwords with ら put
  ら at the head of the corpus glyph-gap queue, which
  `tests/script-inventory-queue.ts` pins empty.
- **A precomposed voiced kana the book never teaches.** Script closure counts
  ず, ば, で and the rest as glyphs in their own right. Only が, ご, ざ, ぼ, ぽ,
  ど and だ are taught. The new headwords made nine glyphs "shown but never
  taught" (`neverTaughtGlyphs`), and the reviews that printed them became
  closure violations, which lifted the corpus ceiling in `script-closure.test.ts`
  and added a `script-closure` finding to the gentle-ramp pin.

**The fix.** The checker now intersects three sets: the taught base kana, the
glyphs the script inventory covers, and a fixed list of the taught precomposed
voiced kana. About 90 candidates dropped out and 60 new ones replaced them.
Each review lesson recalls a word by romanization alone when its headword
carries an untaught voiced kana, and each review's typing activity picks a
word whose kana are all taught.

**Do differently.** For any script, "allowed letters" means taught ∩
inventory-covered, measured on the *precomposed* glyph the lesson prints, not
on an NFD decomposition. Measure it before generating anything, from the
closure report (`measureScriptClosure(...).tracks[].neverTaughtGlyphs`) and the
validator's `uncovered-glyphs` warnings, which should stay at zero.
