### Added — a package build refuses undeclared `pkg::` references

A package's layouts may reference with `pkg::P::C` only the package itself
or a dependency its `mosaic-package.toml` declares
(mosaic-package-publishing-v1 §10 Phase 1b). Before, any package on the
search path could be inlined. Its version was never checked, and the build
tool, which draws its edges from `[dependencies]`, never retested the
consumer when that package changed.

The check lives in `mosaic-package-resolver` (`UndeclaredPackageReference`).
This crate supplies the consumer manifest through
`LayoutPackageResolver::with_consumer`, by way of a new `consumer` field on
the internal `StyleCompositionOptions`. Where each composition gets it:

- `build_package` and the degradation analysis use the package being built;
- `compose_component_with_model_in_package`, which `mosaic-compile` calls
  with `--package-manifest`, uses the manifest it is given;
- dependency style collection, which resolves each dependency's layout as a
  consumer in its own right, uses that dependency's manifest.

Two paths pass no consumer manifest:

- `compose_component*`, a loose triple with no manifest;
- the project shell, whose layouts the per-component loop has already
  composed with the manifest.

Without one, the layout's own references are not checked, as before. The
references inside every package it inlines are still checked against those
packages' own manifests.

A failure surfaces as `BuildError::PackageReferenceError`. The message
names the manifest to fix and the line to add, for example
``add `mosaic-pkg-accent = "0.1.0"` to [dependencies] in …/mosaic-package.toml``.

New test: `a_package_build_refuses_an_undeclared_package_reference`. It
covers an undeclared reference at the top level, one in a dependency's own
layout, a loose triple left unchecked, and the
`compose_component_with_model_in_package` path.
