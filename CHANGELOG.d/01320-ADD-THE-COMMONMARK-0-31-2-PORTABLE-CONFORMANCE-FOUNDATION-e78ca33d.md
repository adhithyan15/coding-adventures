### Add the CommonMark 0.31.2 portable conformance foundation

- Pinned the exact official 652-example CommonMark 0.31.2 corpus once with
  immutable provenance, raw-byte and semantic digests, and closed schemas.
- Added sixteen independent TE00 renderer vectors covering all nineteen node
  types without changing or crediting any native parser or renderer.
- Added a dependency-free bounded validator and adversarial tests for hostile
  JSON, decoded object graphs, corpus metadata, AST containment, and normative
  literal drift.
- Wired the validator and its dedicated falsification suite into the always-run
  repo-wide metadata-contract CI job, since this specs-and-scripts-only change
  intentionally selects no native package build.
- Corrected the follow-up migration inventory to include all sixteen live
  CommonMark and GFM package readers across seven languages.
- Rebased the existing foundation after the verified TypeScript GFM merge and
  restored the current one-active-implementation delivery ceiling.
- Fixed the observed Venture-only CI admission gap: both Node and pinned npm
  setup now run when workflow self-validation selects Venture acceptance even
  without a TypeScript package change. A failing Boolean admission regression
  precedes the fix; generated React and Electron install/test/build gates pass.
