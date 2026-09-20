## Book warning cleanup — 2026-08-03

- Kept punctuation outside the Bengali-only font and replaced five duplicate
  recap anchors with stable chapter-qualified labels.
- Preserved Bengali in PDF bookmarks while suppressing the font-only command
  there, and mapped the vendored static font to every requested shape.
- Let short lesson pages end naturally and made the long farewell title
  breakable so the forced six-chapter build has no layout, bookmark, label,
  font, punctuation-glyph, or package warnings.

