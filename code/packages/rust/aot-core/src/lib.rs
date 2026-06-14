//! # aot-core — Ahead-of-time compilation engine for InterpreterIR
//!
//! `aot-core` compiles an entire [`IIRModule`](interpreter_ir::module::IIRModule)
//! to a self-contained `.aot` binary **before** the program runs.  It is the
//! compile-time counterpart of `jit-core`'s on-the-fly hot-path compilation.
//!
//! ## Compilation pipeline
//!
//! ```text
//! IIRModule
//!     │
//!     ├── for each IIRFunction:
//!     │       infer_types(fn)          → HashMap<String, String>  [infer]
//!     │       aot_specialise(fn, env)  → Vec<CIRInstr>            [specialise]
//!     │       CIROptimizer::run()      → Vec<CIRInstr>            [optimizer]
//!     │       backend.compile(cir)     → Option<Vec<u8>>
//!     │
//!     │   compiled  →  fn_binaries
//!     │   failed    →  untyped_fns  (→ vm-runtime IIR table)
//!     │
//!     ├── link(fn_binaries)                       → (native_code, offsets)
//!     ├── vm_runtime.serialise_iir_table(untyped) → iir_table bytes
//!     └── snapshot::write(native_code, iir_table) → Vec<u8>
//! ```
//!
//! ## Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`core`] | `AOTCore` — the top-level compilation controller |
//! | [`errors`] | `AOTError` — backend and snapshot error types |
//! | [`stats`] | `AOTStats` — compilation statistics |
//! | [`infer`] | `infer_types()` — static type inference pass |
//! | [`specialise`] | `aot_specialise()` — AOT analogue of jit-core's `specialise()` |
//! | [`link`] | `link()` + `entry_point_offset()` — binary concatenation |
//! | [`snapshot`] | `write()` + `read()` + `AOTSnapshot` — `.aot` file format |
//! | [`vm_runtime`] | `VmRuntime` — IIR table serialisation + pre-compiled library |
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use jit_core::backend::NullBackend;
//! use aot_core::core::AOTCore;
//! use aot_core::snapshot::read;
//!
//! // Build a tiny IIRModule with one function.
//! let fn_ = IIRFunction::new(
//!     "main", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")],
//! );
//! let mut module = IIRModule::new("hello", "tetrad");
//! module.add_or_replace(fn_);
//!
//! // Compile to .aot binary.
//! let mut core = AOTCore::new(Box::new(NullBackend), None, 2);
//! let bytes = core.compile(&module).unwrap();
//!
//! // Parse the binary back.
//! let snap = read(&bytes).unwrap();
//! assert_eq!(&bytes[0..4], b"AOT\x00");
//! assert!(!snap.native_code.is_empty());
//! ```

pub mod core;
pub mod errors;
pub mod infer;
pub mod link;
pub mod snapshot;
pub mod specialise;
pub mod stats;
pub mod vm_runtime;

// Convenient top-level re-exports for the most common types.
pub use core::AOTCore;
pub use errors::AOTError;
pub use stats::AOTStats;
pub use snapshot::AOTSnapshot;
pub use vm_runtime::VmRuntime;
