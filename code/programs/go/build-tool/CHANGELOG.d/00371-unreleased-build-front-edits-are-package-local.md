## Unreleased - in-place BUILD front edits no longer rebuild dependents (Rust)

- An in-place edit (Git status M) of a Rust package's own root BUILD front
  (`BUILD`, `BUILD_linux`, `BUILD_mac`, `BUILD_mac_and_linux`,
  `BUILD_windows`) is now package-local, like its changelog and evidence
  data: the package and its prerequisites are selected, its dependents and
  Cargo dev-dependency triggers are not. A BUILD-and-changelog-only change
  to eight Rust hub crates selected 803 packages; it now selects 35.
- The rule is gated: Rust only (closed list in `internal/pkgmeta`, with
  evidence), status M only, the platform-selected front only, shell BUILDs
  only, never a package whose BUILD another package's BUILD runs, and never
  across a `BUILD_TOOL_UNSUPPORTED:` transition
  (`gitdiff.GetChangedFilesWithStatus` asks Git which changed fronts add or
  remove a marker line and reports them with the new `StatusOrdinary`).
- `pkgmeta.IsPackageLocal` is now `IsMetadata || IsBuildFront`;
  `ScanBuildFrontReferences` finds BUILD fronts that other BUILDs run.
- `-validate-build-files` refuses a BUILD line that runs a BUILD front
  without naming the package directory, and any script that runs one.
- The neutral `metadata_policy: "package_local"` variant now treats exact
  root BUILD fronts of package-prefix packages as local;
  `package-metadata-selection-build-front.json` and `-lookalikes.json` pin
  it. No other implementation changes: they skip the variant.
- The front-invocation pattern also recognises interpreter paths and option
  tokens (`/bin/sh`, `bash -euo pipefail`, `env bash`, `cmd /c`, `pwsh -File`),
  `<` operands, `>`/`2>&1` terminators and a bare front as a command.
- The classifier accepts a BUILD front as local only with the new
  `StatusLocalFront` pseudo-status, which the host sets (`clearedBuildFronts`)
  only for fronts its veto pass examined and cleared; anything else fails
  closed to ordinary.
- `TestRepositoryBuildFrontInvocationsResolve` skips only when the
  `python/conduit` probe is absent, so sparse CI checkouts run it.
