## Warning-free complete book (2026-08-03)

- Added explicit static-font faces for every comparison script and readable
  Unicode bookmark fallbacks, eliminating all font-shape and Hyperref warnings.
- Made the five handwritten recap labels unique, shortened only the titles that
  exceeded header or bookmark widths, and added natural page bottoms.
- Adjusted a small set of canonical multilingual examples at durable line-break
  boundaries; regenerated chapters and source hashes remain shared with
  Language Ladder rather than diverging into book-only prose.
- The forced 96-page XeLaTeX build now reports zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings,
  and font warnings. All pages were inspected again with no clipping,
  collision, accidental blank page, or leaked schema metadata.

