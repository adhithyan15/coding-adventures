### Fixed — eight A1 task shapes named Marathi instead of their own language

- Portuguese, Tamil, Kannada, Telugu, Malayalam, Italian, Bengali, and Sanskrit
  each carried the same eight strings copied from the Marathi A1 inventory:
  three listening prompts, one control criterion, and four vocabulary-and-grammar
  criteria.
- Every string now names the inventory's registered target language. Task counts,
  timings, scoring totals, pass thresholds, and sources are unchanged.
- A corpus-wide guard now rejects this copied phrase family in every task-shape
  inventory while allowing legitimate cross-language terminology such as Persian's
  Unicode note about Arabic yeh and kaf.
- Tracked by #15516 and #15522 under the assessment-contract work in #12215.
