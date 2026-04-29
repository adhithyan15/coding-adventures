//! # ir-to-cil-bytecode — Lower `IrProgram` into CLR CIL bytecode.
//!
//! This crate translates a target-independent `IrProgram` into CIL method
//! bytecode for the Common Language Runtime (CLR), the virtual machine behind
//! .NET, Mono, and Xamarin.
//!
//! ## What is CIL?
//!
//! Common Intermediate Language (CIL) is the stack-based bytecode format used
//! by the CLR.  Unlike JVM bytecode (which encodes types in the opcode), CIL
//! infers types from the stack:
//!
//! ```text
//!     JVM:  iadd        (the "i" means int32)
//!     CIL:  add         (type inferred at JIT time)
//! ```
//!
//! CIL methods are stored in .NET PE/COFF files.  This crate emits raw CIL
//! byte bodies that a separate packager wraps in `.method` headers and
//! assembles into a complete `.dll` or `.exe`.
//!
//! ## Pipeline
//!
//! ```text
//! IrProgram
//!   → validate_for_clr()          — check constraints
//!   → lower_ir_to_cil_bytecode()  — emit CIL body bytes
//!   → CILProgramArtifact          — structured multi-method artifact
//!       ↓ (future) CLR packager   — wrap in PE/COFF file
//!       ↓ CLR simulator           — run directly
//! ```
//!
//! ## Module structure
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`builder`] | `CILBytecodeBuilder` — two-pass CIL assembler + encoding helpers |
//! | [`backend`] | `lower_ir_to_cil_bytecode`, `validate_for_clr`, `CILProgramArtifact` |
//! | [`codegen`] | `CILCodeGenerator` — LANG20 `CodeGenerator` adapter |
//!
//! ## Example
//!
//! ```
//! use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
//! use ir_to_cil_bytecode::backend::{lower_ir_to_cil_bytecode, validate_for_clr};
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(
//!     IrOp::LoadImm,
//!     vec![IrOperand::Register(1), IrOperand::Immediate(42)],
//!     1,
//! ));
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
//!
//! let errors = validate_for_clr(&prog);
//! assert!(errors.is_empty());
//!
//! let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
//! assert!(!artifact.methods[0].body.is_empty());
//! assert!(artifact.methods[0].body.contains(&0x2A)); // ret
//! ```

pub mod backend;
pub mod builder;
pub mod codegen;

// Re-export the most commonly needed types at the crate root.
pub use backend::{
    CILBackendConfig,
    CILBackendError,
    CILHelper,
    CILHelperSpec,
    CILMethodArtifact,
    CILProgramArtifact,
    CILTokenProvider,
    SequentialCILTokenProvider,
    lower_ir_to_cil_bytecode,
    validate_for_clr,
};
pub use builder::{
    CILBranchKind,
    CILBytecodeBuilder,
    CILOpcode,
    encode_i4,
    encode_ldc_i4,
    encode_ldarg,
    encode_ldloc,
    encode_metadata_token,
    encode_starg,
    encode_stloc,
};
pub use codegen::CILCodeGenerator;
