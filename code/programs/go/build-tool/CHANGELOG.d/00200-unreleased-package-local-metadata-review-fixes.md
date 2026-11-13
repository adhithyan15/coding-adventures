## Unreleased — package-local metadata review fixes

- **Platform BUILD fronts no longer drop the owner.** Local-only packages
  were classified from the unfiltered path list while ordinary roots used the
  platform-filtered one, so a metadata change beside a `BUILD_windows` edit
  selected nothing on Linux and macOS (287 → 0 for `rust/ct-compare`, 556 → 0
  for `rust/bounded-json`). `selectPackageRoots` now returns both sets from
  the same list; the package is local on Linux/macOS and ordinary on Windows.
- **`README.md` is no longer metadata.** A Python dependent's
  `pip install -e ../<dep>` reads the dependency's `readme`; deleting
  `python/html-renderer/README.md` broke `python/programs/pipeline-visualizer`
  while only the renderer was selected. README changes propagate as before.
- **Only additions and edits can be local.** `gitdiff.GetChangedFilesWithStatus`
  reads `git diff --no-renames --name-status -z`; a deletion, rename (D + A),
  type change or unknown status propagates. `ClassifyChangedFiles` and
  `emitBuildPlanAgainst` take the statuses; a nil map keeps every metadata
  change ordinary.
- **Closed by type.** `CHANGELOG.d/` admits only direct `*.md` shards and
  `evidence/` only `.json`, `.cnf` and `.drat` (the extensions tracked
  today). `-validate-build-files` adds `ValidateNoExecutedMetadata`, which
  flags a tracked BUILD front, script, Cargo `path`/`build`, Rust
  `#[path]` or `include!` that visibly executes a metadata-class location
  (a best-effort tripwire; see above).
- `TestRepositoryInputConsumersPlan` is back to its original README
  expectation (the Mosaic package and the app that depends on it).
