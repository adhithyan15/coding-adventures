//! # ir-to-beam — BEAM bytecode backend for the Rust compiler IR.
//!
//! This crate lowers a [`compiler_ir::IrProgram`] to a BEAM (Erlang VM)
//! binary module.  It is the LANG20 `CodeGenerator<IrProgram, BEAMModule>`
//! adapter for Erlang's BEAM virtual machine.
//!
//! ## Pipeline
//!
//! ```text
//! IrProgram
//!   → validate_for_beam() / BEAMCodeGenerator::validate()
//!   → lower_ir_to_beam()  / BEAMCodeGenerator::generate()  → BEAMModule
//!   → encode_beam()                                          → Vec<u8>  (.beam file)
//! ```
//!
//! ## BEAM file format
//!
//! A `.beam` file is an IFF (Interchange File Format) container that holds
//! several typed chunks:
//!
//! | Chunk | Contents |
//! |-------|----------|
//! | `AtU8` | UTF-8 atom table |
//! | `Code` | Instruction stream (compact-term encoded operands) |
//! | `StrT` | String table (usually empty) |
//! | `ImpT` | Import table (external function references) |
//! | `ExpT` | Export table (publicly visible functions) |
//! | `LocT` | Local function table |
//! | `Attr` | Module attributes (BERT-encoded `[]` in v1) |
//! | `CInf` | Compiler info  (BERT-encoded `[]` in v1) |
//!
//! ## Supported IR opcodes (v1)
//!
//! LABEL, LOAD_IMM, ADD, ADD_IMM, SUB, AND, AND_IMM,
//! JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET, HALT, NOP, COMMENT.
//!
//! Unsupported (validation errors): LOAD_BYTE, STORE_BYTE, LOAD_WORD,
//! STORE_WORD, LOAD_ADDR, SYSCALL, CMP_EQ, CMP_NE, CMP_LT, CMP_GT.
//!
//! ## Quick start
//!
//! ```
//! use compiler_ir::{IrProgram, IrInstruction, IrOp};
//! use ir_to_beam::{BEAMCodeGenerator, encode_beam, validate_for_beam};
//! use codegen_core::codegen::CodeGenerator;
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
//!
//! let gen = BEAMCodeGenerator::new("mymod");
//! assert!(gen.validate(&prog).is_empty());
//! let module = gen.generate(&prog);
//! let bytes = encode_beam(&module);
//! assert_eq!(&bytes[0..4], b"FOR1");
//! ```

pub mod encoder;
pub mod backend;
pub mod codegen;

// ── Public re-exports ────────────────────────────────────────────────────────

pub use encoder::{
    BEAMModule,
    BEAMInstruction,
    BEAMImport,
    BEAMExport,
    BEAMOperand,
    BEAMTag,
    encode_beam,
    encode_compact_term,
};

pub use backend::{
    BEAMBackendConfig,
    BEAMBackendError,
    validate_for_beam,
    lower_ir_to_beam,
};

pub use codegen::BEAMCodeGenerator;
