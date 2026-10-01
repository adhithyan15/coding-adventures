### Changed — the plan CLI test copies only what the plan reads

- `tests/plan-cli.test.ts`: each case copied the whole curriculum into a temp
  directory and deleted it afterwards. It now leaves out the build products the
  plan never reads. That is 45,546 of 105,480 files:
  - per-track narration exports;
  - generated book chapters and figures;
  - the modality manifest;
  - the generated book, narration and figure hash ledgers;
  - the book-generation targets.
- The list was found by tracing every fs read `runCompletionPlan` makes over the
  real corpus.
- The first case now also plans the committed corpus and requires byte-identical
  output from the pruned copy. A list that goes stale fails there, not as a
  mysterious planner change.
- The file runs in ~158s instead of ~169s locally. Each `afterEach` deletes about
  43% fewer files, and that delete is what overran the hook budget on loaded CI
  runners.
