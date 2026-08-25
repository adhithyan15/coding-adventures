### Fixed — the book no longer sends its reader into a git checkout (HL-C50)

- **105 handwritten chapters printed a repo path at somebody holding a PDF**:
  *"Practice lessons: `lessons/AR-C01-*.md`"* (99) and *"Companion lesson:
  `FA-C02-esm-e-man.md`"* (6). `book-cli.ts` already stated the principle — "a reader
  holding the PDF cannot follow a link into a Git repository" — and drops relative
  links for exactly this reason. The handwritten chapters had never been held to it.
- **The worst instance was on a title page, and a chapter-only check could not see
  it.** Japanese and Chinese printed *"Companion practice lessons live alongside this
  book at `code/learning/human-languages/japanese/lessons/`"* on the **title page** —
  more prominent than any chapter clause. `loadBookCorpus` records `entrypoint` as a
  path and never reads it, so the first version of the guard test passed green while
  the defect sat on page one. The test now reads `book.tex` too.
- Chinese's printed "Sources" backmatter also cited `data/scripts/chinese.json` by
  repo path and sent the reader to "the companion Markdown lessons". The source is
  now named without the path, and the dangling pointer is gone.
- Zero generated files touched: 416 chapters, 311 generated, 105 modified — the
  intersection is empty, and the change set is exactly the complete set of handwritten
  chapters.
- The guard covers **all** chapters including generated ones, so it is a regression
  test on the generator too, not only on handwritten prose.

