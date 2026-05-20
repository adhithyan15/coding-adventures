//! mosaic-pkg-dialog — userland Mosaic component package.
//!
//! This crate is intentionally empty.  The package's real deliverables are
//! the `.mil` / `.mll` / `.msl` source files under `src/` and the
//! `mosaic-package.toml` manifest at the package root — they describe a
//! single component (Dialog) composed entirely from UI29 kernel
//! primitives that exist in every Mosaic backend today (Box, Column, Text,
//! HostButton).
//!
//! The Rust crate exists only so a smoke-test integration test can run
//! Dialog's three source files through the mosmodel / moslayout / mosstyle
//! compilers and, on top of that, drive the per-backend artifact builder
//! across every backend the builder currently supports.  See
//! `tests/package_compiles.rs`.
