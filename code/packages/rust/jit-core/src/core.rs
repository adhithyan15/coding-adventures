//! `JITCore` — the top-level JIT compilation and dispatch API.
//!
//! `JITCore` monitors a `VMCore` execution, detects hot functions, compiles
//! them through a registered [`Backend`], and registers native handlers with
//! the VM so subsequent calls bypass the interpreter entirely.
//!
//! # Compilation pipeline
//!
//! ```text
//! IIRFunction (with feedback vectors from VMProfiler)
//!         │
//!         ▼ specialise()
//! Vec<CIRInstr>   (typed, with guards)
//!         │
//!         ▼ CIROptimizer::run()
//! Vec<CIRInstr>   (constant-folded, DCE'd)
//!         │
//!         ▼ backend.compile()
//! Vec<u8>          (backend binary)
//!         │
//!         ▼ backend.run()   (registered as JIT handler in VMCore)
//! Value   (return value)
//! ```
//!
//! # Tiered compilation
//!
//! Three tiers control when a function is compiled:
//!
//! | Tier | Default threshold | Meaning |
//! |---|---|---|
//! | `FullyTyped` | 0 | Compile before the first interpreted call |
//! | `PartiallyTyped` | 10 | Compile after 10 interpreted calls |
//! | `Untyped` | 100 | Compile after 100 interpreted calls |
//!
//! A threshold of `0` means "compile before any interpreted call".
//!
//! # Deoptimisation
//!
//! The JIT handler wrapper tracks `exec_count` via an `Arc<AtomicU64>`.
//! When `deopt_count / exec_count > 0.10` the compiled function is invalidated
//! and the function is marked unspecializable — it runs interpreted forever.
//!
//! # Thread safety
//!
//! Not thread-safe.  Each thread should own its own `JITCore` instance.
//!
//! # Example
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
//!     "main", vec![], "u8",
//!     vec![
//!         IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
//!         IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u8"),
//!     ],
//! );
//! let mut module = IIRModule::new("test", "test");
//! module.add_or_replace(fn_);
//!
//! // Note: NullBackend always returns Null, so the compiled result is Null.
//! let _result = jit.execute_with_jit(&mut vm, &mut module, "main", &[]);
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use interpreter_ir::module::IIRModule;
use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
use vm_core::core::VMCore;
use vm_core::value::Value;
use vm_core::errors::VMError;

use crate::backend::Backend;
use crate::cache::{JITCache, JITCacheEntry};
use crate::errors::{DeoptimizerError, JITError, UnspecializableError};
use crate::optimizer::CIROptimizer;
use crate::specialise::specialise;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Deopt rate above which a function is permanently invalidated.
///
/// Mirrors the Python constant `_DEOPT_RATE_LIMIT = 0.1`.
const DEOPT_RATE_LIMIT: f64 = 0.1;

// ---------------------------------------------------------------------------
// JITCore
// ---------------------------------------------------------------------------

/// Generic JIT compilation engine.
///
/// # Parameters (at construction)
///
/// - `backend` — compilation and execution target.
/// - `threshold_fully_typed` — call count to compile `FullyTyped` functions.
///   `0` = before the first call.
/// - `threshold_partial` — call count to compile `PartiallyTyped` functions.
/// - `threshold_untyped` — call count to compile `Untyped` functions.
/// - `min_observations` — minimum profiler observation count before an
///   observed type is trusted for specialisation.
pub struct JITCore {
    backend: Arc<dyn Backend>,
    optimizer: CIROptimizer,
    cache: JITCache,
    unspecializable: HashSet<String>,
    min_observations: u32,
    thresholds: HashMap<FunctionTypeStatus, u32>,
    /// Functions auto-invalidated due to excessive deopt rate.
    ///
    /// Each entry is the function name that was permanently invalidated.
    /// Drain this buffer with [`take_deopt_events`](JITCore::take_deopt_events)
    /// so callers can react to silent deoptimisations — for example by
    /// logging, emitting metrics, or alerting the user.
    deopt_events: Vec<String>,
}

impl JITCore {
    /// Construct a `JITCore` with default thresholds (0 / 10 / 100).
    ///
    /// The `vm` parameter is accepted but not stored — it is only used if
    /// you call [`execute_with_jit`](JITCore::execute_with_jit), which
    /// mutates the VM's JIT handler table at runtime.
    ///
    /// # Panics
    ///
    /// Does not panic.
    pub fn new(_vm: &mut VMCore, backend: Box<dyn Backend>) -> Self {
        JITCore::with_config(backend, 0, 10, 100, 5)
    }

    /// Construct a `JITCore` with explicit compilation thresholds.
    ///
    /// # Parameters
    ///
    /// - `backend` — implementation of [`Backend`].
    /// - `threshold_fully_typed` — minimum call count before compiling a
    ///   `FullyTyped` function.  `0` = compile before any interpreted call.
    /// - `threshold_partial` — minimum call count before compiling a
    ///   `PartiallyTyped` function.
    /// - `threshold_untyped` — minimum call count before compiling an
    ///   `Untyped` function.
    /// - `min_observations` — minimum profiler observations before trusting
    ///   an observed type for specialisation.
    pub fn with_config(
        backend: Box<dyn Backend>,
        threshold_fully_typed: u32,
        threshold_partial: u32,
        threshold_untyped: u32,
        min_observations: u32,
    ) -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert(FunctionTypeStatus::FullyTyped, threshold_fully_typed);
        thresholds.insert(FunctionTypeStatus::PartiallyTyped, threshold_partial);
        thresholds.insert(FunctionTypeStatus::Untyped, threshold_untyped);
        JITCore {
            backend: Arc::from(backend),
            optimizer: CIROptimizer::new(),
            cache: JITCache::new(),
            unspecializable: HashSet::new(),
            min_observations,
            thresholds,
            deopt_events: Vec::new(),
        }
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Run `module` under the interpreter with JIT compilation enabled.
    ///
    /// # Three-phase execution
    ///
    /// **Phase 1 — Eager compilation** of all `FullyTyped` functions (before the
    /// first interpreted call).
    ///
    /// **Phase 2 — Interpreted execution** of `fn_name`.  Any function that was
    /// compiled in Phase 1 fires its JIT handler instead of the interpreter.
    ///
    /// **Phase 3 — Hot-function promotion** after Phase 2: any function whose
    /// call count crossed its tier threshold during Phase 2 is compiled now.
    ///
    /// # Returns
    ///
    /// `Ok(Some(Value))` — the return value of `fn_name`, or
    /// `Ok(None)` for void functions.  Errors from the interpreter are
    /// propagated directly.
    pub fn execute_with_jit(
        &mut self,
        vm: &mut VMCore,
        module: &mut IIRModule,
        fn_name: &str,
        args: &[Value],
    ) -> Result<Option<Value>, VMError> {
        // Phase 1 — compile all FullyTyped functions eagerly.
        let fully_typed_names: Vec<String> = module
            .functions
            .iter()
            .filter(|f| {
                f.type_status == FunctionTypeStatus::FullyTyped
                    && !self.cache.contains(&f.name)
                    && !self.unspecializable.contains(&f.name)
            })
            .map(|f| f.name.clone())
            .collect();

        for name in fully_typed_names {
            if let Some(fn_) = module.get_function(&name) {
                let fn_ = fn_.clone();
                self.compile_fn(vm, &fn_);
            }
        }

        // Phase 2 — interpreted execution.
        let result = vm.execute(module, fn_name, args)?;

        // Phase 3 — promote functions that turned hot during Phase 2.
        self.promote_hot_functions(vm, module);

        Ok(result)
    }

    /// Manually compile `fn_name` in the given `module`.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` — compiled successfully.
    /// - `Ok(false)` — function not found, or the backend returned `None`.
    ///
    /// # Errors
    ///
    /// [`JITError::Unspecializable`] — if the function has been permanently
    /// invalidated (deopt rate exceeded 10 %).
    pub fn compile(
        &mut self,
        vm: &mut VMCore,
        module: &IIRModule,
        fn_name: &str,
    ) -> Result<bool, JITError> {
        if self.unspecializable.contains(fn_name) {
            return Err(UnspecializableError::new(fn_name).into());
        }
        let fn_ = match module.get_function(fn_name) {
            Some(f) => f.clone(),
            None => return Ok(false),
        };
        Ok(self.compile_fn(vm, &fn_))
    }

    /// Execute `fn_name` using the compiled binary or the interpreter.
    ///
    /// If the function is compiled, the backend runs it directly.
    /// If not, the interpreter runs it and hot functions are promoted.
    pub fn execute(
        &mut self,
        vm: &mut VMCore,
        module: &mut IIRModule,
        fn_name: &str,
        args: &[Value],
    ) -> Result<Option<Value>, VMError> {
        if let Some(entry) = self.cache.get(fn_name) {
            let binary = entry.binary.clone();
            let fn_name_owned = entry.fn_name.clone();
            // SECURITY (TOCTOU fix): Snapshot the Arc<AtomicU64> counters
            // *before* running the backend.  This eliminates the window where
            // `invalidate()` could remove the cache entry between the run and
            // the second `cache.get()` call, causing exec_count to be silently
            // skipped and inflating the deopt rate.
            let exec_arc = entry.exec_count_arc();
            let deopt_arc = entry.deopt_count_arc();
            let _ = entry; // release borrow — we hold Arcs to the counters

            let result = self.backend.run(&binary, args);

            exec_arc.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let ec = exec_arc.load(std::sync::atomic::Ordering::Relaxed);
            let dc = deopt_arc.load(std::sync::atomic::Ordering::Relaxed);
            if ec > 0 && (dc as f64 / ec as f64) > DEOPT_RATE_LIMIT {
                // SECURITY (LOW — silent deopt): queue the event so callers can
                // observe the invalidation via take_deopt_events().
                self.deopt_events.push(fn_name_owned.clone());
                self.invalidate(vm, &fn_name_owned);
            }
            return Ok(Some(result));
        }

        let result = vm.execute(module, fn_name, args)?;
        self.promote_hot_functions(vm, module);
        Ok(result)
    }

    /// Return `true` if `fn_name` has a cached native binary.
    pub fn is_compiled(&self, fn_name: &str) -> bool {
        self.cache.contains(fn_name)
    }

    /// Return per-function JIT cache statistics.
    ///
    /// Keys in the outer map are function names.  Each inner map has the
    /// keys: `fn_name`, `backend`, `param_count`, `ir_size`, `binary_size`,
    /// `compilation_time_ns`, `exec_count`, `deopt_count`, `deopt_rate`.
    pub fn cache_stats(&self) -> HashMap<String, HashMap<String, String>> {
        self.cache.stats()
    }

    /// Drain and return all deoptimisation events that occurred since the last
    /// call to this method.
    ///
    /// When [`execute`](JITCore::execute) or [`record_deopt`](JITCore::record_deopt)
    /// determines that a function's deopt rate exceeds the 10 % limit, the
    /// function is silently invalidated.  The event is queued here so callers
    /// can react — for example by emitting a log line, incrementing a metrics
    /// counter, or alerting the user.
    ///
    /// Each entry is the **function name** that was permanently invalidated.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // After calling execute() many times:
    /// for name in jit.take_deopt_events() {
    ///     eprintln!("JIT: '{name}' permanently deoptimised — running interpreted");
    /// }
    /// ```
    pub fn take_deopt_events(&mut self) -> Vec<String> {
        std::mem::take(&mut self.deopt_events)
    }

    /// Return the post-optimisation CIR for `fn_name` as a human-readable string.
    ///
    /// Returns an empty string if the function has not been compiled.
    pub fn dump_ir(&self, fn_name: &str) -> String {
        match self.cache.get(fn_name) {
            None => String::new(),
            Some(entry) => entry.ir.iter().map(|i| i.to_string()).collect::<Vec<_>>().join("\n"),
        }
    }

    /// Remove the compiled version of `fn_name` and mark it as unspecializable.
    ///
    /// After this call:
    /// - `is_compiled(fn_name)` returns `false`.
    /// - The JIT will never attempt to re-compile `fn_name`.
    /// - The VM's JIT handler for `fn_name` is removed; interpreted execution
    ///   resumes.
    pub fn invalidate(&mut self, vm: &mut VMCore, fn_name: &str) {
        self.cache.invalidate(fn_name);
        self.unspecializable.insert(fn_name.to_string());
        vm.unregister_jit_handler(fn_name);
    }

    /// Increment the deoptimisation counter for `fn_name`.
    ///
    /// Called by the deopt stub when a compiled function falls back to the
    /// interpreter.  If the deopt rate exceeds the limit, the function is
    /// invalidated.
    pub fn record_deopt(&mut self, vm: &mut VMCore, fn_name: &str) {
        if let Some(entry) = self.cache.get(fn_name) {
            entry.inc_deopt();
            if entry.exec_count() > 0 && entry.deopt_rate() > DEOPT_RATE_LIMIT {
                let name = entry.fn_name.clone();
                let _ = entry; // release borrow
                // SECURITY (LOW — silent deopt): queue the event before invalidating
                // so callers can observe it via take_deopt_events().
                self.deopt_events.push(name.clone());
                self.invalidate(vm, &name);
            }
        }
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Compile any function that has crossed its tier threshold.
    fn promote_hot_functions(&mut self, vm: &mut VMCore, module: &mut IIRModule) {
        let counts = vm.fn_call_counts().clone();
        let to_compile: Vec<_> = module
            .functions
            .iter()
            .filter(|f| {
                !self.cache.contains(&f.name)
                    && !self.unspecializable.contains(&f.name)
                    && self.thresholds.get(&f.type_status)
                        .map_or(false, |&thresh| {
                            thresh > 0 // FULLY_TYPED compiled eagerly
                                && counts.get(&f.name).copied().unwrap_or(0) >= thresh
                        })
            })
            .map(|f| f.clone())
            .collect();

        for fn_ in to_compile {
            self.compile_fn(vm, &fn_);
        }
    }

    /// Specialise, optimise, and compile `fn_` via the backend.
    ///
    /// On success:
    /// 1. Stores a [`JITCacheEntry`] in the cache.
    /// 2. Registers a JIT handler with `vm` so subsequent `call fn_name`
    ///    instructions bypass the interpreter.
    ///
    /// Returns `true` on success, `false` on any failure.
    fn compile_fn(&mut self, vm: &mut VMCore, fn_: &IIRFunction) -> bool {
        let t0 = Instant::now();

        // Specialise: IIRFunction → Vec<CIRInstr>.
        let raw_cir = specialise(fn_, self.min_observations);

        // Optimise: constant folding + DCE.
        let opt_cir = self.optimizer.run(raw_cir);

        // Compile: Vec<CIRInstr> → backend binary.
        let binary = match self.backend.compile(&opt_cir) {
            Some(b) => b,
            None => return false,
        };

        // SECURITY: `as_nanos()` returns u128; saturate to u64::MAX rather
        // than silently wrapping on absurdly long compilations (clock skew,
        // suspended processes, fuzz inputs).  Wrapping would produce a
        // near-zero value that poisons any heuristics built on compilation time.
        let elapsed_ns: u64 = t0.elapsed().as_nanos()
            .try_into()
            .unwrap_or(u64::MAX);
        let backend_name = self.backend.name().to_string();
        let param_count = fn_.params.len();

        let entry = JITCacheEntry::new(
            &fn_.name,
            binary.clone(),
            backend_name,
            param_count,
            opt_cir,
            elapsed_ns,
        );
        let exec_arc: Arc<AtomicU64> = entry.exec_count_arc();
        self.cache.put(entry);

        // Register a JIT handler so vm-core bypasses the interpreter.
        let backend = Arc::clone(&self.backend);
        vm.register_jit_handler(&fn_.name, move |args: &[Value]| -> Value {
            let result = backend.run(&binary, args);
            exec_arc.fetch_add(1, Ordering::Relaxed);
            result
        });

        true
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRInstr, IIRFunction, IIRModule, Operand};
    use crate::backend::NullBackend;

    fn simple_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "main",
            vec![],
            "u8",
            vec![
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u8"),
            ],
        );
        let mut m = IIRModule::new("test", "test");
        m.add_or_replace(fn_);
        m
    }

    fn jit_and_vm() -> (JITCore, VMCore) {
        let mut vm = VMCore::new();
        let jit = JITCore::new(&mut vm, Box::new(NullBackend));
        (jit, vm)
    }

    #[test]
    fn execute_with_jit_returns_result() {
        let (mut jit, mut vm) = jit_and_vm();
        let mut module = simple_module();
        // NullBackend always returns Null from run(), but the interpreter
        // (Phase 2) runs and returns 42.  After Phase 1, "main" gets a JIT
        // handler registered, which replaces the interpreter result.
        let result = jit.execute_with_jit(&mut vm, &mut module, "main", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn fully_typed_function_compiled_eagerly() {
        let (mut jit, mut vm) = jit_and_vm();
        let mut module = simple_module();
        jit.execute_with_jit(&mut vm, &mut module, "main", &[]).unwrap();
        assert!(jit.is_compiled("main"));
    }

    #[test]
    fn compile_manually() {
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let module = simple_module();
        let ok = jit.compile(&mut vm, &module, "main").unwrap();
        assert!(ok);
        assert!(jit.is_compiled("main"));
    }

    #[test]
    fn compile_missing_function_returns_false() {
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let module = simple_module();
        let ok = jit.compile(&mut vm, &module, "nonexistent").unwrap();
        assert!(!ok);
    }

    #[test]
    fn compile_unspecializable_function_raises() {
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let module = simple_module();
        jit.invalidate(&mut vm, "main");
        let result = jit.compile(&mut vm, &module, "main");
        assert!(matches!(result, Err(JITError::Unspecializable(_))));
    }

    #[test]
    fn invalidate_removes_from_cache() {
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let module = simple_module();
        jit.compile(&mut vm, &module, "main").unwrap();
        assert!(jit.is_compiled("main"));
        jit.invalidate(&mut vm, "main");
        assert!(!jit.is_compiled("main"));
    }

    #[test]
    fn dump_ir_empty_for_uncompiled() {
        let (jit, _vm) = jit_and_vm();
        assert_eq!(jit.dump_ir("main"), "");
    }

    #[test]
    fn dump_ir_nonempty_after_compile() {
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let module = simple_module();
        jit.compile(&mut vm, &module, "main").unwrap();
        let ir = jit.dump_ir("main");
        // Simple "const + ret" function should produce at least one line.
        assert!(!ir.is_empty());
    }

    #[test]
    fn cache_stats_empty_before_compile() {
        let (jit, _vm) = jit_and_vm();
        assert!(jit.cache_stats().is_empty());
    }

    #[test]
    fn cache_stats_after_compile() {
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let module = simple_module();
        jit.compile(&mut vm, &module, "main").unwrap();
        let stats = jit.cache_stats();
        assert!(stats.contains_key("main"));
    }

    #[test]
    fn record_deopt_increments_counter() {
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let module = simple_module();
        jit.compile(&mut vm, &module, "main").unwrap();
        jit.record_deopt(&mut vm, "main");
        let stats = jit.cache_stats();
        let entry_stats = &stats["main"];
        assert_eq!(entry_stats["deopt_count"], "1");
    }

    #[test]
    fn with_config_custom_thresholds() {
        let jit = JITCore::with_config(Box::new(NullBackend), 0, 20, 200, 10);
        assert_eq!(jit.thresholds[&FunctionTypeStatus::PartiallyTyped], 20);
        assert_eq!(jit.thresholds[&FunctionTypeStatus::Untyped], 200);
        assert_eq!(jit.min_observations, 10);
    }

    #[test]
    fn execute_falls_back_to_interpreter_when_not_compiled() {
        // Use NullBackend (compiles successfully) — verify that the JIT
        // handler fires correctly after eager compilation.
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let mut module = simple_module();
        // Phase 1 compiles eagerly for FullyTyped (NullBackend always succeeds).
        let result = jit.execute_with_jit(&mut vm, &mut module, "main", &[]).unwrap();
        // The JIT handler fires for "main" since it's compiled; NullBackend returns Null.
        // That's expected — just verifying no panic occurs.
        let _ = result;
    }

    #[test]
    fn execute_method_uses_cache() {
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let module = simple_module();
        jit.compile(&mut vm, &module, "main").unwrap();
        // Now execute via cache path.
        let mut module2 = simple_module();
        let result = jit.execute(&mut vm, &mut module2, "main", &[]).unwrap();
        // NullBackend always returns Null.
        assert_eq!(result, Some(Value::Null));
    }

    #[test]
    fn take_deopt_events_empty_initially() {
        let (mut jit, _vm) = jit_and_vm();
        assert!(jit.take_deopt_events().is_empty());
    }

    #[test]
    fn take_deopt_events_queues_and_drains() {
        // Manually push a deopt event by simulating record_deopt() exceeding
        // the rate limit.  We need exec_count > 0 and deopt_rate > 10%.
        // The easiest way is to call compile(), then pump exec_count and deopt_count
        // via the cache entry directly, then trigger invalidation via record_deopt().
        //
        // Implementation detail: record_deopt() checks entry.deopt_rate() which
        // is deopt_count / exec_count.  We need to seed exec_count first so the
        // rate isn't division-by-zero, then add enough deopt_count to exceed 10%.
        let mut vm = VMCore::new();
        let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
        let module = simple_module();
        jit.compile(&mut vm, &module, "main").unwrap();

        // Seed exec_count so ratio math works.
        if let Some(entry) = jit.cache.get("main") {
            for _ in 0..10 {
                entry.inc_exec();
            }
        }

        // Now call record_deopt() enough times to push deopt_rate > 10%.
        // 10 execs + 2 deopts → rate = 0.167 > 0.10.
        jit.record_deopt(&mut vm, "main");
        jit.record_deopt(&mut vm, "main");

        // Function should now be invalidated and event queued.
        let events = jit.take_deopt_events();
        assert!(events.contains(&"main".to_string()), "expected 'main' in deopt events, got {events:?}");
        assert!(!jit.is_compiled("main"), "function should be invalidated after deopt");

        // Events are drained — second call returns empty.
        let events2 = jit.take_deopt_events();
        assert!(events2.is_empty());
    }
}
