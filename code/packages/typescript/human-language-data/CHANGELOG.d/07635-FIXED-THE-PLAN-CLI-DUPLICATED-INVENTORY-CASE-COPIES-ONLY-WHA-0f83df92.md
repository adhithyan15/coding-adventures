### Fixed — the plan CLI's duplicated-inventory case copies only what the plan reads

- `tests/plan-cli.test.ts` "counts a duplicated inventory once" timed out at
  its 30s budget on a loaded machine while its assertions passed. Its
  inventories-only fixture copied all of `core/`, `concepts/` and `data/`,
  about 27,000 files, and ran the plan twice over the copy; about 25,500 of
  those files are the modality manifest and the generated book, narration and
  book-generation ledgers, which the plan never opens.
- The fixture now copies through the same `readByPlan` filter the full-corpus
  fixture already uses (the first case's identical-output check vouches for
  that filter). The case dropped from about 7.1s to about 0.8s on an idle
  machine. No timeout was raised and no assertion changed.
