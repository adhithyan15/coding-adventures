//! Parrot — library surface for integration testing.
//!
//! A binary crate's internal modules are not accessible from the `tests/`
//! directory unless the crate also has a library target (`src/lib.rs`).
//! This file exists solely to re-export the types that integration tests need,
//! specifically [`prompt::ParrotPrompt`].
//!
//! # Why a lib target for a program?
//!
//! Rust's module system draws a hard boundary between a binary crate's
//! internal modules and integration tests in `tests/`. Integration tests
//! are compiled as separate crates that can only `use` items from the
//! crate's *library* target. By adding a `lib.rs` that re-exports
//! `ParrotPrompt`, we let tests import it without duplicating the struct.
//!
//! The `main.rs` binary and this `lib.rs` library share the same crate name
//! (`parrot`), but the binary is not accessible from tests — only the library
//! surface is.

/// The parrot-themed prompt implementation.
///
/// Re-exported here so integration tests can do:
/// ```rust,ignore
/// use parrot::prompt::ParrotPrompt;
/// ```
pub mod prompt;
