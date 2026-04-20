# Changelog

## 0.1.0 - 2026-04-20

- Add the phase-one benchmark-tool CLI.
- Drive CLI parsing from the repository's declarative Go `cli-builder` package
  with an embedded `benchmark-tool.json` spec for local use.
- Resolve manifest subject working directories from the manifest git root so
  local runs do not depend on launching the binary from the repository root.
- Support manifest validation, host diagnostics, command benchmark runs,
  summary/report generation, and simple result comparisons.
- Write raw samples, trial summaries, environment metadata, JSON summaries, and
  Markdown reports into self-contained result directories.
