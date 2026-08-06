//! mosaic-pkg-project-nav — userland Mosaic component package.
//!
//! This crate is intentionally empty. The package's real deliverables are
//! `ProjectNav.mil` / `ProjectNav.mll` / `ProjectNav.dark.msl` /
//! `ProjectNav.light.msl` under `src/` and the `mosaic-package.toml`
//! manifest at the package root — they describe one component
//! (ProjectNav) built entirely from UI29 kernel primitives.
//!
//! The Rust crate exists only so a smoke-test integration test can run the
//! source files through the mosmodel / moslayout / mosstyle compilers and
//! assert the package round-trips at the language-frontend layer. See
//! `tests/package_compiles.rs`.
