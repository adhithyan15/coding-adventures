## [0.1.0] - UNRELEASED

### Changed (tolerate the sandbox's own failed refused calls)

- The first real-build census found 31 of about 33 refused traces a run were
  `refused:clone3`: the filter answers `clone3` with ENOSYS so glibc falls back to
  `clone`, but strace prints the failed call and both readers refused any `clone3`.
  Every threaded tool (python, ruby, JVM, node, dotnet, the gcc driver) was
  unrecordable.
- A refused-class call is now TOLERATED (dropped, no event) when its result is the
  filter's own answer (`filterAnswered`): `clone3 = -1 ENOSYS`, every other refused
  name `= -1 EPERM`. A success, another errno, `= ?` or a call that never resumed
  stays `refused:<name>` / `unresumed-call`. `classifyCall` (table.go) is applied in
  `finishCall` (both readers) and in pass 2 of the model.
- The `<unfinished ...>` half of a refused name is no longer refused by either
  reader (changed together so the cross-check agrees); the resumed half decides.
- A `clone` whose only fault is a forbidden flag (`CLONE_UNTRACED`, `CLONE_NEW*`)
  and which failed with `-1 EPERM` is tolerated; a success or another errno stays
  `clone-flags`.
- `RecorderVersion` 3 -> 4: traces that were refused now parse, so every weak key
  changes. The key vectors do not depend on it (they pin a literal), so none
  were regenerated.
- KILN04 sections 4.3 item 5, 4.4, 4.5 and 4.7 say so.

### Added

- `TestClone3FailedWithENOSYSIsTheFilter`, `TestClone3NotTheFiltersAnswerIsRefused`,
  `TestClone3NeverResumedStaysUnresumed`, `TestRefusedNamesTolerateOnlyTheFiltersEPERM`,
  `TestCloneNamespaceFlagsRefusedByTheFilterAreTolerated` (both readers, via the
  cross-check hook), and fuzz seeds for `FuzzLineDiff` and `FuzzParseDiff`.
- `TestSandboxStraceClone3Enosys` (Linux CI, required in
  `ci/assert_sandbox_tests.py`): a raw `clone3` as the step against real strace
  parses with no refusal.
