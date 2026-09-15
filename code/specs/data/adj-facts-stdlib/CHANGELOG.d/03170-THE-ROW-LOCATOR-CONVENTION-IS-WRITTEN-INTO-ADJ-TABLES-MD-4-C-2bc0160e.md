- **The row-locator convention is written into `ADJ-TABLES.md` §4** (closes the first item of
  #15197), and the shipped set is normalised to it: *a row restates `locator` only when its page
  DIFFERS from the envelope's; otherwise it inherits.* This is what §RS-5e's own normative example
  already did and its prose never said, and what `geography/reference-lines.adj` states in terms —
  the stdlib was deciding it per table and drifting both ways.

  The reason is not tidiness: a row `locator` equal to the envelope's is a **no-op**
  (`row_provenance` assigns `prov.locator` only when the row supplies one), so the duplicated
  literal is something **no test can distinguish from its own absence** — every single-page
  table's harness had to *declare* "drop a row's locator" an equivalent mutant. Inheriting makes
  "this table is single-page" structurally visible instead.

  Measured after normalising: of 349 tables with a table-level locator, **306 carry zero row
  locators, 6 carry one per row, and 3 are mixed** — mixed is normal for a genuinely multi-page
  table, and `geography/reference-lines.adj`, the file that states the rule, is one of them.

  One violation was left by a PR merged earlier the same day: `biology/vitamin-deficiency-symptom.adj`
  had its vitamin-A row restating the envelope's own URL while its four siblings legitimately
  differ. Dropping it produces **byte-identical** output, verified by running both. Its test then
  failed on `locators.len() == 5` — an assertion that was a **proxy** for the property its own
  name claims. Counting locator *lines* measures how a file is written, not what page each row
  resolves to, so it reddened on a change that moved no row's page. It now computes each row's
  **effective** page (its own locator if present, the envelope's otherwise) and asserts those five
  are distinct: stronger, and independent of the convention.

  **Two equivalent-mutant declarations this same change falsified** were left standing until a
  twin-grep found them, and both are now retired. `facts_flowerparts_e2e.rs` still declared that
  "every row on this table cites the envelope's own URL" and that dropping one "cannot be killed" —
  that file now has **zero** row locators. And `anatomy/respiratory-parts.adj`'s HEADER still
  called the lungs-row coincidence a live equivalent mutant while its own table comment, fifteen
  lines below, already recorded that the row inherits: a provenance artifact contradicting itself
  about provenance. Both now record the history in the past tense rather than asserting it.

  Counted on this branch: flower-parts 0 row locators (7 rows), respiratory-parts 7 (8 rows — the
  lungs row inherits), scientific-method-step 0, brain-parts 2 of 15.

  Also corrected in the spec: §2 still called per-row provenance "a documented future extension"
  while §6's own capability table records RS-5e as delivered.

