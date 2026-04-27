//! # compiler-ir — General-purpose intermediate representation for the AOT compiler pipeline.
//!
//! This crate is the **IR layer** of a general-purpose ahead-of-time (AOT)
//! compiler pipeline. It provides:
//!
//! - `IrOp` — 25 opcodes covering constants, memory, arithmetic, comparison,
//!   control flow, system calls, and meta-operations.
//! - `IrOperand` — three operand kinds: virtual register, immediate, label.
//! - `IrInstruction` — an opcode + operands + unique monotonic ID.
//! - `IrDataDecl` — a data segment declaration (label, size, init byte).
//! - `IrProgram` — a complete program (instructions + data + entry label).
//! - `IdGenerator` — produces unique monotonic instruction IDs.
//! - `print_ir` — serializes an `IrProgram` to canonical text.
//! - `parse_ir` — deserializes canonical text back to an `IrProgram`.
//!
//! ## Design philosophy
//!
//! The IR is **general-purpose** — designed to serve as the compilation
//! target for any compiled language, not just Brainfuck. The current v1
//! instruction set is sufficient for Brainfuck; BASIC (the next planned
//! frontend) will add opcodes for multiplication, division, floating-point
//! arithmetic, and string operations.
//!
//! Key rules:
//! 1. Existing opcodes never change semantics — only new ones are appended.
//! 2. A new opcode is added only when a frontend needs it AND it cannot
//!    be efficiently expressed as a sequence of existing opcodes.
//! 3. All frontends and backends remain forward-compatible.
//!
//! ## IR characteristics
//!
//! - **Linear** — no basic blocks, no SSA, no phi nodes
//! - **Register-based** — infinite virtual registers (v0, v1, ...)
//! - **Target-independent** — backends map IR to physical ISA
//! - **Versioned** — `.version` directive in text format (v1 = Brainfuck subset)
//!
//! ## Example
//!
//! ```
//! use compiler_ir::types::{IrProgram, IrInstruction, IrDataDecl, IrOperand};
//! use compiler_ir::opcodes::IrOp;
//! use compiler_ir::printer::print_ir;
//! use compiler_ir::ir_parser::parse_ir;
//!
//! // Build a minimal program
//! let mut prog = IrProgram::new("_start");
//! prog.add_data(IrDataDecl { label: "tape".to_string(), size: 30000, init: 0 });
//! prog.add_instruction(IrInstruction::new(
//!     IrOp::LoadAddr,
//!     vec![IrOperand::Register(0), IrOperand::Label("tape".to_string())],
//!     0,
//! ));
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
//!
//! // Serialize and deserialize
//! let text = print_ir(&prog);
//! let parsed = parse_ir(&text).unwrap();
//! assert_eq!(parsed.instructions.len(), prog.instructions.len());
//! ```

pub mod opcodes;
pub mod types;
pub mod printer;
pub mod ir_parser;

// Re-export the most commonly used items at the crate root
pub use opcodes::{IrOp, parse_op};
pub use types::{IrOperand, IrInstruction, IrDataDecl, IrProgram, IdGenerator};
pub use printer::print_ir;
pub use ir_parser::parse_ir;
