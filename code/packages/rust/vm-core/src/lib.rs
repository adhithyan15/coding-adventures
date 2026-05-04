//! # vm-core — generic register interpreter for InterpreterIR.
//!
//! `vm-core` executes `IIRModule` programs produced by any language frontend.
//! It is the interpreter tier of the LANG pipeline — the engine that runs code
//! while the JIT warms up.
//!
//! ## What this crate provides
//!
//! - [`value::Value`] — the dynamic value type stored in registers
//!   (`Int`, `Float`, `Bool`, `Str`, `Null`)
//! - [`errors::VMError`] — all error variants the VM can raise
//! - [`frame::VMFrame`] — per-call-frame state (register file + IP + name map)
//! - [`profiler::VMProfiler`] — inline type observer; fills `IIRInstr` feedback slots
//! - [`builtins::BuiltinRegistry`] — named built-in function handlers
//! - [`core::VMCore`] — the public execution API (`execute`, `register_jit_handler`, …)
//!
//! ## Quick start
//!
//! ```
//! use vm_core::core::VMCore;
//! use vm_core::value::Value;
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//!
//! // Build a simple "return 42" program.
//! let fn_ = IIRFunction::new(
//!     "main", vec![], "u8",
//!     vec![
//!         IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
//!         IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
//!     ],
//! );
//! let mut module = IIRModule::new("hello", "test");
//! module.add_or_replace(fn_);
//!
//! let mut vm = VMCore::new();
//! let result = vm.execute(&mut module, "main", &[]).unwrap();
//! assert_eq!(result, Some(Value::Int(42)));
//! ```

pub mod branch_stats;
pub mod builtins;
pub mod core;
pub mod dispatch;
pub mod errors;
pub mod frame;
pub mod profiler;
pub mod trace;
pub mod value;

// Re-export the most commonly used items at the crate root.
pub use branch_stats::BranchStats;
pub use core::VMCore;
pub use errors::VMError;
pub use trace::VMTrace;
pub use value::Value;
