//! mosaic-pkg-toolkit — Bootstrap-shaped Mosaic UI component library.
//!
//! This crate is intentionally empty. The package's real deliverables
//! are the `.mil` / `.mll` / `.msl` source files under `src/` and the
//! `mosaic-package.toml` manifest at the package root. Authors consume
//! the package by declaring it in their own `mosaic-package.toml`
//! dependencies and referencing its exported components from `.mll`
//! source.
//!
//! See `code/specs/mosaic-pkg-toolkit.md` for the architecture.
//!
//! The Rust crate exists only so the integration smoke test in
//! `tests/package_compiles.rs` can run each component's three source
//! files through the mosmodel / moslayout / mosstyle compilers and
//! assert the whole package round-trips at the language-frontend
//! layer.
//!
//! ## v0.1 PR-1 exports
//!
//! - **Button** — styled push button, slot-driven variant + size,
//!   wraps the kernel `HostButton`.
//! - **Alert** — colored info banner, slot-driven variant, optional
//!   inline dismiss button, composed from Box + Row + Text + If +
//!   HostButton.
//!
//! The full Tier-1 catalog (13 components) lands across follow-up
//! PRs. See `code/specs/mosaic-pkg-toolkit.md` §3.1.
