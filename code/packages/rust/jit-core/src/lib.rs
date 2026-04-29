//! # jit-core — JIT compilation engine for InterpreterIR.
//!
//! `jit-core` monitors a `VMCore` execution, detects hot functions, compiles
//! them through a pluggable [`Backend`](backend::Backend), and registers
//! native handlers with the VM so subsequent calls bypass the interpreter.
//!
//! ## Pipeline
//!
//! ```text
//! Tetrad / BASIC / Python source
//!         │
//!         ▼  IIRModule  (interpreter-ir)
//!   ┌────────────┐
//!   │  vm-core   │  ← interprets IIR, fills profiling feedback slots
//!   └────────────┘
//!         │ VMProfiler → IIRInstr.observed_type
//!         ▼
//!   ┌────────────┐
//!   │  jit-core  │  ← this crate
//!   │            │
//!   │  specialise()  IIRFunction → Vec<CIRInstr>
//!   │  CIROptimizer  constant folding + DCE
//!   │  Backend::compile()  Vec<CIRInstr> → bytes
//!   └────────────┘
//!         │
//!         ▼  native binary (WASM / JVM / machine code / …)
//! ```
//!
//! ## Quick start
//!
//! ```
//! use jit_core::core::JITCore;
//! use jit_core::backend::NullBackend;
//! use vm_core::core::VMCore;
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//!
//! let mut vm = VMCore::new();
//! let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
//!
//! let fn_ = IIRFunction::new(
//!     "add",
//!     vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
//!     "u8",
//!     vec![
//!         IIRInstr::new("add", Some("v0".into()), vec![
//!             Operand::Var("a".into()), Operand::Var("b".into()),
//!         ], "u8"),
//!         IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u8"),
//!     ],
//! );
//! let mut module = IIRModule::new("hello", "tetrad");
//! module.add_or_replace(fn_);
//!
//! let _result = jit.execute_with_jit(&mut vm, &mut module, "add", &[]);
//! assert!(jit.is_compiled("add"));
//! ```
//!
//! ## Module structure
//!
//! | Module | Contents |
//! |---|---|
//! | [`errors`] | `JITError`, `DeoptimizerError`, `UnspecializableError` |
//! | [`cir`] | `CIRInstr`, `CIROperand` — typed compiler IR |
//! | [`backend`] | `Backend` trait, `NullBackend`, `EchoBackend` |
//! | [`optimizer`] | `CIROptimizer` — constant folding + DCE |
//! | [`specialise`] | `specialise()` — IIRFunction → Vec\<CIRInstr\> |
//! | [`cache`] | `JITCache`, `JITCacheEntry` |
//! | [`core`] | `JITCore` — the top-level API |

pub mod backend;
pub mod cache;
pub mod cir;
pub mod core;
pub mod errors;
pub mod optimizer;
pub mod specialise;

// Re-export the most commonly used types at the crate root.
pub use backend::Backend;
pub use cir::{CIRInstr, CIROperand};
pub use core::JITCore;
pub use errors::{DeoptimizerError, JITError, UnspecializableError};
pub use optimizer::CIROptimizer;
pub use specialise::specialise;
