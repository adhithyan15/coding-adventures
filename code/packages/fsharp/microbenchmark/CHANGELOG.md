# Changelog

## 0.1.0

- Add F# paired collection, exact-rank analysis, and strict JSON replay.
- Add the native indexed-array CLI and deterministic fixture/property tests.
- Ignore blank revision environment values, reject blank CLI revisions, and reject negative array batch counts while allowing zero.
- Launch the installed-package consumer verifier explicitly through PowerShell from Windows BUILD files.
- Restore the installed consumer with an explicit temporary NuGet.Config and isolated package cache, avoiding Windows source-argument parsing differences.
- Apply the shared ASCII-whitespace workload-label rule and keep other Unicode whitespace valid.
