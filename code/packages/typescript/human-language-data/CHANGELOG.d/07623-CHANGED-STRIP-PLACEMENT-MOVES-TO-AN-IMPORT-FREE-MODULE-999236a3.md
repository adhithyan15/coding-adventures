### Changed — strip placement moves to an import-free module

- `filmstripBlockIndex`, `stripBlockIndex`, `letterBlockIndex`,
  `modelledPracticeBlockIndex` and `MODELLED_WRITING_STAGES` move, unchanged,
  from `figure-targets.ts` to a new `src/strip-placement.ts` with no imports.
  `figure-targets.ts` re-exports them, so every existing import still works
  and no book, figure or ledger output changes.
- They now take any `{ blocks }` whose blocks carry a `title`, an optional
  `writingStage` and optional `knowledge.introduces`
  (`StripPlacementLesson`); a `ParsedLesson` is one.
- Why: the language-ladder app kept its own copy of the rule, and the copy
  had drifted (it took the first Writing OR Script section, whichever came
  first), so 435 strips sat a section higher in the app than in the book.
  The app now imports this module directly; `figure-targets.ts` itself could
  not go into a browser bundle because it imports `node:path`.
