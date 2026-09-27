### Changed — remaining Indian corpus regressions have stable owners

- Split Bengali, Gujarati, Kannada, Marathi, Marwadi, Punjabi, Sanskrit, Tamil,
  Telugu, and Urdu corpus regressions into stable concern and chapter modules.
- Added deterministic per-language entrypoints that reject duplicate or unsafe
  owners and prevent the retired flat aggregates from returning.
- Taught the exam-inventory census to inspect explicitly loaded corpus owner
  modules, keeping Sanskrit and Telugu coverage pins visible after the split.
- Preserved all 82 existing assertions and one Vitest worker entry point per
  language so the ownership boundary remains CI-capacity neutral.
