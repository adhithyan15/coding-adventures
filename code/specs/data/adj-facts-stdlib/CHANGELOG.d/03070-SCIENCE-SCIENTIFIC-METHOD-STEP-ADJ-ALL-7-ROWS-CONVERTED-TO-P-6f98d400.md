- `science/scientific-method-step.adj` — all 7 rows converted to per-row provenance (RS-5e,
  #14986). The STEP 1 sentence was this table's `source`, the field that carries the tier, for
  every row, so `? scientific_method_step(step_6, $D)` came back proved by *"Ask a question or
  make a statement that you can test by an experiment. This statement is called a hypothesis."*
  All eight spans are on the one NASA Space Place page and each row spells that locator out
  rather than inheriting it, so **the locator was never wrong here — only the span**, which is why this table's citation assertion never noticed.

  **Measured against the pre-change file by running it**: the step-1 sentence occurred **14**
  times in a full-table query (seven answers, twice each — once under `citations`, once under
  `steps`). It now occurs **twice**. The controls sentence went the same way, 14 to 2.

  **One row needs two sentences.** `step_2` is `define_variables_and_controls` and the page
  states variables and controls separately, so that row carries the variables sentence as
  `source` and the controls sentence as a **row-level `cites`**, which emits as a
  `corroborations` entry on that row's answer and no other. Established by running it.

  **The envelope is NOT a framing span here, and that is disclosed rather than dressed up.**
  Narrowed to what was checked: no sentence *outside the seven `Step N.` labels* frames the
  method, and the non-step text is **two** headings — "Steps in Scientific Method" and "One Way
  to Do Science...", the second of which the file's own header already named — plus a worked
  kitten example. An envelope `source` is required
  (an envelope carrying only `locator` and `trust` is rejected with
  `Lower(TableMissingProvenance)`), so the envelope keeps the step-1 sentence; every row
  overrides `source`, so it reaches no answer, and `step_1` carries its own copy. What this
  cascade requires is that the envelope not MIS-WARRANT a row, and that holds without inventing
  a second source page for a framing sentence this one does not contain.

  `facts_scientificmethodstep_e2e.rs`: 6 tests. **The whole-chain pin from #13934 installment
  4o is replaced, and what it protected is kept.** That pin ran from the step-1 `source`
  through all seven `corroborations` and closed on the corroborations `]`, so a fabricated
  `cites` could not be appended without reddening (#14735) — but those seven corroborations
  *were* the other six rows' evidence, hung off every answer because one envelope carried them.
  A chain pin cannot survive the chain being distributed. Each per-row needle now closes on
  that row's own `corroborations` — `[]` for six rows, the controls entry for `step_2` — so an
  appended `cites` still reddens, per row. Both directions are mutants and both are killed.

  **A surviving mutant is a finding**: renaming the `columns` line survived the first full
  harness run, because column names are positional and never reach the output — the same
  survivor found in `heredity-term`. Pinned, and the harness then killed 16 of 16.
