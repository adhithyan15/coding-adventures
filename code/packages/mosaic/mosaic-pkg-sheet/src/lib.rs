//! mosaic-pkg-sheet — userland Mosaic component package.
//!
//! This crate is intentionally empty. The package's real deliverables
//! are `Sheet.mil` / `Sheet.mll` / `Sheet.dark.msl` / `Sheet.light.msl`
//! under `src/` and the `mosaic-package.toml` manifest at the package
//! root — they describe one component (Sheet) composed from
//! mosaic-pkg-grid's Grid and mosaic-pkg-toolkit's Select plus a
//! handful of kernel primitives.
//!
//! The Rust crate exists only so a smoke-test integration test can run
//! the source files through the mosmodel / moslayout / mosstyle
//! compilers and assert the package round-trips at the language-
//! frontend layer. See `tests/package_compiles.rs`.
