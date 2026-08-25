### Changed - one verb per chapter, slice 4: chapter 21 (HL-C99)

- Split chapter 21, the book's **second and third paradigms**. They were still
  bundled three-cells-to-a-lesson while `-ar` had just been given five chapters,
  so the ramp contradicted itself mid-book.
- `comer` now owns a chapter with one `-er` cell per lesson -- `como`, `comes`,
  `come` -- and a review lesson that earns `ES-GRAMMAR-ER-PRESENT-SINGULAR`.
  The `yo` slot is taught as **free**: `-o` is the ending the learner already
  owns, and only the other two slots carry new information.
- **`-ir` deliberately did not get the same treatment.** In the singular its
  endings ARE the `-er` endings, so three per-cell lessons would have been
  padding. `vivir` declares all three CONJ3 cells in one lesson on the grounds
  that `maxNewGrammarCellsPerLesson` should count **new forms, not new slots**.
  Flagged for HL10 §5.2.
- Add a synthesis chapter where the three families and the two question words
  become a four-line exchange in which every word is one the reader built.
- Fix three forward references introduced while writing, one of them subtle:
  "*comer* **comes** from Latin *comedere*" made the English verb *comes*
  report as the Spanish `tú` form, six sequences before its lesson.
  `continuity.ts` guards this class of collision but does not list `comes`;
  filed as HL-C103.
- Spanish 69 -> **72 chapters**; old 22-69 -> 25-72. Paradigm-shaped tables
  **93 -> 91**; R2 reinforcement misses **1827 -> 1826**; forward references
  unchanged at **423**.

