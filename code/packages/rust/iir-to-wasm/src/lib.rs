//! # iir-to-wasm — IIR → WebAssembly 1.0 backend.
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a [`wasm_types::WasmModule`]
//! without going through the deprecated `compiler-ir` layer.
//!
//! ## Pipeline
//!
//! ```text
//! IIRModule
//!   → validate_for_wasm()       — pre-flight check, returns Vec<String>
//!   → lower_iir_to_wasm()       — two-pass lowering, returns WasmModule
//!   → encode_module()           — binary encoding (wasm-module-encoder)
//! ```
//!
//! ## Why IIR → WASM directly?
//!
//! The existing `ir-to-wasm-compiler` crate operated on the deprecated
//! `compiler_ir::IrProgram` — a flat, single-function IR with no type
//! information.  `IIRModule` is richer: it has multiple functions, named
//! variables, static type hints, and a full operator set that maps cleanly to
//! WASM's typed numeric opcodes.  This crate exploits that richness without
//! retrofitting it through a deprecated intermediate.
//!
//! ## Key differences from the BEAM backend
//!
//! - Float constants (`Operand::Float`) ARE supported — WASM has native
//!   `f64.const` and `f32.const` instructions.
//! - All four WASM numeric types are used: `i32`, `i64`, `f32`, `f64`.
//! - Control flow uses a dispatch-loop pattern (one nested `block` per label,
//!   a `loop` for re-entry, `br_table` for dispatch).
//!
//! ## Quick start
//!
//! ```rust
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//! use iir_to_wasm::{validate_for_wasm, lower_iir_to_wasm, IIRWasmConfig};
//! use wasm_module_encoder::encode_module;
//!
//! let fn_ = IIRFunction::new(
//!     "add",
//!     vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
//!     "i32",
//!     vec![
//!         IIRInstr::new("add", Some("v0".into()),
//!             vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
//!         IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
//!     ],
//! );
//! let module = IIRModule {
//!     name: "calc".into(),
//!     functions: vec![fn_],
//!     entry_point: Some("add".into()),
//!     language: "test".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! let errors = validate_for_wasm(&module);
//! assert!(errors.is_empty());
//!
//! let config = IIRWasmConfig::new("calc");
//! let wasm_module = lower_iir_to_wasm(&module, &config).unwrap();
//! let bytes = encode_module(&wasm_module).unwrap();
//! assert!(bytes.starts_with(b"\x00asm"));
//! ```

pub mod codegen;
pub mod lower;
pub mod validate;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use codegen::IIRWasmCodeGenerator;
pub use lower::{IIRWasmConfig, IIRWasmError, lower_iir_to_wasm};
pub use validate::validate_for_wasm;

// Re-export the encoder so callers only need one dependency.
pub use wasm_module_encoder::encode_module;
