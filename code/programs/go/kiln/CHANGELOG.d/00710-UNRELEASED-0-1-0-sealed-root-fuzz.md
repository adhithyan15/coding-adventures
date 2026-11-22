## [0.1.0] - UNRELEASED

### Added (tests: the differential fuzzer covers the D43 `reshaped` taint)

- `model_test.go` bursts 22 to 27: a rename or `RENAME_EXCHANGE` of an own
  directory, of a directory the action did not create, of files it did not
  create, and of own regular files; absent probes at sandbox-absent paths
  (`/package.json`, `/opt/x`, `/var/x`) before and after the move; and failed
  relative probes through `..` (`gone/../x`) from a cwd that was renamed away.
- Two new oracles in `runDifferential`, both about a record that has no reason
  otherwise: once the model says a directory moved (`model.reshaped`), no D43
  sealed absence may be left standing, and a failed relative probe whose text
  walk reaches a missing component and then `..` must be refused as
  `unstable-path`. The classifier may taint more than the model (any move of an
  object the action did not create) but never less. Mutation-checked: removing
  the `settleReshaped` refusals or `noteMove`'s taint fails the seeded run.

### Changed (spec: KILN04 D43 and D40)

- D43 point 3 and D40 now say that D40's tmpfs containment (the stability
  rules skip contained paths unless `tmpLinked` or `tmpMoved`) also covers the
  scratch directories, because `underTmp` includes them, and why that is sound:
  each is a fresh per-step tmpfs mount like Home and Tmpdir.
- No classification semantics changed.
