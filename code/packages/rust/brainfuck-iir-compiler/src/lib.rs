//! # brainfuck-iir-compiler — BF04: Brainfuck → InterpreterIR
//!
//! This crate bridges the Brainfuck language and the LANG pipeline's
//! generic interpreter ([`vm-core`](vm_core)).  A Brainfuck program is
//! compiled to a single-function [`IIRModule`](interpreter_ir::module::IIRModule)
//! and executed by the generic [`VMCore`](vm_core::core::VMCore).
//!
//! ## Pipeline
//!
//! ```text
//! Brainfuck source
//!        │
//!        ▼  brainfuck::parse_brainfuck()
//! GrammarASTNode
//!        │
//!        ▼  compile_to_iir() / compile_source()
//! IIRModule (one function: "main", FULLY_TYPED)
//!        │
//!        ▼  BrainfuckVM::run() → vm-core
//! Vec<u8>  (stdout bytes)
//! ```
//!
//! ## Key design decisions
//!
//! **Fixed registers** — Brainfuck has no functions or variables, so the
//! compiler uses four fixed register names (`ptr`, `v`, `c`, `k`) and
//! overwrites them in place.  This matches the natural register model of
//! `vm-core`'s frame.
//!
//! **FULLY_TYPED from birth** — Every instruction carries a concrete
//! `type_hint` (`"u8"` or `"u32"` or `"void"`).  The resulting
//! `IIRFunction` has `type_status = FullyTyped`, so a future JIT (BF05)
//! can tier up on the first call without waiting for profiling.
//!
//! **Structured loop shape** — Loops are emitted as:
//!
//! ```text
//! label   loop_N_start
//! load_mem c ptr u8
//! jmp_if_false c loop_N_end
//! ... body ...
//! jmp     loop_N_start
//! label   loop_N_end
//! ```
//!
//! This canonical form is what `ir-to-wasm-compiler` recognises for
//! structured-loop lowering (BF05).
//!
//! ## Public API
//!
//! | Item | Description |
//! |---|---|
//! | [`compile_source`] | Lex + parse + compile → `IIRModule` |
//! | [`compile_to_iir`] | Compile an existing AST → `IIRModule` |
//! | [`BrainfuckVM`] | High-level wrapper: source → `Vec<u8>` stdout |
//! | [`BrainfuckError`] | Brainfuck-level execution errors |
//!
//! ## Example
//!
//! ```
//! use brainfuck_iir_compiler::{BrainfuckVM, compile_source};
//!
//! // One-shot execution via the VM wrapper
//! let vm = BrainfuckVM::new(false, 30_000, None).unwrap();
//! let out = vm.run("+++.", b"").unwrap();
//! assert_eq!(out, vec![3u8]);
//!
//! // Or compile and inspect the IIR directly
//! let module = compile_source("++", "demo").unwrap();
//! assert_eq!(module.entry_point, Some("main".to_string()));
//! assert_eq!(module.functions[0].type_status,
//!            interpreter_ir::function::FunctionTypeStatus::FullyTyped);
//! ```

pub mod compiler;
pub mod errors;
pub mod vm;

pub use compiler::{compile_source, compile_to_iir};
pub use errors::BrainfuckError;
pub use vm::BrainfuckVM;
