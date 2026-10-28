### Fixed — the undeclared-reference test passes on Windows

`a_package_build_refuses_an_undeclared_package_reference` built its expected
manifest path with `join("mosaic-pkg-shell/mosaic-package.toml")`. On
Windows that displays as `…\mosaic-pkg-shell/mosaic-package.toml` (one `\`,
one `/`), while the error names `…\mosaic-pkg-shell\mosaic-package.toml`, so
the `contains` assertion failed on every Windows build that ran this crate's
tests. The test now joins one component at a time; the build itself was
always right.
