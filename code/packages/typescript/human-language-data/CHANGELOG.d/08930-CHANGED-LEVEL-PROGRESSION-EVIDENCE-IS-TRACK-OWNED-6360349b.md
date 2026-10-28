### Changed — level progression evidence is track-owned

- Extended each `tests/level-gate-attainment/<language>.json` owner to pin
  attained level, highest spine reach, and concise current evidence together.
- Replaced shared exact language lists and chronological advancement diaries in
  `level-gate.test.ts` and `levels.test.ts` with generic owner-driven invariants,
  removing the files touched by 82 and 38 of the latest 100 human-language
  commits from ordinary language progression work.
- Added fail-closed owner identity, file-type, symlink, case-fold, schema, CEFR,
  evidence-length, rank-consistency, mismatch-diagnostic, and source-shape
  regressions.
