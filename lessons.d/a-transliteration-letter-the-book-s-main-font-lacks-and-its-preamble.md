---
category: Testing & coverage
---

# A transliteration letter the book's main font lacks and its preamble does not map, such as ṁ in the Urdu book, fails the glyph gate

The Urdu B1 vocabulary cited Sanskrit forms with CDIAL's anusvara letter
ṁ (U+1E41), as in *sáṁtāyatē*. The Gujarati book prints ṁ because its
preamble maps it with `\newunicodechar`; the Urdu preamble maps nothing, and
Latin Modern has no ṁ. `tests/glyph-coverage.test.ts` ("every character in
every generated book renders") failed on eleven Urdu chapters. Other accented
letters such as ṓ and ḗ passed, because the book generator writes them as TeX
accents.

The fix was in the content: the specs write ṃ (U+1E43), the equivalent IAST
anusvara, which the main font carries. No preamble, font or baseline changed.

What to do differently: before generating a track's etymologies, check each
non-ASCII transliteration letter against that track's own book. A letter that
prints in one book can fail in another, because each preamble maps different
characters. When in doubt, use the letter the corpus already prints in that
track (ṃ, ṅ, ṇ, ṛ, ṣ, ś).
