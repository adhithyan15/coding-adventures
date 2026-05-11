//! # iir-codegen-adapters
//!
//! Unified IIR backend registry: compile an [`IIRModule`] to BEAM, WASM, JVM,
//! or CLR bytecode by name.
//!
//! ## Why this crate?
//!
//! LANG29 delivered four new Rust crates that each lower an [`IIRModule`] to a
//! target VM bytecode (`iir-to-beam`, `iir-to-wasm`, `iir-to-jvm-class-file`,
//! `iir-to-cil-bytecode`).  This crate provides the integration layer:
//!
//! - [`compile_iir(module, backend)`](compile_iir) — dispatch to any backend
//!   by name and get a single `Result<IIRBackendArtifact, _>` back.
//! - [`build_iir_codegen_registry()`] — register all four generators in a
//!   [`CodeGeneratorRegistry`] for pipeline-driver use cases that need to
//!   enumerate or dynamically select backends.
//! - [`list_iir_backends()`] — enumerate the four backend names.
//! - [`IIRBackendArtifact`] — closed enum wrapping all four artifact types.
//! - [`IIRAdapterError`] — unified error type for unknown backend, validation
//!   failure, and lowering failure.
//!
//! ## Quick start
//!
//! ```rust
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//! use iir_codegen_adapters::{compile_iir, list_iir_backends, IIRBackendArtifact};
//!
//! // Build: add(a: i32, b: i32) -> i32
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
//! };
//!
//! // Show available backends.
//! println!("Backends: {:?}", list_iir_backends());
//!
//! // Compile to each backend.
//! for backend in list_iir_backends() {
//!     let artifact = compile_iir(&module, backend).unwrap();
//!     println!("{}", artifact);   // e.g. "Wasm(types=1, functions=1)"
//! }
//! ```
//!
//! ## Pipeline-driver usage (with CodeGeneratorRegistry)
//!
//! ```rust
//! use iir_codegen_adapters::build_iir_codegen_registry;
//! use iir_to_wasm::IIRWasmCodeGenerator;
//!
//! let reg = build_iir_codegen_registry();
//! let any = reg.get("iir-wasm").unwrap();
//! let gen = any.downcast_ref::<IIRWasmCodeGenerator>().unwrap();
//! assert_eq!(gen.name(), "iir-wasm");
//! ```
//!
//! ## Module map
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`artifact`] | `IIRBackendArtifact` enum + accessors + `Display` |
//! | [`error`] | `IIRAdapterError` enum |
//! | [`registry`] | `build_iir_codegen_registry()` |
//! | [`dispatch`] | `compile_iir()` + `list_iir_backends()` |
//!
//! [`IIRModule`]: interpreter_ir::IIRModule
//! [`CodeGeneratorRegistry`]: codegen_core::codegen::CodeGeneratorRegistry

pub mod artifact;
pub mod dispatch;
pub mod error;
pub mod registry;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use artifact::IIRBackendArtifact;
pub use dispatch::{compile_iir, list_iir_backends};
pub use error::IIRAdapterError;
pub use registry::build_iir_codegen_registry;

// Re-export the CodeGeneratorRegistry so callers can use it without a separate
// codegen-core import.
pub use codegen_core::codegen::CodeGeneratorRegistry;
