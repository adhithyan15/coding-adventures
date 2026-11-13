## Unreleased — package-local metadata no longer rebuilds dependents

- A package whose only changes are package-local metadata — its root
  `README.md`, `CHANGELOG.md`, `CHANGELOG.d/` entries,
  `required_capabilities.json`, `effect_certificate.json`,
  `assurance_report.json` or `evidence/` files — is now selected with its
  prerequisites but without its dependents or Cargo dev-dependency test
  triggers, the way a repository input consumer is. The package itself still
  rebuilds, because its BUILD may check the file. Any other change in the
  package (or a strict-glob declared source, a fixture seed, a forced or
  boundary-consumer root) propagates as before; `-force` and the hash
  fallback are unchanged. A capability-manifest-only change to
  `rust/bounded-json` now selects 1 package instead of 556.
- New `internal/pkgmeta` holds the closed, root-only, case-sensitive list
  and the repository evidence for each entry. `gitdiff.ClassifyChangedFiles`
  splits changed packages into propagating and local; `MapFilesToPackages`
  is now their union. `package_metadata.go` wires the local set into the
  detect plan and every platform override.
- `internal/graphdiff` replays the new `metadata_policy: "package_local"`
  diff-selection variant (`package-metadata-selection-*.json`), and changes
  to those cases seed the Dart, Go, Java, Kotlin and OCaml build-tool
  readers.
- `TestRepositoryInputConsumersPlan`'s README control now expects only the
  Mosaic package itself, not the app that depends on it.
