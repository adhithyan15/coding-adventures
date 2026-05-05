//! `AOTCore` — the ahead-of-time compilation controller.
//!
//! `AOTCore` compiles an entire `IIRModule` to a `.aot` binary **before** the
//! program runs.  Unlike `JITCore` it:
//!
//! - Runs **static type inference** ([`infer_types`]) instead of consulting
//!   runtime profiler feedback.
//! - Compiles **every function** in the module (not just hot ones).
//! - Writes the result to a `.aot` binary rather than registering JIT handlers.
//! - Supports **cross-compilation** transparently via the pluggable `Backend`.
//!
//! # Compilation pipeline
//!
//! ```text
//! IIRModule
//!     │
//!     ├── for each IIRFunction:
//!     │       infer_types(fn)          → HashMap<String, String>
//!     │       aot_specialise(fn, env)  → Vec<CIRInstr>
//!     │       [CIROptimizer::run()]    → Vec<CIRInstr>  (opt levels 1 / 2)
//!     │       backend.compile(cir)     → Option<Vec<u8>>
//!     │
//!     │   compiled  →  fn_binaries
//!     │   failed    →  untyped_fns  (→ vm-runtime IIR table)
//!     │
//!     ├── link(fn_binaries)                       → (native_code, offsets)
//!     ├── vm_runtime.serialise_iir_table(untyped) → iir_table bytes (if any)
//!     └── snapshot::write(native_code, iir_table, entry_point) → Vec<u8>
//! ```
//!
//! # Optimization levels
//!
//! | Level | Effect |
//! |-------|--------|
//! | 0 | Raw specialised CIR — no optimization |
//! | 1 | Constant folding + dead-code elimination (`CIROptimizer`) |
//! | 2 | Same as 1 (AOT-specific passes reserved for future work) |
//!
//! # vm-runtime behaviour
//!
//! When a function cannot be compiled (backend returns `None`), it is emitted
//! into the IIR table section.  If a `VmRuntime` instance is provided, its
//! `library_bytes` are appended after the snapshot so a native linker can
//! embed an interpreter for those functions.
//!
//! # Example
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use jit_core::backend::NullBackend;
//! use aot_core::core::AOTCore;
//! use aot_core::snapshot::{read, HEADER_SIZE};
//!
//! let fn_ = IIRFunction::new(
//!     "main",
//!     vec![],
//!     "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")],
//! );
//! let mut module = IIRModule::new("test", "tetrad");
//! module.add_or_replace(fn_);
//!
//! let mut core = AOTCore::new(Box::new(NullBackend), None, 1);
//! let bytes = core.compile(&module).unwrap();
//! assert!(bytes.len() >= HEADER_SIZE);
//! assert_eq!(&bytes[0..4], b"AOT\x00");
//! ```

use std::time::Instant;

use interpreter_ir::module::IIRModule;
use interpreter_ir::function::IIRFunction;
use jit_core::backend::Backend;
use jit_core::optimizer::CIROptimizer;

use crate::errors::AOTError;
use crate::infer::infer_types;
use crate::link;
use crate::snapshot;
use crate::specialise::aot_specialise;
use crate::stats::AOTStats;
use crate::vm_runtime::VmRuntime;

// ---------------------------------------------------------------------------
// AOTCore
// ---------------------------------------------------------------------------

/// Ahead-of-time compilation controller.
pub struct AOTCore {
    backend: Box<dyn Backend>,
    vm_runtime: Option<VmRuntime>,
    optimization_level: u8,
    stats: AOTStats,
}

impl AOTCore {
    /// Create a new `AOTCore`.
    ///
    /// # Parameters
    ///
    /// - `backend` — a `Backend` that compiles `Vec<CIRInstr>` → `Option<Vec<u8>>`.
    /// - `vm_runtime` — optional pre-compiled vm-runtime library for untyped
    ///   functions.  Pass `None` when no runtime embedding is needed.
    /// - `optimization_level` — `0` = none, `1` = fold+DCE, `2` = fold+DCE+AOT.
    pub fn new(backend: Box<dyn Backend>, vm_runtime: Option<VmRuntime>, optimization_level: u8) -> Self {
        AOTCore {
            backend,
            vm_runtime,
            optimization_level,
            stats: AOTStats::new(optimization_level),
        }
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Compile the entire `module` to a `.aot` binary.
    ///
    /// # Returns
    ///
    /// `Ok(Vec<u8>)` — complete `.aot` binary (header + code section +
    /// optional IIR table).
    ///
    /// `Err(AOTError)` is currently never returned by the main pipeline
    /// (failures are handled gracefully by falling back to the IIR table),
    /// but the `Result` type is kept for future error paths.
    pub fn compile(&mut self, module: &IIRModule) -> Result<Vec<u8>, AOTError> {
        let t_start = Instant::now();

        let mut fn_binaries: Vec<(String, Vec<u8>)> = Vec::new();
        let mut untyped_fns: Vec<IIRFunction> = Vec::new();

        for iir_fn in &module.functions {
            match self.compile_fn(iir_fn) {
                Some(binary) => {
                    self.stats.functions_compiled += 1;
                    self.stats.total_binary_size += binary.len();
                    fn_binaries.push((iir_fn.name.clone(), binary));
                }
                None => {
                    self.stats.functions_untyped += 1;
                    untyped_fns.push(iir_fn.clone());
                }
            }
        }

        // Link compiled functions into one code section.
        let (native_code, offsets) = if fn_binaries.is_empty() {
            (Vec::new(), std::collections::HashMap::new())
        } else {
            link::link(&fn_binaries)
        };
        let ep_offset = link::entry_point_offset(&offsets, None) as u32;

        // Serialise untyped functions into the IIR table.
        let iir_table: Option<Vec<u8>> = if untyped_fns.is_empty() {
            None
        } else {
            let rt = self.vm_runtime.as_ref().cloned().unwrap_or_default();
            Some(rt.serialise_iir_table(&untyped_fns))
        };

        self.stats.compilation_time_ns +=
            t_start.elapsed().as_nanos() as u64;

        let mut data = snapshot::write(&native_code, iir_table.as_deref(), ep_offset);

        // Append pre-compiled vm-runtime library bytes when present.
        if iir_table.is_some() {
            if let Some(rt) = &self.vm_runtime {
                if !rt.is_empty() {
                    data.extend_from_slice(&rt.library_bytes);
                }
            }
        }

        Ok(data)
    }

    /// Return a snapshot of cumulative compilation statistics.
    pub fn stats(&self) -> &AOTStats {
        &self.stats
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Infer, specialise, optionally optimise, and compile one function.
    ///
    /// Returns `Some(bytes)` on success, `None` on any failure (backend
    /// returned `None`, or an exception-equivalent panic caught with
    /// `std::panic::catch_unwind`).
    fn compile_fn(&self, fn_: &IIRFunction) -> Option<Vec<u8>> {
        let inferred = infer_types(fn_);
        let mut cir = aot_specialise(fn_, Some(&inferred));

        if self.optimization_level >= 1 {
            cir = CIROptimizer::new().run(cir);
        }

        // Prefer the richer `compile_function` entry point so native
        // backends (ARM64, x86-64) get the param + return-type info they
        // need for ABI lowering.  IR-only backends (NullBackend, etc.) use
        // the default trait impl which falls back to `compile(&cir)`.
        let ctx = jit_core::backend::FunctionContext {
            name:        &fn_.name,
            params:      &fn_.params,
            return_type: &fn_.return_type,
        };
        self.backend.compile_function(&ctx, &cir)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;
    use jit_core::backend::NullBackend;

    fn tiny_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        );
        let mut m = IIRModule::new("test", "tetrad");
        m.add_or_replace(fn_);
        m
    }

    fn typed_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "main",
            vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
            "u8",
            vec![
                IIRInstr::new("add", Some("v".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u8"),
            ],
        );
        let mut m = IIRModule::new("typed_test", "tetrad");
        m.add_or_replace(fn_);
        m
    }

    #[test]
    fn compile_produces_aot_magic() {
        let mut core = AOTCore::new(Box::new(NullBackend), None, 1);
        let bytes = core.compile(&tiny_module()).unwrap();
        assert_eq!(&bytes[0..4], b"AOT\x00");
    }

    #[test]
    fn compile_header_size_at_least() {
        use crate::snapshot::HEADER_SIZE;
        let mut core = AOTCore::new(Box::new(NullBackend), None, 1);
        let bytes = core.compile(&tiny_module()).unwrap();
        assert!(bytes.len() >= HEADER_SIZE);
    }

    #[test]
    fn compile_typed_module() {
        let mut core = AOTCore::new(Box::new(NullBackend), None, 1);
        let bytes = core.compile(&typed_module()).unwrap();
        assert!(bytes.len() > 0);
    }

    #[test]
    fn stats_functions_compiled() {
        let mut core = AOTCore::new(Box::new(NullBackend), None, 1);
        core.compile(&tiny_module()).unwrap();
        assert_eq!(core.stats().functions_compiled, 1);
    }

    #[test]
    fn stats_functions_untyped_null_backend() {
        // NullBackend always succeeds (returns Some), so untyped count = 0.
        let mut core = AOTCore::new(Box::new(NullBackend), None, 1);
        core.compile(&tiny_module()).unwrap();
        assert_eq!(core.stats().functions_untyped, 0);
    }

    #[test]
    fn stats_time_positive_after_compile() {
        let mut core = AOTCore::new(Box::new(NullBackend), None, 1);
        core.compile(&tiny_module()).unwrap();
        // compilation_time_ns may be 0 on very fast machines; just check it
        // doesn't panic and is a valid u64.
        let _ = core.stats().compilation_time_ns;
    }

    #[test]
    fn empty_module_produces_valid_snapshot() {
        use crate::snapshot::read;
        let mut core = AOTCore::new(Box::new(NullBackend), None, 0);
        let empty = IIRModule::new("empty", "tetrad");
        let bytes = core.compile(&empty).unwrap();
        let snap = read(&bytes).unwrap();
        assert!(snap.native_code.is_empty());
        assert!(snap.iir_table.is_none());
    }

    #[test]
    fn round_trip_snapshot_readable() {
        use crate::snapshot::read;
        let mut core = AOTCore::new(Box::new(NullBackend), None, 1);
        let bytes = core.compile(&tiny_module()).unwrap();
        let snap = read(&bytes).unwrap();
        // NullBackend produces 1 sentinel byte per function.
        assert!(!snap.native_code.is_empty());
    }

    #[test]
    fn optimization_level_0_still_compiles() {
        let mut core = AOTCore::new(Box::new(NullBackend), None, 0);
        let bytes = core.compile(&tiny_module()).unwrap();
        assert_eq!(&bytes[0..4], b"AOT\x00");
    }

    #[test]
    fn optimization_level_2_compiles() {
        let mut core = AOTCore::new(Box::new(NullBackend), None, 2);
        let bytes = core.compile(&tiny_module()).unwrap();
        assert_eq!(&bytes[0..4], b"AOT\x00");
    }

    #[test]
    fn stats_optimization_level_stored() {
        let core = AOTCore::new(Box::new(NullBackend), None, 2);
        assert_eq!(core.stats().optimization_level, 2);
    }

    #[test]
    fn vm_runtime_empty_no_library_appended() {
        use crate::snapshot::read;
        let rt = VmRuntime::new(vec![]);
        let mut core = AOTCore::new(Box::new(NullBackend), Some(rt), 1);
        // NullBackend always compiles, so no untyped fns → no IIR table → no library.
        let bytes = core.compile(&tiny_module()).unwrap();
        let snap = read(&bytes).unwrap();
        assert!(snap.iir_table.is_none());
    }

    #[test]
    fn multiple_compile_calls_accumulate_stats() {
        let mut core = AOTCore::new(Box::new(NullBackend), None, 1);
        core.compile(&tiny_module()).unwrap();
        core.compile(&tiny_module()).unwrap();
        assert_eq!(core.stats().functions_compiled, 2);
    }
}
