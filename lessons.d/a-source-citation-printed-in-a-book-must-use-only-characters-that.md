---
category: Testing & coverage
---

# A source citation printed in a book must use only characters that book's fonts carry

The HL-C443 letter lessons print their stroke-order source under the numbered
steps. The source text comes verbatim from the `script-ductus` ductus records.
The glyph-coverage gate (`tests/glyph-coverage.test.ts`) caught the same kind of
failure twice in one day:

- Arabic ى cited a site with a Japanese title, (アラビア語学習メモ). The Arabic
  book's fonts have no CJK, so eight characters would not render.
- Urdu خ cited a section titled "Ḳhe" (U+1E32, K with dot below), and the Urdu
  book's fonts have no glyph for it.

Both would have failed CI's XeLaTeX build as missing characters. The gate caught
them locally because it scans every generated chapter.

The fix is in the lesson generator. It drops a parenthesised CJK title, since
the English title beside it still identifies the source, and it writes Ḳ as
plain K. The ductus data keeps the exact citation; only the printed copy is
normalised.

**What to do differently:** text copied from a data source into a book is
content like any other, and it inherits the book's font limits. Run
`tests/glyph-coverage.test.ts` after generating chapters, before the full
suite. It is fast and names the exact character and file.
