//! # iir-to-beam — IIR → BEAM bytecode backend.
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a [`ir_to_beam::BEAMModule`]
//! without going through the deprecated `compiler-ir` layer.
//!
//! ## Pipeline
//!
//! ```text
//! IIRModule
//!   → validate_for_beam()         — pre-flight check, returns Vec<String>
//!   → lower_iir_to_beam()         — two-pass lowering, returns BEAMModule
//!   → encode_beam()               — binary encoding, returns Vec<u8>
//! ```
//!
//! ## Why IIR → BEAM directly?
//!
//! The existing `ir-to-beam` crate lowers `compiler_ir::IrProgram` — a flat,
//! single-function IR with no type information.  `IIRModule` is richer: it has
//! multiple functions, named variables, static type hints, and a full comparison
//! operator set that maps cleanly to BEAM's conditional branch instructions.
//! This crate exploits that richness without retrofitting it through a deprecated
//! intermediate.
//!
//! ## Quick start
//!
//! ```rust
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//! use iir_to_beam::{validate_for_beam, lower_iir_to_beam, IIRBeamConfig, encode_beam};
//!
//! let fn_ = IIRFunction::new(
//!     "main",
//!     vec![],
//!     "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")],
//! );
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![fn_],
//!     entry_point: Some("main".into()),
//!     language: "test".into(),
//! };
//!
//! let errors = validate_for_beam(&module);
//! assert!(errors.is_empty());
//!
//! let config = IIRBeamConfig::new("demo");
//! let beam_module = lower_iir_to_beam(&module, &config).unwrap();
//! let bytes = encode_beam(&beam_module);
//! assert_eq!(&bytes[0..4], b"FOR1");
//! ```

pub mod codegen;
pub mod lower;
pub mod validate;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use validate::validate_for_beam;
pub use lower::{IIRBeamConfig, IIRBeamError, lower_iir_to_beam};
pub use codegen::IIRBeamCodeGenerator;

// Re-export the BEAM encoder types so callers do not need a separate
// `ir-to-beam` dependency just to encode the module to bytes.
pub use ir_to_beam::{BEAMModule, encode_beam};
