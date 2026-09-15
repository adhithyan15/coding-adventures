- **The last five converted tables gained a structural test of their own** (#15193). That issue
  recorded "2 of 17" on 2026-09-14; re-measured with an inline-aware parser on a branch merging
  `origin/main` with the four open per-row PRs, it is **21 of 26**. (On this branch alone, without those PRs, it was 20 of 25
  before this change — the denominators differ by `physics/em-spectrum.adj` and
  `earth-science/water-cycle.adj`, converted in unmerged PRs. Each number is labelled with the
  tree it was counted on rather than presented as one figure.) **After this change, measured on
  this branch: 25 of 25** — every converted table asserts its own span structure. These are the
  five that did not: `anatomy/body-counts.adj` (9 rows), `astronomy/planets.adj` (8), `biology/kingdoms.adj`
  (22), `metrology/si-base-units.adj` (7), `science/scientific-method-step.adj` (7).

  **The parser being inline-aware is the point, not a detail.** Three of the five write their row
  blocks inline — `row (venus, 2) { source "..." }` — and an eight-space `strip_prefix` reads
  **zero** spans from those files. Every census run for #14986 used that needle, which is how
  `kingdoms` and `si-base-units` came to be published as *unconverted* when they were converted,
  and how `planets` was reported with one row span instead of eight. A structural test written
  with the same needle would not fail; it would go **silently vacuous**, reading an empty list and
  finding nothing wrong with it. So every one of these tests asserts its parse found the expected
  number of rows and spans **before** asserting anything about them — and that assertion earned
  its place immediately: `si-base-units.adj` parsed as **one** row instead of seven, because the
  row scan split on `)` and `row (length, meter, "m") { source "Length - meter (m)" }` contains
  one inside the span. The scan is quote-aware now.

  Each table gets the shape its own data has, because there is no single template: `kingdoms` is
  22 rows over 5 spans grouped by kingdom, `planets` is 8 rows over 8 spans with exactly one row
  at `trust inferred` (the one the page does not state literally), `body-counts` is 9 rows across
  7 pages, `si-base-units` and `scientific-method-step` are 7 rows over 7 spans with no row
  locators at all.

  **Two assertions of mine fired on the real files and were wrong, not the files.** I asserted
  `body-counts` had nine distinct pages; it has **seven**, because the three hand-bone rows share
  a span *and* its page — so the assertion became the stronger and truer one, that the page
  multiset matches the span multiset exactly. And a mutant that truncated one **inline**
  `si-base-units` span survived everything: the count stayed 7 and the spans stayed distinct, and
  only two of its seven rows were pinned by content anywhere. Those spans are short enough that a
  truncation is cheap to ship, so all seven are now pinned by value.

  **Security review found a CI-blocking error and, worse, that the assertion causing it was
  inert.** `assert_eq!(adj.matches("}").count() >= 7, true, ...)` is a clippy
  `bool_assert_comparison` error under the repo's `cargo clippy --all-targets -- -D warnings`
  gate — which a green `cargo test` says nothing about, because this package's BUILD only runs
  `cargo test`. And it checked nothing: `planets.adj` has NINE `}`, so reflowing all eight rows
  to multi-line, the exact regression its message claimed to guard, would still leave nine. It
  now counts inline row blocks directly and fires on a reflow. Verified with both controls: a
  clean `cargo clippy --all-targets -- -D warnings` exits 0, and reintroducing a literal-bool
  assert reproduces the error.

  Review also found the scanners **desync on an escaped quote** — no span in these five files
  carries one, but `anatomy/brain-parts.adj` does (`"the \"flash drive\" of the human brain"`),
  and copying this parser there would truncate the span and invert the quote state for the rest
  of the line: the silent-vacuity failure this change exists to prevent, one table over. Both
  scanners are escape-aware now. Three smaller fixes: the `cites` locator scan was bounded to
  its own line (searching the rest of the body, a `cites` missing its mandatory locator would
  steal the row's own); `trust` is no longer read out of `%` comment prose; and a per-row span
  count is asserted before an index that a 0/2/1/1/1/1/1 spread would have panicked on.

  **24 of 24 mutants killed** across the five suites, each with a green baseline before and after
  — including an inline-block truncation and an inline row locator, the two shapes that have
  already defeated a guard in this series. Re-run after the parser changes above, since a kill
  record does not survive a change to the thing doing the killing. Local scratch harness, so that
  count is not reproducible from the repo.

