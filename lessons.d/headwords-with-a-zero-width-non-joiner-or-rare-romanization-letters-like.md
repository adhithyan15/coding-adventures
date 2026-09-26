---
category: Repo policy / workflow reminders
---

# Headwords with a zero-width non-joiner or rare romanization letters like ṁ fail the book glyph-coverage gate

Two generated tranches failed `glyph-coverage.test.ts` ("every character in
every generated book renders"):

- Marwadi romanized मांस as *māṁs*. The book font has no U+1E41 (ṁ).
- Persian spelled سیب‌زمینی and قهوه‌ای with U+200C (ZWNJ). The book's Arabic
  font has no glyph for it.

The fix: romanize nasals with a combining tilde (*mā̃s*), which the fonts
already cover because Urdu and Hindi use it, and drop or replace ZWNJ
headwords.

Before generating a tranche, grep the candidate list for U+200C/U+200D and for
precomposed dot-above letters (ṁ ṅ ḣ). A quicker check than the full suite is
`npx vitest run tests/glyph-coverage.test.ts` after `generate:books`.
