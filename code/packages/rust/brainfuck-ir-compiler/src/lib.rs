//! # brainfuck-ir-compiler — Brainfuck AOT compiler frontend.
//!
//! This crate compiles Brainfuck ASTs into the general-purpose intermediate
//! representation (IR) defined by the `compiler-ir` crate.
//!
//! This is the Brainfuck-specific **frontend** of the AOT compiler pipeline.
//! It knows Brainfuck semantics (tape, cells, pointer, loops, I/O) and
//! translates them into target-independent IR instructions. It does NOT
//! know about RISC-V, ARM, ELF, or any specific machine target.
//!
//! ## Pipeline position
//!
//! ```text
//! Brainfuck source
//!        |
//!   [lexer] ← brainfuck crate
//!        |
//!   [parser] ← brainfuck crate → GrammarASTNode
//!        |
//!   [brainfuck-ir-compiler] ← THIS CRATE
//!        |
//!   IrProgram + SourceMapChain
//!        |
//!   [optimizer] (future)
//!        |
//!   [codegen-riscv] (future)
//! ```
//!
//! ## Outputs
//!
//! The compiler produces two outputs:
//! 1. An `IrProgram` containing the compiled IR instructions
//! 2. A `SourceMapChain` with `SourceToAst` and `AstToIr` segments filled in
//!
//! ## Register allocation
//!
//! Brainfuck needs very few virtual registers:
//!
//! | Register | Name      | Purpose                                  |
//! |----------|-----------|------------------------------------------|
//! | v0       | tape_base | base address of the tape                 |
//! | v1       | tape_ptr  | current cell offset (0-based index)      |
//! | v2       | temp      | temporary for cell values                |
//! | v3       | temp2     | temporary for bounds checks              |
//! | v4       | sys_arg   | syscall argument/return register         |
//! | v5       | max_ptr   | tape_size - 1 (for upper bounds check)   |
//! | v6       | zero      | constant 0 (for lower bounds check)      |

pub mod build_config;
pub mod compiler;

pub use build_config::{BuildConfig, debug_config, release_config};
pub use compiler::{compile, CompileResult};
