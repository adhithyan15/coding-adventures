//! # iir-to-cil-bytecode — Lower `IIRModule` to `CILProgramArtifact`
//!
//! This crate translates an [`IIRModule`] (from `interpreter-ir`) into a
//! [`CILProgramArtifact`] (from `ir-to-cil-bytecode`) **without going through
//! the deprecated `compiler-ir` layer**.
//!
//! It is the direct CLR-backend path in the LANG pipeline:
//!
//! ```text
//! Language Frontend  →  IIRModule  →  iir-to-cil-bytecode  →  CILProgramArtifact
//!                                                                   ↓
//!                                                          CLR simulator / PE packager
//! ```
//!
//! ## Pipeline
//!
//! ```text
//! IIRModule
//!   → validate_iir_for_clr()   — pre-flight validation
//!   → lower_iir_to_cil()       — emit CIL body bytes per function
//!   → CILProgramArtifact       — structured multi-method artifact
//! ```
//!
//! ## Module structure
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`validate`] | `validate_iir_for_clr` — pre-flight checks |
//! | [`lower`]    | `IIRClrConfig`, `IIRClrError`, `lower_iir_to_cil` |
//! | [`codegen`]  | `IIRClrCodeGenerator` — `CodeGenerator` protocol adapter |
//!
//! ## Quick example
//!
//! ```
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//! use iir_to_cil_bytecode::{IIRClrConfig, lower_iir_to_cil, validate_iir_for_clr};
//!
//! // Build: add(a: i32, b: i32) -> i32
//! let fn_ = IIRFunction::new(
//!     "add",
//!     vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
//!     "i32",
//!     vec![
//!         IIRInstr::new("add", Some("v0".into()),
//!             vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
//!         IIRInstr::new("ret", None,
//!             vec![Operand::Var("v0".into())], "i32"),
//!     ],
//! );
//! let mut module = IIRModule::new("example", "tetrad");
//! module.entry_point = Some("add".into());
//! module.add_or_replace(fn_);
//!
//! let errors = validate_iir_for_clr(&module);
//! assert!(errors.is_empty());
//!
//! let artifact = lower_iir_to_cil(&module, &IIRClrConfig::default()).unwrap();
//! assert!(!artifact.methods[0].body.is_empty());
//! assert!(artifact.methods[0].body.contains(&0x2A)); // ret
//! ```

pub mod codegen;
pub mod lower;
pub mod validate;

// Re-export the most commonly used types at the crate root.
pub use validate::validate_iir_for_clr;
pub use lower::{IIRClrConfig, IIRClrError, lower_iir_to_cil};
pub use codegen::IIRClrCodeGenerator;

// Re-export the artifact types so callers don't need to depend on
// `ir-to-cil-bytecode` directly.
pub use ir_to_cil_bytecode::{CILProgramArtifact, CILMethodArtifact};
