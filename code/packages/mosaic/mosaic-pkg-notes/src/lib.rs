//! mosaic-pkg-notes — userland Mosaic component package.
//!
//! This crate is intentionally empty. The package's real deliverables are
//! `Notes.mil` / `Notes.mll` / `Notes.dark.msl` / `Notes.light.msl` under
//! `src/` and the `mosaic-package.toml` manifest at the package root — they
//! describe one component (Notes) built from UI29 kernel primitives plus one
//! deliberate use of the legacy `Input` primitive (for its `multiline`
//! textarea support — see the package's README).
//!
//! The Rust crate exists only so a smoke-test integration test can run the
//! source files through the mosmodel / moslayout / mosstyle compilers and
//! assert the package round-trips at the language-frontend layer. See
//! `tests/package_compiles.rs`.
