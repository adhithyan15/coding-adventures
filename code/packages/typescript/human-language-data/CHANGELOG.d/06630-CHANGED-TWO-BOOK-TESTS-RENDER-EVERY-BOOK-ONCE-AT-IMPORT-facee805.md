### Changed — two book tests render every book once, at import

`tests/chapter-modality-book.test.ts` and `tests/book-tex.test.ts` now render every book once, at import. Previously the test body rendered them, and in `book-tex` that happened in two different tests.

"covers every generated and handwritten chapter opening in all 23 books" timed out at the 30s budget under full-suite load on CI, on the Kannada A1 PR. It passes in isolation. This follows the shared-parse change to `modality-manifest` and `chapter-intro`: rendering all 23 books grows with every content PR, so the render moves out of the test budgets rather than the budget going up. No assertion changes. That test now runs in 0.3s locally.
