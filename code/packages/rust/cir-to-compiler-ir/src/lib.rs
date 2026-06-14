//! # cir-to-compiler-ir — LANG21 bridge: CIR → `IrProgram`
//!
//! This crate connects the JIT specialisation layer (`jit-core`) to the
//! ahead-of-time compilation backends (`compiler-ir` / `ir-to-*`).
//!
//! ## Why is this bridge needed?
//!
//! The Tetrad pipeline has two representation worlds:
//!
//! ```text
//! [Tetrad source]
//!       │  (tetrad-lexer → parser → type-checker → compiler)
//!       ▼
//! [IIRModule]                   ← interpreter-ir (typed bytecode)
//!       │  (vm-core interprets + profiles)
//!       ▼
//! [Vec<CIRInstr>]               ← jit-core: specialised, type-resolved CIR
//!       │  (THIS CRATE: lower_cir_to_ir_program)    ← LANG21
//!       ▼
//! [IrProgram]                   ← compiler-ir: target-independent AOT IR
//!       │  (ir-to-wasm-compiler / ir-to-jvm-class-file / ir-to-intel-4004-compiler / …)
//!       ▼
//! [WasmModule / JVMClassArtifact / assembly text / …]
//! ```
//!
//! Without this bridge, the JIT output (`Vec<CIRInstr>`) cannot flow into
//! any of the AOT backends. LANG21 provides the one missing link.
//!
//! ## V1 limitations
//!
//! The v1 `IrOp` set does not include `MUL`, `DIV`, `OR`, `XOR`, `NOT`, or
//! any floating-point opcodes. CIR instructions that require these will cause
//! `lower_cir_to_ir_program` to return a `CIRLoweringError`. Adding these
//! opcodes to `compiler-ir` and the AOT backends is planned for a future LANG.
//!
//! ## Public API
//!
//! | Symbol | Description |
//! |---|---|
//! | [`CIRLoweringError`] | Error returned on unsupported or malformed CIR |
//! | [`validate_cir_for_lowering`] | Pre-lowering safety checks → `Vec<String>` |
//! | [`lower_cir_to_ir_program`] | Main lowering entry point → `Result<IrProgram>` |
//!
//! ## Example
//!
//! ```
//! use jit_core::{CIRInstr, CIROperand};
//! use cir_to_compiler_ir::{validate_cir_for_lowering, lower_cir_to_ir_program};
//!
//! let instrs = vec![
//!     CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(40)], "i32"),
//!     CIRInstr::new("const_i32", Some("y".to_string()), vec![CIROperand::Int(2)],  "i32"),
//!     CIRInstr::new("add_i32", Some("z".to_string()),
//!                   vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
//!     CIRInstr::new("ret_void", None::<String>, vec![], "void"),
//! ];
//!
//! // Validate first to get early, clear errors.
//! let validation_errors = validate_cir_for_lowering(&instrs);
//! assert!(validation_errors.is_empty());
//!
//! // Lower to IrProgram.
//! let prog = lower_cir_to_ir_program(&instrs, "_start").unwrap();
//! assert_eq!(prog.entry_label, "_start");
//! assert!(!prog.instructions.is_empty());
//! ```

pub mod errors;
pub mod lowering;
pub mod validator;

pub use errors::CIRLoweringError;
pub use lowering::lower_cir_to_ir_program;
pub use validator::validate_cir_for_lowering;
