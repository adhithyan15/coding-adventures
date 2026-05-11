//! # twig-to-beam — end-to-end Twig → BEAM bytecode pipeline
//!
//! This crate is the "one function call" interface for compiling a Twig source
//! string all the way to a BEAM binary that a conformant BEAM VM can load and
//! execute.
//!
//! ## Pipeline
//!
//! ```text
//! Twig source (&str)
//!   │
//!   ▼  twig-ir-compiler :: compile_source
//! IIRModule   (type_hint = "any" on all instructions)
//!   │
//!   ▼  iir-type-checker :: infer_and_check
//! IIRModule   (type_hint inferred where possible: "i64", "bool", "f64", …)
//!   │
//!   ▼  iir-builtin-lowering :: lower_builtins
//! IIRModule   (call_builtin "+" → add, "=" → eq, …)
//!   │
//!   ▼  iir-to-beam :: lower_iir_to_beam
//! BEAMModule
//!   │
//!   ▼  iir-to-beam :: encode_beam     (re-exports ir-to-beam::encode_beam)
//! Vec<u8>     ← BEAM binary, starts with b"FOR1"
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use twig_to_beam::compile_twig_to_beam;
//!
//! let bytes = compile_twig_to_beam("(+ 1 2)", "arith").unwrap();
//! // The result is a BEAM binary (starts with the IFF magic "FOR1").
//! assert!(bytes.starts_with(b"FOR1"));
//! assert!(!bytes.is_empty());
//! ```
//!
//! ## Error handling
//!
//! Each pipeline stage has its own error type.  This crate wraps them all in
//! [`TwigToBeamError`] so callers only need one `match` arm per stage, not
//! one per crate.
//!
//! ```rust
//! use twig_to_beam::{compile_twig_to_beam, TwigToBeamError};
//!
//! match compile_twig_to_beam("undefined_name", "bad") {
//!     Err(TwigToBeamError::CompileError(e)) => {
//!         // Twig lex/parse/compile failed.
//!         eprintln!("compile error: {e}");
//!     }
//!     Err(TwigToBeamError::BeamError(e)) => {
//!         // BEAM lowering or encoding failed.
//!         eprintln!("BEAM error: {e}");
//!     }
//!     Err(e) => eprintln!("other error: {e}"),
//!     Ok(_) => unreachable!("undefined_name should fail"),
//! }
//! ```
//!
//! ## What "Twig" is
//!
//! Twig is the LANG project's Lisp-family language.  It looks like a minimal
//! Scheme: S-expressions, `define`, `lambda`, `if`, `let`, and a core set of
//! built-in functions (`+`, `-`, `*`, `/`, `=`, `<`, `>`, `cons`, `car`, `cdr`,
//! `null?`, `print`).  Recursion and higher-order functions are fully supported.
//!
//! Programs that use only numeric operations (no closures, no `cons`) compile
//! cleanly to BEAM bytecode.  Programs that use `call_builtin` operations that
//! the BEAM backend does not support (e.g. heap allocation, I/O dispatch) will
//! fail at the `BeamError` stage with an `UnsupportedOp` diagnostic.

pub mod error;
pub mod pipeline;

// ── Public API ────────────────────────────────────────────────────────────────

pub use error::TwigToBeamError;
pub use pipeline::compile_twig_to_beam;
