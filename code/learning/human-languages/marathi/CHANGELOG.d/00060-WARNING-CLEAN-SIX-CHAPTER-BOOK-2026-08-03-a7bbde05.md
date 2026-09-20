## Warning-clean six-chapter book — 2026-08-03

- Gave the five handwritten recap sections stable canonical lesson labels rather
  than five copies of `lesson:practice`.
- Kept Devanagari text in PDF bookmarks while removing font-only commands from
  Hyperref strings, and let pages around unbreakable callouts end naturally.
- Declared the vendored static Devanagari file for every requested font shape.
  The forced 31-page build now has zero LaTeX/package warnings, missing glyphs,
  overfull or underfull boxes, and duplicate destinations.

