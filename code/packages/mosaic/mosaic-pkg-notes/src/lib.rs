//! mosaic-pkg-notes — userland Mosaic component package.
//!
//! This crate is intentionally empty. The package's real deliverables are
//! `Notes.mil` / `Notes.mll` / `Notes.dark.msl` / `Notes.light.msl` under
//! `src/` and the `mosaic-package.toml` manifest at the package root — they
//! describe one component (Notes), composed since 0.3.0 from two shared
//! package components and a few kernel primitives:
//!
//! - `pkg::mosaic-pkg-toolkit::RecordList` draws the note list;
//! - `pkg::mosaic-pkg-draft-editor::DraftEditor` draws the title, the body
//!   (the kernel's multi-line `HostInput`, where Ctrl/Cmd+Enter saves) and
//!   the Save/Delete/Cancel actions;
//! - Notes' own `HostButton`, `Text` and `HostInput` draw "+ New note", the
//!   attach-to-task field and the empty-state hint.
//!
//! The Rust crate exists only so integration tests can run the source files
//! through the mosmodel / moslayout / mosstyle compilers
//! (`tests/package_compiles.rs`) and measure the composed package on the five
//! native backends (`tests/native_complete_gate.rs`).
