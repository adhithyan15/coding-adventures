//! # iir-linker — Static linker for `IIRModule`s (LANG33)
//!
//! This crate implements the static linker described in the LANG33 spec.  It
//! takes a slice of `IIRModule`s (each produced by a separate compilation unit
//! or loaded from disk) and merges them into one self-contained `IIRModule`
//! that can be handed to `vm-core.execute()` or any of the four native
//! backends.
//!
//! ## Quick start
//!
//! ```rust
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
//! use interpreter_ir::module_exports::{IIRExport, IIRImport};
//! use iir_linker::link;
//!
//! // --- module "math" exports "add" ---
//! let mut math = IIRModule::new("math", "twig");
//! math.entry_point = None;
//! math.add_or_replace(IIRFunction::new(
//!     "add",
//!     vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
//!     "i64",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")],
//! ));
//! math.exports.push(IIRExport::new("add"));
//!
//! // --- module "app" imports "add" from "math" ---
//! let mut app = IIRModule::new("app", "twig");
//! app.add_or_replace(IIRFunction::new(
//!     "main", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")],
//! ));
//! app.imports.push(IIRImport::new("math", "add", "any"));
//!
//! let merged = link(&[math, app]).unwrap();
//! // Both functions are now in the merged module.
//! assert!(merged.get_function("add").is_some());
//! assert!(merged.get_function("main").is_some());
//! // The merged module is self-contained — no imports left.
//! assert!(merged.imports.is_empty());
//! ```
//!
//! ## Module layout
//!
//! ```text
//! iir-linker/src/
//!   lib.rs       — re-exports + this module-level doc
//!   error.rs     — LinkError enum
//!   resolve.rs   — import/export resolution + type checking
//!   merge.rs     — function renaming + call rewriting + module merging
//!   linker.rs    — IIRLinker struct + link/link_strict/verify_imports entry points
//! tests/
//!   test_linker.rs — ≥ 30 integration tests
//! ```
//!
//! ## Design principles
//!
//! - **Language agnostic**: the linker operates on `IIRModule`s regardless of
//!   the source language.  Twig, NIB, Prolog, Brainfuck all get the same linker.
//! - **Fail loudly**: every error carries enough context to produce a helpful
//!   diagnostic without re-reading source files.
//! - **No side effects**: `link` is a pure function — no global state, no I/O.

pub mod error;
pub mod linker;
pub mod merge;
pub mod resolve;

// Re-export the most-used entry points and types.
pub use error::LinkError;
pub use linker::{link, link_strict, verify_imports, IIRLinker};
