## Unreleased — Kiln export covers every selector relation

- **`repo_inputs` now includes every package -> external input relation the
  selector uses**, not only `repositoryInputConsumers`: the shared/duplicate
  discovery, CI-gate, test-trigger, package-metadata, toolchain-detection,
  source-collection, graph/diff and per-case resolution, hashing-cache,
  validation and plan fixtures, plus the STP01, BF03, TE04, G2D02 corpora and
  the CV02 spec. The relations are built in `external_inputs.go` from the very
  variables and constants `selectPackageRoots` reads (the hardcoded seeds were
  hoisted into named constants and consumer lists; behaviour is unchanged), so
  they cannot drift. Real repo: 53 packages with inputs, 0 unknown. Consumers
  are keyed by package name, as the selector does (the .NET fronts are
  language `dotnet` although their table rows say csharp/fsharp).
- **Tests:** every consumer the selector can seed must export inputs or
  `repo_inputs_unknown`; every file a repository-input predicate accepts
  (real checkout plus synthetic names) must be covered by a declared path or
  glob; fixture-family globs cover the selector helpers.
- **Export hardening:** atomic temp-file write, symlink targets refused,
  empty package set and un-relativizable package paths are errors, per-OS
  `needs-toolchain` read for Starlark host packages (shell `BUILD_windows`),
  and the real-repo run fails if any relation names an undiscovered package.
