---
category: Repo policy / workflow reminders
---

# Romanizations must use only characters the book fonts carry: U+02B9 for the Russian soft sign fails glyph coverage

**What went wrong (Russian A1 tranche).** The new word specs wrote the soft
sign in romanizations as U+02B9 MODIFIER LETTER PRIME (*plóshchadʹ*,
*kholodílʹnik*). The book's main Latin font has no glyph for it. The glyph
coverage gate ("every character in every generated book renders") failed on
29 chapter files, and only the full suite caught it.

**Fix.** Russian romanizations on main already drop the soft sign (*pyat*,
*désyat*, *sem*), so the new lessons, narration and book chapters dropped it
too. Book, narration and modality ledgers were regenerated.

**Do differently.** Before generating a tranche, build its romanizations from
the characters earlier lessons in the same track already use. Run
`npx vitest run tests/glyph-coverage.test.ts` right after generating the book
chapters. It takes seconds, and it catches a missing glyph before the full
suite does.
