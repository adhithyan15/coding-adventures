//! # twig-to-wasm — end-to-end Twig → WebAssembly pipeline
//!
//! This crate is the "one function call" interface for compiling a Twig source
//! string all the way to a WASM 1.0 binary that any WebAssembly runtime can
//! load and execute.
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
//! IIRModule   (call_builtin "+" → add, "=" → cmp_eq, …)
//!   │
//!   ▼  pipeline :: fixup_control_flow_types  [local pass]
//! IIRModule   (ret/jmp/label "any" hints repaired: "void" or propagated type)
//!   │
//!   ▼  iir-to-wasm :: lower_iir_to_wasm
//! WasmModule
//!   │
//!   ▼  iir-to-wasm :: encode_module  (re-exports wasm-module-encoder)
//! Vec<u8>     ← WASM binary, starts with b"\x00asm"
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use twig_to_wasm::compile_twig_to_wasm;
//!
//! let bytes = compile_twig_to_wasm(
//!     "(define (add a b) (+ a b)) (add 1 2)",
//!     "arith",
//! ).unwrap();
//! // WASM magic: \0asm
//! assert!(bytes.starts_with(b"\x00asm"));
//! assert!(!bytes.is_empty());
//! ```
//!
//! ## Error handling
//!
//! All stage errors are wrapped in [`TwigToWasmError`]:
//!
//! ```rust
//! use twig_to_wasm::{compile_twig_to_wasm, TwigToWasmError};
//!
//! match compile_twig_to_wasm("undefined_name", "bad") {
//!     Err(TwigToWasmError::CompileError(e)) => {
//!         eprintln!("compile error: {e}");
//!     }
//!     Err(TwigToWasmError::WasmError(e)) => {
//!         eprintln!("WASM error: {e}");
//!     }
//!     Err(e) => eprintln!("other error: {e}"),
//!     Ok(_) => unreachable!("undefined_name should fail"),
//! }
//! ```
//!
//! ## What compiles vs. what fails
//!
//! Twig is a dynamically-typed Lisp.  At the IIR level, all operations start
//! as `call_builtin`.  After type inference + builtin lowering, *arithmetic*
//! operations (`+`, `-`, `*`, `/`, `=`, `<`, `>`) become typed IIR ops.
//!
//! Programs that use only numeric operations (define + call site) compile
//! cleanly.  Programs that reference non-arithmetic builtins (`nil`, `cons`,
//! `make_closure`, `apply_closure`, `global_get`/`set`) will encounter
//! `call_builtin` instructions that the WASM backend cannot lower, and the
//! compilation will return `WasmError(ValidationFailed(...))`.
//!
//! This is correct behavior — the WASM backend needs explicit host imports
//! for those operations, which this crate does not currently wire up.

pub mod error;
pub mod pipeline;

// ── Public API ────────────────────────────────────────────────────────────────

pub use error::TwigToWasmError;
pub use pipeline::compile_twig_to_wasm;
