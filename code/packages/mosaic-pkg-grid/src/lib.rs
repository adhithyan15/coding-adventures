//! mosaic-pkg-grid — userland Mosaic component package.
//!
//! This crate is intentionally empty.  The package's real deliverables are
//! the `.mil` / `.mll` / `.msl` source files under `src/` and the
//! `mosaic-package.toml` manifest at the package root — they describe
//! three components (Grid, Cell, Column) composed entirely from UI29
//! kernel primitives.
//!
//! The Rust crate exists only so a smoke-test integration test can run
//! each component's three source files through the mosmodel /
//! moslayout / mosstyle compilers and assert the whole package round-
//! trips at the language-frontend layer.  See `tests/package_compiles.rs`.
