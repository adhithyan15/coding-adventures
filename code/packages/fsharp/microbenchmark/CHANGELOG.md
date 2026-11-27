# Changelog

## 0.1.0

- Add F# paired collection, exact-rank analysis, and strict JSON replay.
- Add the native indexed-array CLI and deterministic fixture/property tests.
- Ignore blank revision environment values, reject blank CLI revisions, and reject negative array batch counts while allowing zero.
- Launch the installed-package consumer verifier explicitly through PowerShell from Windows BUILD files.
- Restore the installed consumer with a temporary NuGet.Config that maps CodingAdventures packages only to the packed local feed, plus an isolated cache and guarded temp cleanup.
- Apply the shared ASCII-whitespace workload-label rule and keep other Unicode whitespace valid.
- Declare the actual clock, revision environment, and standard-stream capabilities; bound direct analysis inputs to the documented pair limit.
- Invoke the installed consumer verifier with `pwsh` from the POSIX BUILD recipe.
- Replay the native 40-pair CLI report through the separately installed NuGet consumer.
