//! [`VMCore`] — the public API for the vm-core register interpreter.
//!
//! `VMCore` is a generic, language-agnostic register VM that executes
//! `InterpreterIR` (IIR) modules.  A language front-end produces an
//! `IIRModule` containing `IIRFunction` objects; calling `execute()` runs
//! the nominated entry-point function and returns the result.
//!
//! # Architecture
//!
//! ```text
//!              ┌─────────────┐
//!  IIRModule ──▶│  VMCore     │──▶  return Value
//!              │             │
//!              │ ┌─────────┐ │
//!              │ │ frame   │ │  ← VMFrame stack (one per call)
//!              │ │ stack   │ │
//!              │ └─────────┘ │
//!              │ ┌─────────┐ │
//!              │ │ opcode  │ │  ← handler lookup (O(1) match)
//!              │ │ table   │ │
//!              │ └─────────┘ │
//!              │ ┌─────────┐ │
//!              │ │profiler │ │  ← type observations → jit-core
//!              │ └─────────┘ │
//!              └─────────────┘
//! ```
//!
//! # Key configuration
//!
//! | Field | Default | Effect |
//! |-------|---------|--------|
//! | `u8_wrap` | `false` | Mask arithmetic results with `& 0xFF` (Tetrad mode) |
//! | `profiler_enabled` | `true` | Enable inline type profiling |
//! | `max_frames` | 512 | Maximum call-stack depth |
//!
//! # JIT integration
//!
//! Call `register_jit_handler(fn_name, handler)` to short-circuit the
//! interpreter for a compiled function.  The handler receives a slice of
//! argument values and returns the result.  The handler is invoked whenever
//! the interpreter encounters a `call` instruction naming that function,
//! before the interpreter would normally push a new frame.
//!
//! # Thread safety
//!
//! Not thread-safe.  Each thread should own its own `VMCore` instance.
//!
//! # Example
//!
//! ```
//! use vm_core::core::VMCore;
//! use vm_core::value::Value;
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//!
//! let fn_ = IIRFunction::new(
//!     "main",
//!     vec![],
//!     "u8",
//!     vec![
//!         IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
//!         IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
//!     ],
//! );
//! let mut module = IIRModule::new("test", "test");
//! module.add_or_replace(fn_);
//!
//! let mut vm = VMCore::new();
//! let result = vm.execute(&mut module, "main", &[]).unwrap();
//! assert_eq!(result, Some(Value::Int(42)));
//! ```

use std::collections::HashMap;
use interpreter_ir::module::IIRModule;
use crate::builtins::BuiltinRegistry;
use crate::dispatch::{run_dispatch_loop, DispatchCtx, JitHandlerMap, OpcodeHandler};
use crate::errors::VMError;
use crate::frame::VMFrame;
use crate::profiler::VMProfiler;
use crate::value::Value;

/// The vm-core register interpreter.
pub struct VMCore {
    // ------------------------------------------------------------------
    // Configuration (set at construction, usually not changed after)
    // ------------------------------------------------------------------

    /// When `true`, all arithmetic results are masked with `& 0xFF`.
    ///
    /// Required for Tetrad compatibility (8-bit register semantics).
    /// Disabled by default so other languages get full 64-bit precision.
    pub u8_wrap: bool,

    /// Toggle the inline type profiler.
    ///
    /// Disable for benchmarks where profiling overhead is unwanted.
    /// Default: `true`.
    pub profiler_enabled: bool,

    /// Hard limit on call-stack depth.
    ///
    /// Prevents stack overflow from runaway recursion — raises
    /// `VMError::FrameOverflow` instead.  Default: 512.
    pub max_frames: usize,

    /// Maximum number of distinct addresses in the flat memory map.
    ///
    /// Guards against unbounded `HashMap` growth from loops that call
    /// `store_mem` with ever-increasing addresses.  Default: 1_000_000.
    pub max_memory_entries: usize,

    /// Optional hard cap on the total number of instructions dispatched per
    /// `execute()` call.
    ///
    /// `None` — no limit (suitable for trusted, ahead-of-time compiled code).
    /// `Some(N)` — raises `VMError::Custom` after N instructions, preventing
    /// infinite-loop denial-of-service when running untrusted user code.
    pub max_instructions: Option<u64>,

    // ------------------------------------------------------------------
    // Runtime state
    // ------------------------------------------------------------------

    /// Active call-stack frames (top = last).
    frames: Vec<VMFrame>,

    /// Flat address-space memory (used by `load_mem` / `store_mem` / I/O).
    memory: HashMap<i64, Value>,

    /// Module-level global variables (LANG-FULL E6), keyed by name.
    /// `global_store("g", v)` writes here; `global_load("g")` reads it (a
    /// never-written global reads as `Value::Int(0)`, matching the zero-init the
    /// code-gen backends give their `_twig_globals` slots). This is distinct
    /// from `memory` (addressed by `i64`) and from the dynamic
    /// `call_builtin "global_set"/"global_get"` table the Twig front-end uses —
    /// these are the *lowered, typed* `global_load`/`global_store` IIR ops a
    /// statically-typed frontend (ALGOL procedures over an enclosing variable)
    /// emits directly.
    globals: HashMap<String, Value>,

    /// Heap of bounds-checked arrays (LANG-FULL E5). `alloc_array` pushes a new
    /// `Vec<Value>` and binds its 0-based index as the array *handle* (an
    /// `i64`); `array_get`/`array_set` index the `Vec` with a range check, and
    /// `array_len` reads its length. This is the interpreter's representation of
    /// the representation-agnostic IIR array — the managed backends use a native
    /// array, the static backends a length-prefixed flat block, but the VM keeps
    /// it simplest with a per-allocation `Vec` so distinct arrays never alias
    /// (unlike the single Brainfuck byte-tape, which is one flat space).
    arrays: Vec<Vec<Value>>,

    /// JIT handler registry.  When a `call` instruction names a function
    /// listed here, the handler is called instead of the interpreter.
    jit_handlers: JitHandlerMap,

    /// Language-specific opcode extensions.  Entries here shadow the
    /// standard opcode table, enabling languages to add or override any
    /// mnemonic.
    extra_opcodes: HashMap<String, OpcodeHandler>,

    /// Built-in function registry for `call_builtin` instructions.
    builtins: BuiltinRegistry,

    /// Per-function call count accumulator (distinct from `IIRFunction.call_count`
    /// which lives on the module).  Exposed via `fn_call_counts()`.
    fn_call_counts: HashMap<String, u32>,

    /// The inline type profiler.  `None` when `profiler_enabled = false`.
    profiler: Option<VMProfiler>,

    // ------------------------------------------------------------------
    // Metrics (read-only externally via accessor methods)
    // ------------------------------------------------------------------

    /// Total instructions dispatched since construction.
    metrics_instrs: u64,
    /// Total JIT handler invocations since construction.
    metrics_jit_hits: u64,

    // ── LANG17: branch + loop metrics ────────────────────────────────────
    /// Per-function, per-ip conditional branch taken/not-taken counts.
    ///
    /// Written by `jmp_if_true` / `jmp_if_false` handlers in the dispatch
    /// loop.  Read via `VMCore::branch_profile(fn_name, ip)`.
    branch_stats: HashMap<String, HashMap<usize, crate::branch_stats::BranchStats>>,

    /// Per-function, per-source-ip back-edge (loop iteration) counts.
    ///
    /// Written whenever any jump lands on an earlier instruction index.
    /// Read via `VMCore::loop_iterations(fn_name)`.
    loop_back_edge_counts: HashMap<String, HashMap<usize, u64>>,
}

impl VMCore {
    /// Create a `VMCore` with sensible defaults.
    pub fn new() -> Self {
        VMCore {
            u8_wrap: false,
            profiler_enabled: true,
            max_frames: 512,
            max_memory_entries: 1_000_000,
            max_instructions: None, // unlimited — trusted code path
            frames: Vec::new(),
            memory: HashMap::new(),
            globals: HashMap::new(),
            arrays: Vec::new(),
            jit_handlers: HashMap::new(),
            extra_opcodes: HashMap::new(),
            builtins: BuiltinRegistry::new(),
            fn_call_counts: HashMap::new(),
            profiler: Some(VMProfiler::new()),
            metrics_instrs: 0,
            metrics_jit_hits: 0,
            branch_stats: HashMap::new(),
            loop_back_edge_counts: HashMap::new(),
        }
    }

    /// Create a `VMCore` with `u8_wrap = true` (Tetrad 8-bit mode).
    pub fn with_u8_wrap() -> Self {
        let mut vm = VMCore::new();
        vm.u8_wrap = true;
        vm
    }

    // ------------------------------------------------------------------
    // JIT integration
    // ------------------------------------------------------------------

    /// Register a native handler for `fn_name`.
    ///
    /// When the interpreter encounters `call fn_name`, it calls this handler
    /// with the resolved argument values instead of pushing a new frame.
    /// Call `unregister_jit_handler` to revert to interpreted execution.
    pub fn register_jit_handler<F>(&mut self, fn_name: impl Into<String>, handler: F)
    where
        F: Fn(&[Value]) -> Value + Send + Sync + 'static,
    {
        self.jit_handlers.insert(fn_name.into(), Box::new(handler));
    }

    /// Remove the JIT handler for `fn_name`, reverting to interpreted execution.
    pub fn unregister_jit_handler(&mut self, fn_name: &str) {
        self.jit_handlers.remove(fn_name);
    }

    /// Return `true` if a JIT handler is registered for `fn_name`.
    pub fn is_jit_compiled(&self, fn_name: &str) -> bool {
        self.jit_handlers.contains_key(fn_name)
    }

    // ------------------------------------------------------------------
    // Language extension
    // ------------------------------------------------------------------

    /// Register a language-specific opcode handler.
    ///
    /// Entries here shadow the standard opcode table.  Use this to add
    /// language-specific opcodes (e.g. `"tetrad.move"`) without subclassing.
    pub fn register_opcode(&mut self, op: impl Into<String>, handler: OpcodeHandler) {
        self.extra_opcodes.insert(op.into(), handler);
    }

    /// Return a mutable reference to the built-in registry.
    pub fn builtins_mut(&mut self) -> &mut BuiltinRegistry {
        &mut self.builtins
    }

    // ------------------------------------------------------------------
    // Execution
    // ------------------------------------------------------------------

    /// Execute the named function in `module` with the given arguments.
    ///
    /// # Arguments
    ///
    /// - `module` — the program to run; its `IIRFunction` call counts and
    ///   profiling feedback slots are mutated in place.
    /// - `fn_name` — name of the entry-point function.
    /// - `args` — argument values, mapped to the first N parameters.
    ///
    /// # Returns
    ///
    /// The value produced by the outermost `ret` instruction, or `None` if
    /// the function ends with `ret_void`.
    pub fn execute(
        &mut self,
        module: &mut IIRModule,
        fn_name: &str,
        args: &[Value],
    ) -> Result<Option<Value>, VMError> {
        // Find the entry-point function.
        let fn_idx = module.functions.iter().position(|f| f.name == fn_name)
            .ok_or_else(|| VMError::UnknownOpcode(format!("function {fn_name:?}")))?;

        let params = module.functions[fn_idx].params.clone();
        let reg_count = module.functions[fn_idx].register_count;

        // Build the root frame.
        let mut root_frame = VMFrame::for_function(fn_name, &params, reg_count, None);
        for (i, arg) in args.iter().enumerate().take(params.len()) {
            root_frame.registers[i] = arg.clone();
        }

        // Push root frame and run.
        self.frames.clear();
        self.frames.push(root_frame);

        // Count the entry-point call (handle_call only counts callee functions
        // reached via `call` instructions, not the top-level execute() entry).
        *self.fn_call_counts.entry(fn_name.to_string()).or_insert(0) += 1;

        let mut profiler = if self.profiler_enabled {
            Some(VMProfiler::new())
        } else {
            None
        };

        // Reset the per-execute instruction counter so max_instructions applies
        // to this invocation, not cumulative lifetime instructions.
        let instrs_before = self.metrics_instrs;

        let mut ctx = DispatchCtx {
            frames: &mut self.frames,
            module_fns: &mut module.functions,
            builtins: &self.builtins,
            memory: &mut self.memory,
            globals: &mut self.globals,
            arrays: &mut self.arrays,
            u8_wrap: self.u8_wrap,
            max_frames: self.max_frames,
            max_memory_entries: self.max_memory_entries,
            // Use saturating_add so that a near-u64::MAX metrics_instrs + a
            // large limit does not overflow (which would panic in debug builds
            // or wrap silently in release builds, disabling the limit).
            max_instructions: self.max_instructions.map(|limit| instrs_before.saturating_add(limit)),
            fn_call_counts: &mut self.fn_call_counts,
            metrics_instrs: &mut self.metrics_instrs,
            metrics_jit_hits: &mut self.metrics_jit_hits,
            branch_stats: &mut self.branch_stats,
            loop_back_edge_counts: &mut self.loop_back_edge_counts,
            tracer: None,
        };

        // extra_opcodes and jit_handlers are passed as separate references
        // so that handlers can also borrow ctx mutably without conflict.
        let result = run_dispatch_loop(
            &mut ctx,
            &self.extra_opcodes,
            &self.jit_handlers,
            &mut profiler,
        )?;
        self.profiler = profiler;
        Ok(result)
    }

    /// Execute `fn_name` and record a per-instruction `VMTrace` for each
    /// dispatched instruction.
    ///
    /// Returns `(result, traces)`.  Traces are in execution order.
    ///
    /// **Overhead warning**: this path clones the register file twice per
    /// instruction.  Use only for debugging, test assertions, and reproducer
    /// generation — never on the hot measurement path.
    pub fn execute_traced(
        &mut self,
        module: &mut IIRModule,
        fn_name: &str,
        args: &[Value],
    ) -> Result<(Option<Value>, Vec<crate::trace::VMTrace>), VMError> {
        let fn_idx = module.functions.iter().position(|f| f.name == fn_name)
            .ok_or_else(|| VMError::UnknownOpcode(format!("function {fn_name:?}")))?;

        let params = module.functions[fn_idx].params.clone();
        let reg_count = module.functions[fn_idx].register_count;

        let mut root_frame = VMFrame::for_function(fn_name, &params, reg_count, None);
        for (i, arg) in args.iter().enumerate().take(params.len()) {
            root_frame.registers[i] = arg.clone();
        }

        self.frames.clear();
        self.frames.push(root_frame);

        // Count the entry-point call (same as execute()).
        *self.fn_call_counts.entry(fn_name.to_string()).or_insert(0) += 1;

        let mut profiler = if self.profiler_enabled {
            Some(VMProfiler::new())
        } else {
            None
        };

        let instrs_before = self.metrics_instrs;
        let mut traces: Vec<crate::trace::VMTrace> = Vec::new();

        let mut ctx = DispatchCtx {
            frames: &mut self.frames,
            module_fns: &mut module.functions,
            builtins: &self.builtins,
            memory: &mut self.memory,
            globals: &mut self.globals,
            arrays: &mut self.arrays,
            u8_wrap: self.u8_wrap,
            max_frames: self.max_frames,
            max_memory_entries: self.max_memory_entries,
            max_instructions: self.max_instructions.map(|limit| instrs_before.saturating_add(limit)),
            fn_call_counts: &mut self.fn_call_counts,
            metrics_instrs: &mut self.metrics_instrs,
            metrics_jit_hits: &mut self.metrics_jit_hits,
            branch_stats: &mut self.branch_stats,
            loop_back_edge_counts: &mut self.loop_back_edge_counts,
            tracer: Some(&mut traces),
        };

        let result = run_dispatch_loop(
            &mut ctx,
            &self.extra_opcodes,
            &self.jit_handlers,
            &mut profiler,
        )?;
        self.profiler = profiler;
        Ok((result, traces))
    }

    // ------------------------------------------------------------------
    // Metrics / introspection
    // ------------------------------------------------------------------

    /// Total instructions dispatched since construction.
    pub fn metrics_instrs(&self) -> u64 {
        self.metrics_instrs
    }

    /// Total JIT handler invocations since construction.
    pub fn metrics_jit_hits(&self) -> u64 {
        self.metrics_jit_hits
    }

    /// Per-function call counts accumulated by the interpreter.
    ///
    /// Note: JIT handler calls are counted separately in `metrics_jit_hits`,
    /// not in this map.
    pub fn fn_call_counts(&self) -> &HashMap<String, u32> {
        &self.fn_call_counts
    }

    /// Total profiling observations recorded in the last `execute()` call.
    pub fn total_observations(&self) -> u64 {
        self.profiler.as_ref().map_or(0, |p| p.total_observations())
    }

    // ── LANG17: branch / loop / hot-function introspection ────────────────

    /// Names of functions called at least `threshold` times.
    ///
    /// Useful for identifying hot functions that the JIT should prioritise.
    /// The list is sorted by call count descending.
    pub fn hot_functions(&self, threshold: u32) -> Vec<String> {
        let mut pairs: Vec<(&String, u32)> = self.fn_call_counts
            .iter()
            .filter(|(_, &count)| count >= threshold)
            .map(|(name, &count)| (name, count))
            .collect();
        pairs.sort_by(|a, b| b.1.cmp(&a.1));
        pairs.into_iter().map(|(name, _)| name.clone()).collect()
    }

    /// Branch taken/not-taken statistics for the conditional at instruction
    /// index `ip` in function `fn_name`.
    ///
    /// Returns `None` if that instruction has never been observed (function
    /// not called, or non-conditional instruction at `ip`).
    pub fn branch_profile(
        &self,
        fn_name: &str,
        ip: usize,
    ) -> Option<&crate::branch_stats::BranchStats> {
        self.branch_stats.get(fn_name)?.get(&ip)
    }

    /// Back-edge (loop iteration) counts for all back-edges in `fn_name`.
    ///
    /// Keys are the instruction index of the jump instruction.  Values are
    /// the number of times that jump was executed as a backward jump.
    ///
    /// Returns an empty map if the function has never been called or has no
    /// back-edges.
    pub fn loop_iterations(&self, fn_name: &str) -> HashMap<usize, u64> {
        self.loop_back_edge_counts
            .get(fn_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Reference to the full branch-stats map (all functions).
    pub fn all_branch_stats(
        &self,
    ) -> &HashMap<String, HashMap<usize, crate::branch_stats::BranchStats>> {
        &self.branch_stats
    }

    /// Reference to the full loop-back-edge-counts map (all functions).
    pub fn all_loop_back_edge_counts(&self) -> &HashMap<String, HashMap<usize, u64>> {
        &self.loop_back_edge_counts
    }

    /// Zero all metrics accumulators and per-instruction feedback.
    ///
    /// Useful in REPL scenarios where the user wants a clean profiling
    /// snapshot after warm-up without recreating the VM.
    pub fn reset_metrics(&mut self) {
        self.metrics_instrs = 0;
        self.metrics_jit_hits = 0;
        self.fn_call_counts.clear();
        self.branch_stats.clear();
        self.loop_back_edge_counts.clear();
        if let Some(prof) = self.profiler.as_mut() {
            *prof = VMProfiler::new();
        }
    }
}

impl Default for VMCore {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for VMCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VMCore")
            .field("u8_wrap", &self.u8_wrap)
            .field("profiler_enabled", &self.profiler_enabled)
            .field("max_frames", &self.max_frames)
            .field("frames_depth", &self.frames.len())
            .field("metrics_instrs", &self.metrics_instrs)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn make_const_fn(value: i64) -> IIRModule {
        let fn_ = IIRFunction::new(
            "main", vec![], "u8",
            vec![
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(value)], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u8"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(fn_);
        module
    }

    fn make_add_fn() -> IIRModule {
        let fn_ = IIRFunction::new(
            "add",
            vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
            "u8",
            vec![
                IIRInstr::new("add", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u8"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(fn_);
        module
    }

    #[test]
    fn execute_const_returns_value() {
        let mut vm = VMCore::new();
        let mut module = make_const_fn(42);
        let result = vm.execute(&mut module, "main", &[]).unwrap();
        assert_eq!(result, Some(Value::Int(42)));
    }

    #[test]
    fn execute_add_with_args() {
        let mut vm = VMCore::new();
        let mut module = make_add_fn();
        let result = vm.execute(&mut module, "add", &[Value::Int(10), Value::Int(32)]).unwrap();
        assert_eq!(result, Some(Value::Int(42)));
    }

    #[test]
    fn u8_wrap_mode_masks_result() {
        let mut vm = VMCore::with_u8_wrap();
        // 200 + 100 = 300, wraps to 300 & 0xFF = 44
        let fn_ = IIRFunction::new(
            "main", vec![], "u8",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Int(200)], "u8"),
                IIRInstr::new("const", Some("b".into()), vec![Operand::Int(100)], "u8"),
                IIRInstr::new("add", Some("c".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "u8"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(fn_);
        let result = vm.execute(&mut module, "main", &[]).unwrap();
        assert_eq!(result, Some(Value::Int(44))); // 300 & 0xFF = 44
    }

    #[test]
    fn jit_handler_intercepts_call() {
        let mut vm = VMCore::new();
        vm.register_jit_handler("double", |args| {
            let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            Value::Int(n * 2)
        });

        let double_fn = IIRFunction::new(
            "double",
            vec![("n".into(), "u8".into())],
            "u8",
            vec![
                IIRInstr::new("add", Some("v".into()),
                    vec![Operand::Var("n".into()), Operand::Var("n".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u8"),
            ],
        );
        let main_fn = IIRFunction::new(
            "main", vec![], "u8",
            vec![
                IIRInstr::new("const", Some("x".into()), vec![Operand::Int(21)], "u8"),
                IIRInstr::new("call", Some("r".into()),
                    vec![Operand::Var("double".into()), Operand::Var("x".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(double_fn);
        module.add_or_replace(main_fn);

        let result = vm.execute(&mut module, "main", &[]).unwrap();
        assert_eq!(result, Some(Value::Int(42)));
        assert_eq!(vm.metrics_jit_hits(), 1);
    }

    #[test]
    fn conditional_branch_works() {
        // if 10 > 5: ret 1 else: ret 0
        let fn_ = IIRFunction::new(
            "main", vec![], "u8",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Int(10)], "u8"),
                IIRInstr::new("const", Some("b".into()), vec![Operand::Int(5)], "u8"),
                IIRInstr::new("cmp_gt", Some("cond".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
                IIRInstr::new("jmp_if_true", None,
                    vec![Operand::Var("cond".into()), Operand::Var("taken".into())], "void"),
                IIRInstr::new("const", Some("r".into()), vec![Operand::Int(0)], "u8"),
                IIRInstr::new("jmp", None, vec![Operand::Var("end".into())], "void"),
                IIRInstr::new("label", None, vec![Operand::Var("taken".into())], "void"),
                IIRInstr::new("const", Some("r".into()), vec![Operand::Int(1)], "u8"),
                IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(fn_);
        let mut vm = VMCore::new();
        let result = vm.execute(&mut module, "main", &[]).unwrap();
        // The taken branch assigns r = Int(1) via "const r 1"; ret returns that.
        assert_eq!(result, Some(Value::Int(1)));
    }

    #[test]
    fn function_call_and_return() {
        // main calls add(3, 4) → 7
        let add_fn = IIRFunction::new(
            "add",
            vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
            "u8",
            vec![
                IIRInstr::new("add", Some("r".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
            ],
        );
        let main_fn = IIRFunction::new(
            "main", vec![], "u8",
            vec![
                IIRInstr::new("const", Some("x".into()), vec![Operand::Int(3)], "u8"),
                IIRInstr::new("const", Some("y".into()), vec![Operand::Int(4)], "u8"),
                IIRInstr::new("call", Some("result".into()),
                    vec![Operand::Var("add".into()), Operand::Var("x".into()), Operand::Var("y".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("result".into())], "u8"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(add_fn);
        module.add_or_replace(main_fn);

        let mut vm = VMCore::new();
        let result = vm.execute(&mut module, "main", &[]).unwrap();
        assert_eq!(result, Some(Value::Int(7)));
    }

    #[test]
    fn loop_with_counter() {
        // sum = 0; i = 0; while i < 5: sum += i; i++; ret sum
        // Expected: 0+1+2+3+4 = 10
        let fn_ = IIRFunction::new(
            "main", vec![], "u8",
            vec![
                IIRInstr::new("const", Some("sum".into()), vec![Operand::Int(0)], "u8"),
                IIRInstr::new("const", Some("i".into()), vec![Operand::Int(0)], "u8"),
                IIRInstr::new("const", Some("limit".into()), vec![Operand::Int(5)], "u8"),
                IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "u8"),
                IIRInstr::new("label", None, vec![Operand::Var("loop_start".into())], "void"),
                IIRInstr::new("cmp_lt", Some("cond".into()),
                    vec![Operand::Var("i".into()), Operand::Var("limit".into())], "bool"),
                IIRInstr::new("jmp_if_false", None,
                    vec![Operand::Var("cond".into()), Operand::Var("loop_end".into())], "void"),
                IIRInstr::new("add", Some("sum".into()),
                    vec![Operand::Var("sum".into()), Operand::Var("i".into())], "u8"),
                IIRInstr::new("add", Some("i".into()),
                    vec![Operand::Var("i".into()), Operand::Var("one".into())], "u8"),
                IIRInstr::new("jmp", None, vec![Operand::Var("loop_start".into())], "void"),
                IIRInstr::new("label", None, vec![Operand::Var("loop_end".into())], "void"),
                IIRInstr::new("ret", None, vec![Operand::Var("sum".into())], "u8"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(fn_);
        let mut vm = VMCore::new();
        let result = vm.execute(&mut module, "main", &[]).unwrap();
        assert_eq!(result, Some(Value::Int(10)));
    }

    #[test]
    fn unknown_function_returns_error() {
        let mut vm = VMCore::new();
        let mut module = IIRModule::new("test", "test");
        module.entry_point = None;
        assert!(vm.execute(&mut module, "missing", &[]).is_err());
    }

    #[test]
    fn profiler_records_observations_for_any_type() {
        let fn_ = IIRFunction::new(
            "main", vec![], "any",
            vec![
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "any"),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "any"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(fn_);
        let mut vm = VMCore::new();
        vm.execute(&mut module, "main", &[]).unwrap();
        // The const instruction (type_hint="any") should have been profiled.
        let obs_count = module.get_function("main").unwrap().instructions[0].observation_count;
        assert_eq!(obs_count, 1);
    }

    // ── LANG17 metrics tests ──────────────────────────────────────────────────

    /// Build a loop module: i = 0; while i < limit: i++; ret i
    /// The loop runs `limit` iterations.
    fn make_loop_module(limit: i64) -> IIRModule {
        let fn_ = IIRFunction::new(
            "main", vec![], "u8",
            vec![
                /* 0 */ IIRInstr::new("const", Some("i".into()), vec![Operand::Int(0)], "u8"),
                /* 1 */ IIRInstr::new("const", Some("lim".into()), vec![Operand::Int(limit)], "u8"),
                /* 2 */ IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "u8"),
                /* 3 */ IIRInstr::new("label", None, vec![Operand::Var("top".into())], "void"),
                /* 4 */ IIRInstr::new("cmp_lt", Some("cond".into()),
                    vec![Operand::Var("i".into()), Operand::Var("lim".into())], "bool"),
                /* 5 */ IIRInstr::new("jmp_if_false", None,
                    vec![Operand::Var("cond".into()), Operand::Var("end".into())], "void"),
                /* 6 */ IIRInstr::new("add", Some("i".into()),
                    vec![Operand::Var("i".into()), Operand::Var("one".into())], "u8"),
                /* 7 */ IIRInstr::new("jmp", None, vec![Operand::Var("top".into())], "void"),
                /* 8 */ IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
                /* 9 */ IIRInstr::new("ret", None, vec![Operand::Var("i".into())], "u8"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(fn_);
        module
    }

    #[test]
    fn branch_stats_not_taken_incremented_on_loop_exit() {
        let mut module = make_loop_module(3);
        let mut vm = VMCore::new();
        vm.execute(&mut module, "main", &[]).unwrap();

        // jmp_if_false at ip=5: taken 1 time (loop exits when i==3),
        // not taken 3 times (when i<3 the branch is not taken and loop continues).
        let stats = vm.branch_profile("main", 5).expect("branch stats for jmp_if_false");
        assert_eq!(stats.taken_count, 1);   // one exit
        assert_eq!(stats.not_taken_count, 3); // three iterations
    }

    #[test]
    fn branch_stats_taken_ratio_near_zero_for_busy_loop() {
        // With limit=100, the branch is NOT taken 100 times and taken 1 time.
        let mut module = make_loop_module(100);
        let mut vm = VMCore::new();
        vm.execute(&mut module, "main", &[]).unwrap();
        let stats = vm.branch_profile("main", 5).unwrap();
        assert_eq!(stats.taken_count, 1);
        assert_eq!(stats.not_taken_count, 100);
        // taken_ratio ≈ 0.0099 (well below 0.5)
        assert!(stats.taken_ratio() < 0.05);
    }

    #[test]
    fn loop_iterations_counts_back_edges() {
        // limit=5 → jmp at ip=7 is the back-edge; it fires 5 times.
        let mut module = make_loop_module(5);
        let mut vm = VMCore::new();
        vm.execute(&mut module, "main", &[]).unwrap();
        let counts = vm.loop_iterations("main");
        // jmp at ip=7 is a back-edge (target = "top" = ip 3 < 7).
        assert!(counts.contains_key(&7), "back-edge at ip=7 not found: {:?}", counts);
        assert_eq!(counts[&7], 5);
    }

    #[test]
    fn loop_iterations_empty_for_no_loops() {
        let mut vm = VMCore::new();
        let mut module = make_const_fn(99);
        vm.execute(&mut module, "main", &[]).unwrap();
        assert!(vm.loop_iterations("main").is_empty());
    }

    #[test]
    fn hot_functions_returns_names_above_threshold() {
        let mut vm = VMCore::new();
        let mut module = make_const_fn(1);
        // Call "main" 5 times.
        for _ in 0..5 {
            vm.execute(&mut module, "main", &[]).unwrap();
        }
        let hot = vm.hot_functions(3);
        assert!(hot.contains(&"main".to_string()));
        let not_hot = vm.hot_functions(10);
        assert!(not_hot.is_empty());
    }

    #[test]
    fn reset_metrics_clears_all_counters() {
        let mut vm = VMCore::new();
        let mut module = make_loop_module(3);
        vm.execute(&mut module, "main", &[]).unwrap();

        assert!(vm.metrics_instrs() > 0);
        assert!(vm.branch_profile("main", 5).is_some());
        assert!(!vm.loop_iterations("main").is_empty());

        vm.reset_metrics();

        assert_eq!(vm.metrics_instrs(), 0);
        assert!(vm.branch_profile("main", 5).is_none());
        assert!(vm.loop_iterations("main").is_empty());
    }

    #[test]
    fn execute_traced_returns_trace_records() {
        let mut vm = VMCore::new();
        let mut module = make_const_fn(42);
        let (result, traces) = vm.execute_traced(&mut module, "main", &[]).unwrap();
        assert_eq!(result, Some(Value::Int(42)));
        // "const" + "ret" = 2 instructions → 2 trace records.
        assert_eq!(traces.len(), 2);
        assert_eq!(traces[0].fn_name, "main");
        assert_eq!(traces[0].ip, 0);
        assert_eq!(traces[0].instr.op, "const");
        assert_eq!(traces[1].instr.op, "ret");
    }

    #[test]
    fn execute_traced_records_registers_before_and_after() {
        let mut vm = VMCore::new();
        let mut module = make_const_fn(7);
        let (_result, traces) = vm.execute_traced(&mut module, "main", &[]).unwrap();
        // After "const v=7": registers_after[0] should be Int(7).
        let after = &traces[0].registers_after;
        assert!(!after.is_empty());
        assert_eq!(after[0], Value::Int(7));
    }

    #[test]
    fn execute_traced_records_frame_depth() {
        let mut vm = VMCore::new();
        let mut module = make_const_fn(1);
        let (_result, traces) = vm.execute_traced(&mut module, "main", &[]).unwrap();
        // Top-level function: frame_depth = 0.
        assert_eq!(traces[0].frame_depth, 0);
    }

    #[test]
    fn branch_stats_profile_returns_none_for_unexecuted_branch() {
        let vm = VMCore::new();
        assert!(vm.branch_profile("main", 0).is_none());
        assert!(vm.branch_profile("nonexistent", 42).is_none());
    }

    /// The byte-tape ops (LANG-MATRIX Phase V — Brainfuck on the VM). `alloc_bytes`
    /// binds a tape base, `store_byte` writes a cell's low byte (so a value > 255
    /// wraps — Brainfuck's 8-bit cell), and `load_byte` reads it back *unsigned*
    /// (so a high byte like 200 reads as 200, never a sign-extended negative).
    #[test]
    fn byte_tape_round_trips_unsigned_and_wraps() {
        // 200 stored, read back unsigned as 200.
        let fn_ = IIRFunction::new(
            "main", vec![], "i64",
            vec![
                IIRInstr::new("const", Some("size".into()), vec![Operand::Int(8)], "i64"),
                IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("size".into())], "i64"),
                IIRInstr::new("const", Some("idx".into()), vec![Operand::Int(0)], "i64"),
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(200)], "i64"),
                IIRInstr::new("store_byte", None, vec![
                    Operand::Var("tape".into()), Operand::Var("idx".into()), Operand::Var("v".into()),
                ], "i64"),
                IIRInstr::new("load_byte", Some("got".into()), vec![
                    Operand::Var("tape".into()), Operand::Var("idx".into()),
                ], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("got".into())], "i64"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(fn_);
        let mut vm = VMCore::new();
        assert_eq!(vm.execute(&mut module, "main", &[]).unwrap(), Some(Value::Int(200)));

        // 257 stored wraps to 1 (257 & 0xFF) — the 8-bit cell.
        let wrap = IIRFunction::new(
            "main", vec![], "i64",
            vec![
                IIRInstr::new("const", Some("size".into()), vec![Operand::Int(8)], "i64"),
                IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("size".into())], "i64"),
                IIRInstr::new("const", Some("idx".into()), vec![Operand::Int(3)], "i64"),
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(257)], "i64"),
                IIRInstr::new("store_byte", None, vec![
                    Operand::Var("tape".into()), Operand::Var("idx".into()), Operand::Var("v".into()),
                ], "i64"),
                IIRInstr::new("load_byte", Some("got".into()), vec![
                    Operand::Var("tape".into()), Operand::Var("idx".into()),
                ], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("got".into())], "i64"),
            ],
        );
        let mut wm = IIRModule::new("test", "test");
        wm.add_or_replace(wrap);
        let mut vm2 = VMCore::new();
        assert_eq!(vm2.execute(&mut wm, "main", &[]).unwrap(), Some(Value::Int(1)));
    }

    /// An untouched tape cell reads `0` (Brainfuck's zero-cell convention): the
    /// sparse `memory` map has no entry, so `load_byte` returns the default.
    #[test]
    fn untouched_byte_tape_cell_reads_zero() {
        let fn_ = IIRFunction::new(
            "main", vec![], "i64",
            vec![
                IIRInstr::new("const", Some("size".into()), vec![Operand::Int(8)], "i64"),
                IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("size".into())], "i64"),
                IIRInstr::new("const", Some("idx".into()), vec![Operand::Int(5)], "i64"),
                IIRInstr::new("load_byte", Some("got".into()), vec![
                    Operand::Var("tape".into()), Operand::Var("idx".into()),
                ], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("got".into())], "i64"),
            ],
        );
        let mut module = IIRModule::new("test", "test");
        module.add_or_replace(fn_);
        let mut vm = VMCore::new();
        assert_eq!(vm.execute(&mut module, "main", &[]).unwrap(), Some(Value::Int(0)));
    }

    // ---- E2: register-arithmetic width & wrap (mod-2ⁿ) ----

    /// Run `op a b` with the given result `type_hint` and return the i64 value.
    fn run_binop(op: &str, a: i64, b: i64, ty: &str) -> i64 {
        let fn_ = IIRFunction::new(
            "main", vec![], "i64",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Int(a)], "i64"),
                IIRInstr::new("const", Some("b".into()), vec![Operand::Int(b)], "i64"),
                IIRInstr::new(op, Some("r".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], ty),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
            ],
        );
        let mut m = IIRModule::new("test", "test");
        m.add_or_replace(fn_);
        VMCore::new().execute(&mut m, "main", &[]).unwrap().unwrap().as_i64().unwrap()
    }

    /// Run `op a` (unary) with the given result `type_hint`.
    fn run_unop(op: &str, a: i64, ty: &str) -> i64 {
        let fn_ = IIRFunction::new(
            "main", vec![], "i64",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Int(a)], "i64"),
                IIRInstr::new(op, Some("r".into()), vec![Operand::Var("a".into())], ty),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
            ],
        );
        let mut m = IIRModule::new("test", "test");
        m.add_or_replace(fn_);
        VMCore::new().execute(&mut m, "main", &[]).unwrap().unwrap().as_i64().unwrap()
    }

    #[test]
    fn u8_arithmetic_wraps_mod_256() {
        assert_eq!(run_binop("add", 200, 100, "u8"), 44);   // 300 & 0xFF
        assert_eq!(run_binop("mul", 16, 16, "u8"), 0);      // 256 & 0xFF
        assert_eq!(run_binop("sub", 0, 1, "u8"), 255);      // -1 & 0xFF
        assert_eq!(run_binop("add", 255, 1, "u8"), 0);      // classic cell wrap
    }

    #[test]
    fn u8_bitwise_not_flips_eight_bits() {
        assert_eq!(run_unop("not", 0, "u8"), 255);          // ~0 over a byte
        assert_eq!(run_unop("not", 0xF0, "u8"), 0x0F);
    }

    #[test]
    fn u8_left_shift_masks_off_high_bits() {
        assert_eq!(run_binop("shl", 1, 7, "u8"), 128);
        assert_eq!(run_binop("shl", 1, 8, "u8"), 0);        // shifted past the byte
        assert_eq!(run_binop("shl", 3, 6, "u8"), 192);      // 0b11000000
    }

    #[test]
    fn u4_arithmetic_wraps_mod_16() {
        assert_eq!(run_binop("add", 10, 10, "u4"), 4);      // 20 & 0xF
        assert_eq!(run_unop("not", 0, "u4"), 15);
    }

    #[test]
    fn u16_and_u32_widths_wrap() {
        assert_eq!(run_binop("add", 60000, 10000, "u16"), 70000 & 0xFFFF); // 4464
        assert_eq!(run_binop("mul", 0x1_0000, 0x1_0000, "u32"), 0);        // 2^32 & 0xFFFF_FFFF
    }

    #[test]
    fn i64_hint_does_not_mask() {
        // The width mask only fires for narrow hints; plain i64 keeps full width.
        assert_eq!(run_binop("add", 200, 100, "i64"), 300);
        assert_eq!(run_binop("mul", 16, 16, "i64"), 256);
        assert_eq!(run_unop("not", 0, "i64"), -1);
    }

    // ---- E3: floating-point (f64) execution ----

    /// Run `op fa fb` with an `f64` result hint and return the float value.
    /// Operands are `Operand::Float` constants — exactly what the ALGOL real
    /// frontend emits for a `real` expression.
    fn run_fbinop(op: &str, fa: f64, fb: f64) -> f64 {
        let fn_ = IIRFunction::new(
            "main", vec![], "f64",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Float(fa)], "f64"),
                IIRInstr::new("const", Some("b".into()), vec![Operand::Float(fb)], "f64"),
                IIRInstr::new(op, Some("r".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
            ],
        );
        let mut m = IIRModule::new("test", "test");
        m.add_or_replace(fn_);
        VMCore::new().execute(&mut m, "main", &[]).unwrap().unwrap().as_f64().unwrap()
    }

    /// Run a float comparison `cmp_* fa fb` and return the resulting bool.
    fn run_fcmp(op: &str, fa: f64, fb: f64) -> bool {
        let fn_ = IIRFunction::new(
            "main", vec![], "bool",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Float(fa)], "f64"),
                IIRInstr::new("const", Some("b".into()), vec![Operand::Float(fb)], "f64"),
                IIRInstr::new(op, Some("r".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "bool"),
            ],
        );
        let mut m = IIRModule::new("test", "test");
        m.add_or_replace(fn_);
        VMCore::new().execute(&mut m, "main", &[]).unwrap().unwrap().as_bool().unwrap()
    }

    #[test]
    fn f64_arithmetic_computes_in_double() {
        assert_eq!(run_fbinop("add", 2.5, 0.25), 2.75);
        assert_eq!(run_fbinop("sub", 2.5, 0.25), 2.25);
        assert_eq!(run_fbinop("mul", 2.5, 4.0), 10.0);
        assert_eq!(run_fbinop("div", 7.0, 2.0), 3.5);   // true division, not 3
    }

    #[test]
    fn f64_division_by_zero_is_inf_not_a_trap() {
        // IEEE-754: matches LLVM/WASM/JVM `fdiv` (the other backends), so a
        // real-division program agrees cross-backend instead of trapping.
        assert!(run_fbinop("div", 1.0, 0.0).is_infinite());
    }

    #[test]
    fn f64_ordered_comparisons() {
        assert!(run_fcmp("cmp_lt", 1.5, 2.5));
        assert!(!run_fcmp("cmp_lt", 2.5, 1.5));
        assert!(run_fcmp("cmp_gt", 2.5, 1.5));
        assert!(run_fcmp("cmp_le", 2.5, 2.5));
        assert!(run_fcmp("cmp_ge", 2.5, 2.5));
    }

    #[test]
    fn f64_equality_via_value_eq() {
        // cmp_eq / cmp_ne route through Value's PartialEq, which compares floats.
        assert!(run_fcmp("cmp_eq", 5.0, 5.0));
        assert!(!run_fcmp("cmp_eq", 5.0, 5.1));
        assert!(run_fcmp("cmp_ne", 5.0, 5.1));
    }

    #[test]
    fn f64_neg_stays_float() {
        let fn_ = IIRFunction::new(
            "main", vec![], "f64",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Float(2.5)], "f64"),
                IIRInstr::new("neg", Some("r".into()), vec![Operand::Var("a".into())], "f64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
            ],
        );
        let mut m = IIRModule::new("test", "test");
        m.add_or_replace(fn_);
        let r = VMCore::new().execute(&mut m, "main", &[]).unwrap().unwrap();
        assert_eq!(r, Value::Float(-2.5));
    }

    // ---- E8: numeric conversions (integer ↔ real) ----

    /// Run a single unary conversion `op` on a constant operand and return the
    /// raw `Value` (or the error). `lit` is the input constant, `in_ty`/`ret_ty`
    /// its IIR types.
    fn run_convert(
        op: &str,
        lit: Operand,
        in_ty: &str,
        ret_ty: &str,
    ) -> Result<Value, crate::errors::VMError> {
        let fn_ = IIRFunction::new(
            "main", vec![], ret_ty,
            vec![
                IIRInstr::new("const", Some("a".into()), vec![lit], in_ty),
                IIRInstr::new(op, Some("r".into()), vec![Operand::Var("a".into())], ret_ty),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], ret_ty),
            ],
        );
        let mut m = IIRModule::new("test", "test");
        m.add_or_replace(fn_);
        VMCore::new().execute(&mut m, "main", &[]).map(|o| o.unwrap())
    }

    #[test]
    fn int_to_real_widens_exactly() {
        assert_eq!(run_convert("int_to_real", Operand::Int(3), "i64", "f64").unwrap(), Value::Float(3.0));
        assert_eq!(run_convert("int_to_real", Operand::Int(-7), "i64", "f64").unwrap(), Value::Float(-7.0));
        assert_eq!(run_convert("int_to_real", Operand::Int(0), "i64", "f64").unwrap(), Value::Float(0.0));
    }

    #[test]
    fn real_to_int_trunc_rounds_toward_zero() {
        // Truncation: positive and negative both round toward zero.
        assert_eq!(run_convert("real_to_int_trunc", Operand::Float(2.7), "f64", "i64").unwrap(), Value::Int(2));
        assert_eq!(run_convert("real_to_int_trunc", Operand::Float(-2.7), "f64", "i64").unwrap(), Value::Int(-2));
        assert_eq!(run_convert("real_to_int_trunc", Operand::Float(5.0), "f64", "i64").unwrap(), Value::Int(5));
    }

    #[test]
    fn real_to_int_floor_rounds_toward_neg_inf() {
        // Floor (ALGOL entier): a negative non-integer rounds DOWN, the key
        // difference from truncation — entier(-2.7) = -3, not -2.
        assert_eq!(run_convert("real_to_int_floor", Operand::Float(2.7), "f64", "i64").unwrap(), Value::Int(2));
        assert_eq!(run_convert("real_to_int_floor", Operand::Float(-2.7), "f64", "i64").unwrap(), Value::Int(-3));
        assert_eq!(run_convert("real_to_int_floor", Operand::Float(-3.0), "f64", "i64").unwrap(), Value::Int(-3));
    }

    #[test]
    fn real_to_int_traps_on_nan_and_infinity() {
        // Fail-closed (like array bounds / divide-by-zero), never a garbage int.
        assert!(run_convert("real_to_int_trunc", Operand::Float(f64::NAN), "f64", "i64").is_err());
        assert!(run_convert("real_to_int_floor", Operand::Float(f64::INFINITY), "f64", "i64").is_err());
        assert!(run_convert("real_to_int_trunc", Operand::Float(f64::NEG_INFINITY), "f64", "i64").is_err());
    }

    #[test]
    fn real_to_int_traps_on_out_of_range() {
        // 2^63 and beyond does not fit i64 — trap rather than saturate/wrap.
        let two_pow_63 = 9_223_372_036_854_775_808.0_f64; // = 2^63 = -(i64::MIN as f64)
        assert!(run_convert("real_to_int_trunc", Operand::Float(two_pow_63), "f64", "i64").is_err());
        assert!(run_convert("real_to_int_floor", Operand::Float(-1e30), "f64", "i64").is_err());
    }

    // ---- E5: bounds-checked arrays ----

    /// Helper: build `main` from instrs returning `ret_ty`, run, return result.
    fn run_arr(instrs: Vec<IIRInstr>, ret_ty: &str) -> Result<Option<Value>, crate::errors::VMError> {
        let fn_ = IIRFunction::new("main", vec![], ret_ty, instrs);
        let mut m = IIRModule::new("test", "test");
        m.add_or_replace(fn_);
        VMCore::new().execute(&mut m, "main", &[])
    }

    /// `integer array A[3]; A[0]:=10; A[2]:=12; ret A[0]+A[2]` → 22.
    #[test]
    fn array_alloc_set_get_roundtrip() {
        let r = run_arr(vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(3)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("i2".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new("const", Some("v10".into()), vec![Operand::Int(10)], "i64"),
            IIRInstr::new("const", Some("v12".into()), vec![Operand::Int(12)], "i64"),
            IIRInstr::new("array_set", None, vec![Operand::Var("a".into()), Operand::Var("i0".into()), Operand::Var("v10".into())], "array<i64>"),
            IIRInstr::new("array_set", None, vec![Operand::Var("a".into()), Operand::Var("i2".into()), Operand::Var("v12".into())], "array<i64>"),
            IIRInstr::new("array_get", Some("x".into()), vec![Operand::Var("a".into()), Operand::Var("i0".into())], "i64"),
            IIRInstr::new("array_get", Some("y".into()), vec![Operand::Var("a".into()), Operand::Var("i2".into())], "i64"),
            IIRInstr::new("add", Some("r".into()), vec![Operand::Var("x".into()), Operand::Var("y".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ], "i64");
        assert_eq!(r.unwrap(), Some(Value::Int(22)));
    }

    /// A fresh array is zero-initialised: reading an unset element gives 0.
    #[test]
    fn array_default_initialised_to_zero() {
        let r = run_arr(vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(4)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("array_get", Some("r".into()), vec![Operand::Var("a".into()), Operand::Var("i".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ], "i64");
        assert_eq!(r.unwrap(), Some(Value::Int(0)));
    }

    /// `array_len` returns the element count.
    #[test]
    fn array_len_returns_count() {
        let r = run_arr(vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(7)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("array_len", Some("r".into()), vec![Operand::Var("a".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ], "i64");
        assert_eq!(r.unwrap(), Some(Value::Int(7)));
    }

    /// An `f64` array zero-inits to `0.0` and round-trips a real value.
    #[test]
    fn f64_array_roundtrips() {
        let r = run_arr(vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<f64>"),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(2.5)], "f64"),
            IIRInstr::new("array_set", None, vec![Operand::Var("a".into()), Operand::Var("i".into()), Operand::Var("v".into())], "array<f64>"),
            IIRInstr::new("array_get", Some("r".into()), vec![Operand::Var("a".into()), Operand::Var("i".into())], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
        ], "f64");
        assert_eq!(r.unwrap(), Some(Value::Float(2.5)));
    }

    /// An out-of-bounds `array_get` traps (the index check is by-definition).
    #[test]
    fn array_get_out_of_bounds_traps() {
        let r = run_arr(vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(3)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(5)], "i64"),
            IIRInstr::new("array_get", Some("r".into()), vec![Operand::Var("a".into()), Operand::Var("i".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ], "i64");
        assert!(r.is_err(), "out-of-bounds array_get must trap, got {r:?}");
    }

    /// A negative index also traps (the lower-bound half of the check).
    #[test]
    fn array_set_negative_index_traps() {
        let r = run_arr(vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(3)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(-1)], "i64"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(9)], "i64"),
            IIRInstr::new("array_set", None, vec![Operand::Var("a".into()), Operand::Var("i".into()), Operand::Var("v".into())], "array<i64>"),
            IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i64"),
        ], "i64");
        assert!(r.is_err(), "negative array_set index must trap, got {r:?}");
    }

    /// Distinct arrays do not alias (the per-allocation Vec model).
    #[test]
    fn distinct_arrays_do_not_alias() {
        let r = run_arr(vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("alloc_array", Some("b".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("va".into()), vec![Operand::Int(11)], "i64"),
            IIRInstr::new("const", Some("vb".into()), vec![Operand::Int(22)], "i64"),
            IIRInstr::new("array_set", None, vec![Operand::Var("a".into()), Operand::Var("i".into()), Operand::Var("va".into())], "array<i64>"),
            IIRInstr::new("array_set", None, vec![Operand::Var("b".into()), Operand::Var("i".into()), Operand::Var("vb".into())], "array<i64>"),
            // a[0] must still be 11 (not clobbered by b[0]=22).
            IIRInstr::new("array_get", Some("r".into()), vec![Operand::Var("a".into()), Operand::Var("i".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ], "i64");
        assert_eq!(r.unwrap(), Some(Value::Int(11)));
    }

    /// `max_memory_entries` is an **aggregate** ceiling: many small arrays whose
    /// element counts sum past the cap must trap, not just one oversized array.
    /// (Defeats an `alloc_array`-in-a-loop OOM, the E5 security-review finding.)
    #[test]
    fn aggregate_array_allocation_is_capped() {
        let mut m = IIRModule::new("test", "test");
        m.add_or_replace(IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("const", Some("n".into()), vec![Operand::Int(6)], "i64"),
                // First 6-element array: fits under a cap of 10.
                IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
                // Second 6-element array: 6 + 6 = 12 > 10 → must trap.
                IIRInstr::new("alloc_array", Some("b".into()), vec![Operand::Var("n".into())], "array<i64>"),
                IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i64"),
            ],
        ));
        let mut vm = VMCore::new();
        vm.max_memory_entries = 10;
        let r = vm.execute(&mut m, "main", &[]);
        assert!(r.is_err(), "aggregate over-allocation must trap, got {r:?}");
    }
}
