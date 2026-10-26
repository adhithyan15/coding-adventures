### Added — a package build checks its own dependency versions

`build_package` and the degradation analysis now check the package's
`[dependencies]` requirements against the packages its search path holds
(mosaic-package-publishing-v1 §7.4, Phase 1), before any component compiles.
A bare `"0.16.0"` accepts `>=0.16.0, <0.17.0` and `"=0.16.0"` only itself. A
miss fails with the new `BuildError::DependencyVersion`, whose message names
both packages, the requirement as a range, the version found and the manifest
to fix.

This uses `LayoutPackageResolver::check_dependencies`, the same by-name
locator the layout inliner uses, so the version checked is the version that
gets compiled. Requirements declared *by a dependency* are checked by the
inliner itself while it inlines from that dependency, and surface as
`BuildError::PackageReferenceError`. A declared dependency missing from the
search path is not an error here, as before.

New test: `a_package_build_checks_its_own_dependency_requirements` (caret and
exact, satisfied and not).
