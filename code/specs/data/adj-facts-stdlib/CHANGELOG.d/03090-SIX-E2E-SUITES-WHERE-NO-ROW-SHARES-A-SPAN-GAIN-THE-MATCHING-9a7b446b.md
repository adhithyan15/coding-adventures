- Six e2e suites where NO row shares a span gain the matching assertion (#15193, third batch):
  `respiratory-parts`, `plant-tropisms`, `rainforest-layer`, `atmosphere-layers`,
  `air-quality-index` and `energy-forms` now assert, against the shipped `.adj`, that every
  row's sentence is distinct. For the five single-page tables that also means asserting that
  **no row carries a `locator` at all** — which is what makes "one page" a checked fact rather
  than a paragraph; a row-level locator appearing there would mean the table had quietly become
  multi-page.

  **`astronomy/planets.adj` is excluded, and why is stated**: it has ONE row, so "all spans
  distinct" is true of it no matter what anyone edits. An assertion that cannot fail is
  decoration.

  **12 fire-proofs, 12 behaved as intended** — a duplicated span per table, plus an added
  row-level locator (or, for `respiratory-parts` whose rows do carry locators, two pages
  collapsed into one). The harness names the test that must redden, so a kill by one of the
  five-plus other assertions in these suites is reported as a failed experiment rather than
  counted as success.

  With the sibling entries, **16 of the 17 tables converted under #14986 now assert their own
  span structure, up from the 2 counted on 2026-09-14** — the seventeenth being `planets`,
  excluded above. That count was recounted by enumerating the tables against the batches; a
  first draft of this entry said 14.

