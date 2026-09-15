- **28 redundant row locators dropped** across `anatomy/brain-parts.adj` (13), `biology/flower-parts.adj`
  (7), `science/scientific-method-step.adj` (7) and `anatomy/respiratory-parts.adj` (1), per the rule
  `geography/reference-lines.adj` already ships: restate a row `locator` when its page DIFFERS from
  the envelope's, inherit when it is the same. The rows that legitimately differ keep theirs —
  brain-parts keeps 2, respiratory-parts keeps 7.

  **No answer moved**, and that is the whole claim: `row_provenance` assigns `locator` only when a
  row supplies one, so a row locator equal to the envelope's is a no-op. Proved per table by
  running the full-table query against the pre-drop and post-drop files and requiring
  **byte-identical** output. 4 of 4 identical.

  **Dropping duplication is not the same as preventing it**, and the proof harness caught that:
  re-adding the envelope URL to a row was originally caught only on `respiratory-parts`. On
  `flower-parts` and `scientific-method-step` it passed the entire suite; on `brain-parts` it
  reddened five unrelated tests that noticed the changed citation, but nothing asserting the
  rule. So each table gained the guard its shape calls for — single-page tables assert NO row
  locator; brain-parts asserts none equals the envelope's — and all four now redden.

  Two harness defects surfaced on the way, both mine: the brain-parts guard hardcoded an
  envelope URL from memory of an earlier state of the table and failed against a correct file
  (it now **reads** the envelope from the file), and the proof harness expected a test name
  `scientific-method-step` does not have, reporting a missing guard that was present.
