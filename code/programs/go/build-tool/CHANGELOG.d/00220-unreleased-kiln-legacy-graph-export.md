## Unreleased — Kiln legacy graph export

- **`-emit-kiln-graph <path>`.** Writes the `kiln-legacy-graph/1` document
  (KILN03 section 2): every package with per-OS BUILD file names (or `null`),
  per-OS direct deps, `dev_deps` (inverted test triggers), `repo_inputs` and
  `extra_toolchains`. Implies `-force` and `-language all`, honours
  `-validate-build-files`, and is deterministic. Go-only by design; the
  educational ports are frozen (KILN00 section 16).
- **Discovery across OSes.** New `discovery.DiscoverPackagesAllPlatforms`
  so a package whose only file is `BUILD_windows` (python/starlark-compiler)
  is not invisible on a Linux or macOS host. Normal discovery is unchanged.
- **Repository-input consumers declare paths.** Each table row gains a
  `paths` list mirroring its predicate, pinned by a test; the Mosaic guard is
  predicate-only and has none.
