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
use crate::dispatch::{run_dispatch_loop, DispatchCtx, OpcodeHandler};
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

    /// JIT handler registry.  When a `call` instruction names a function
    /// listed here, the handler is called instead of the interpreter.
    jit_handlers: HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,

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
            jit_handlers: HashMap::new(),
            extra_opcodes: HashMap::new(),
            builtins: BuiltinRegistry::new(),
            fn_call_counts: HashMap::new(),
            profiler: Some(VMProfiler::new()),
            metrics_instrs: 0,
            metrics_jit_hits: 0,
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
}
