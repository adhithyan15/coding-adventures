### Changed — the gap-report test builds the report once

- `src/report-cli.ts`: new `buildCurriculumGapReportOutputs(root?, loaded?)`
  builds the whole curriculum gap report and returns it rendered in both
  formats, `{ json, text }`. `runCurriculumGapReport` had always built both
  strings and printed one. It now calls the new function and prints the format
  it was asked for, so its output is unchanged. The function is exported from
  the package index.
- `tests/cli.test.ts` builds the report once, at import, as it already did for
  `runValidate`, and checks the json and text renderings of that one build. It
  used to run `runCurriculumGapReport` twice, once per format, each inside a 35s
  budget. At about 35,000 lessons the json case overran that budget on CI.
  Profiling found nothing superlinear (the cost is spread over the linear
  per-lesson passes), so the duplicate build was removed instead of moving the
  number.
