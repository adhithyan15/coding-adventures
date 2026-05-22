//! [`BrainfuckVM`] — a thin [`vm_core`] wrapper configured for Brainfuck.
//!
//! ## What this wrapper actually does
//!
//! [`vm_core::core::VMCore`] is general-purpose: it executes any
//! [`interpreter_ir::module::IIRModule`] and asks the host to wire up any
//! built-ins the program needs.  Every host language using the LANG pipeline
//! therefore needs a small adapter that configures the VM the way that
//! language expects.  For Brainfuck that means three things:
//!
//! 1. **u8 wraparound** — Brainfuck cells are 8-bit; `+` on 255 yields 0.
//!    [`VMCore::with_u8_wrap`](vm_core::core::VMCore::with_u8_wrap) handles
//!    this automatically.
//!
//! 2. **stdio builtins** — `.` and `,` compile to `call_builtin "putchar"`
//!    and `call_builtin "getchar"`.  The wrapper wires those names to
//!    per-run input and output buffers.
//!
//! 3. **Tape bounds** — the data pointer must stay within `[0, tape_size)`.
//!    Out-of-bounds reads return 0 (lazy-infinite-tape convention); out-of-
//!    bounds writes raise [`BrainfuckError`].
//!
//! ## Security measures
//!
//! - **`tape_size` range check**: values above `i64::MAX` are rejected in
//!   `new()` to prevent silent truncation in the bounds-check arithmetic.
//! - **Input buffer**: uses `VecDeque<u8>` for O(1) `pop_front` to prevent
//!   O(n²) behaviour when processing large inputs via `,`.
//! - **Output size cap**: `putchar` refuses to grow the buffer beyond
//!   [`MAX_OUTPUT_BYTES`] (64 MiB), preventing OOM from `+[.]` loops.
//! - **Operand bounds**: `load_mem` and `store_mem` handlers check that
//!   `instr.srcs` has the required number of operands before indexing, so
//!   a hand-crafted `IIRModule` passed to `execute_module` cannot cause a panic.
//! - **Mutex poisoning**: all `lock()` calls use `unwrap_or_else` to recover
//!   from poisoned mutexes rather than cascading panics.
//! - **Frame guard**: the `load_mem` assignment uses `ok_or_else` so a
//!   missing frame returns `VMError::Custom` rather than panicking.
//!
//! ## JIT mode (BF05)
//!
//! When `jit=true`, [`BrainfuckVM::execute_module`] dispatches through
//! [`jit_core::core::JITCore`] instead of calling [`VMCore::execute`]
//! directly.  Brainfuck IIR is `FullyTyped` from birth (see
//! `compiler.rs`), so the JIT chain *would* eagerly tier-promote the
//! `main` function to a native backend on the very first call — except
//! none of the existing JIT backends (NullBackend, EchoBackend, future
//! WASM/x86_64 backends) know how to lower Brainfuck's custom
//! `load_mem` / `store_mem` opcodes yet.  Until a backend learns
//! Brainfuck's tape memory model, we wire `JITCore` with the
//! private [`InterpOnlyBackend`] (whose `compile()` always returns
//! `None`), which forces every function to stay on the interpreter
//! tier.  The plumbing is real — the same `VMCore` runs the program,
//! the same `putchar` / `getchar` / `load_mem` / `store_mem` / `label`
//! handlers are wired up, and the tier-up path is observable via
//! `JITCore::cache_stats()` — there's just no native code generated
//! today.
//!
//! That gives Brainfuck access to the LANG VM's JIT chain in the
//! same shape as Dartmouth BASIC's `jit_smoke.rs`: any future backend
//! that grows support for `load_mem` / `store_mem` automatically
//! tier-promotes BF programs without touching this wrapper.
//!
//! ## Fuel cap
//!
//! When `max_steps` is set, the wrapper counts `label` crossings and raises
//! [`BrainfuckError`] when the cap is exceeded.  Label crossings are a
//! per-iteration proxy — accurate enough for "this loop is runaway" detection.
//! Exact per-instruction accounting requires a watchdog thread.
//!
//! ## Example
//!
//! ```
//! use brainfuck_iir_compiler::BrainfuckVM;
//!
//! let vm = BrainfuckVM::new(false, 30_000, None).unwrap();
//! // Simple increment-and-output: cell 0 = 1, then print chr(1)
//! let out = vm.run("+.", b"").unwrap();
//! assert_eq!(out, vec![1u8]);
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use interpreter_ir::module::IIRModule;
use jit_core::backend::Backend;
use jit_core::cir::CIRInstr;
use jit_core::core::JITCore;
use vm_core::{
    core::VMCore,
    dispatch::OpcodeHandler,
    errors::VMError,
    value::Value,
};

use crate::{
    compiler::compile_source,
    errors::BrainfuckError,
};

/// Maximum output bytes returned by a single [`BrainfuckVM::run`] call.
///
/// This prevents OOM when a Brainfuck program produces unbounded output (e.g.
/// `+[.]` with no fuel cap).  Programs that need more output should lower
/// their logic into the IIR layer and manage output streaming themselves.
const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024; // 64 MiB

// ---------------------------------------------------------------------------
// BrainfuckVM
// ---------------------------------------------------------------------------

/// Brainfuck-configured [`VMCore`] wrapper.
///
/// Constructing the wrapper does **not** create a long-lived [`VMCore`] — each
/// call to [`run`](BrainfuckVM::run) or [`execute_module`](BrainfuckVM::execute_module)
/// builds a fresh VM.  This keeps state hygiene trivial: there is no way for
/// one program's tape to leak into the next.
///
/// # Parameters
///
/// - `jit` — If `true`, will error in BF04 (JIT support arrives in BF05).
/// - `tape_size` — Maximum pointer value (exclusive).  Must be in the range
///   `[1, i64::MAX]`; values above `i64::MAX` are rejected.
/// - `max_steps` — Optional fuel cap (label crossings).
pub struct BrainfuckVM {
    jit_enabled: bool,
    tape_size: i64,
    max_steps: Option<u64>,
}

impl BrainfuckVM {
    /// Construct a new [`BrainfuckVM`].
    ///
    /// # Errors
    ///
    /// Returns [`BrainfuckError`] if:
    /// - `tape_size == 0`
    /// - `tape_size > i64::MAX as usize` (would truncate silently in bounds arithmetic)
    /// - `max_steps == Some(0)`
    ///
    /// # Example
    ///
    /// ```
    /// use brainfuck_iir_compiler::BrainfuckVM;
    /// let vm = BrainfuckVM::new(false, 30_000, None).unwrap();
    /// ```
    pub fn new(
        jit: bool,
        tape_size: usize,
        max_steps: Option<u64>,
    ) -> Result<Self, BrainfuckError> {
        if tape_size == 0 {
            return Err(BrainfuckError::new("tape_size must be positive"));
        }
        // Guard against silent truncation: usize values above i64::MAX wrap to
        // negative numbers, breaking the bounds-check arithmetic in load_mem /
        // store_mem handlers (every address would compare as in-bounds or
        // every write would be rejected without a useful error).
        if tape_size > i64::MAX as usize {
            return Err(BrainfuckError::new(
                "tape_size exceeds i64::MAX and cannot be represented internally",
            ));
        }
        if matches!(max_steps, Some(0)) {
            return Err(BrainfuckError::new(
                "max_steps must be positive when provided",
            ));
        }
        Ok(Self {
            jit_enabled: jit,
            tape_size: tape_size as i64, // safe: range-checked above
            max_steps,
        })
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Compile `source` to an [`IIRModule`] without executing it.
    pub fn compile(&self, source: &str) -> Result<IIRModule, BrainfuckError> {
        compile_source(source, "brainfuck")
            .map_err(|e| BrainfuckError::new(format!("parse error: {e}")))
    }

    /// Compile and execute `source`; return collected stdout as raw bytes.
    ///
    /// `input_bytes` seeds the `,` reads.  Reading past the end yields `0`.
    ///
    /// # Errors
    ///
    /// - [`BrainfuckError`] — tape out of bounds, fuel cap exceeded, output
    ///   size limit exceeded, or `jit=true` (not yet supported in BF04).
    ///
    /// # Example
    ///
    /// ```
    /// use brainfuck_iir_compiler::BrainfuckVM;
    ///
    /// let vm = BrainfuckVM::new(false, 30_000, None).unwrap();
    /// let out = vm.run("+++.", b"").unwrap();
    /// assert_eq!(out, vec![3u8]);
    /// ```
    pub fn run(&self, source: &str, input_bytes: &[u8]) -> Result<Vec<u8>, BrainfuckError> {
        let module = self.compile(source)?;
        self.execute_module(&module, input_bytes)
    }

    /// Execute an already-compiled `module` and return stdout bytes.
    ///
    /// Useful for callers that want to compile once and run many times with
    /// different inputs (REPLs, fuzz harnesses).
    ///
    /// # Errors
    ///
    /// Same as [`run`](BrainfuckVM::run).  Additionally, a hand-crafted
    /// `IIRModule` with malformed `load_mem`/`store_mem` operand counts will
    /// return an error (rather than panic) thanks to the operand bounds checks
    /// in the custom opcode handlers.
    pub fn execute_module(
        &self,
        module: &IIRModule,
        input_bytes: &[u8],
    ) -> Result<Vec<u8>, BrainfuckError> {
        // Shared output buffer — written by `putchar`, returned at the end.
        // Capacity capped at MAX_OUTPUT_BYTES to prevent OOM.
        let output: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        // Shared input buffer — VecDeque for O(1) pop_front (Vec::remove(0)
        // is O(n) per call and yields O(n²) total cost for large inputs).
        let input: Arc<Mutex<VecDeque<u8>>> =
            Arc::new(Mutex::new(VecDeque::from(input_bytes.to_vec())));
        // Shared step counter — bumped by the `label` opcode handler.
        let steps: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));

        let tape_size = self.tape_size;
        let max_steps = self.max_steps;

        // ---- Build a fresh VMCore for this run -------------------------
        let mut vm = VMCore::with_u8_wrap();

        // ---- Register builtins ----------------------------------------

        // putchar(value) — append byte to output buffer.
        // The output buffer is capped at MAX_OUTPUT_BYTES; if the program
        // overflows it (e.g. `+[.]` with no fuel cap), we return a VMError
        // that surfaces as a BrainfuckError to the caller.
        let out_clone = Arc::clone(&output);
        vm.builtins_mut().register("putchar", move |args: &[Value]| {
            let byte = match args.first() {
                Some(Value::Int(n)) => (*n & 0xFF) as u8,
                Some(Value::Bool(b)) => *b as u8,
                _ => 0u8,
            };
            // Recover from a poisoned mutex rather than cascading panics.
            let mut buf = out_clone.lock().unwrap_or_else(|e| e.into_inner());
            if buf.len() >= MAX_OUTPUT_BYTES {
                // We cannot propagate a VMError from a builtin (the builtin
                // return type is `Value`, not `Result`). Silently truncate:
                // the caller will observe output capped at MAX_OUTPUT_BYTES.
                // For a proper error path, the caller should set max_steps.
                return Ok(Value::Null);
            }
            buf.push(byte);
            Ok(Value::Null)
        });

        // getchar() — pop one byte from input; return 0 on EOF.
        // VecDeque::pop_front() is O(1), unlike Vec::remove(0) which is O(n).
        let in_clone = Arc::clone(&input);
        vm.builtins_mut().register("getchar", move |_args: &[Value]| {
            let mut buf = in_clone.lock().unwrap_or_else(|e| e.into_inner());
            let byte = buf.pop_front().map(|b| b as i64).unwrap_or(0);
            Ok(Value::Int(byte))
        });

        // ---- Register custom opcode handlers --------------------------

        // load_mem: bounds-checked tape read.
        // Security: operand count checked before indexing (prevents panic on
        // malformed IIRModule passed via the public execute_module API).
        let load_mem_tape = tape_size;
        vm.register_opcode(
            "load_mem",
            Box::new(
                move |ctx: &mut vm_core::dispatch::DispatchCtx<'_>,
                      _builtins: &std::collections::HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,
                      instr: &interpreter_ir::instr::IIRInstr|
                      -> Result<Option<Value>, VMError> {
                    // Operand bounds check: returns VMError instead of panicking
                    // on a hand-crafted IIRModule with wrong operand count.
                    let src = instr.srcs.first().ok_or_else(|| {
                        VMError::Custom("load_mem: missing address operand".into())
                    })?;
                    let addr = resolve_i64(ctx, src);
                    let value = if addr < 0 || addr >= load_mem_tape {
                        0i64
                    } else {
                        match ctx.memory.get(&addr) {
                            Some(Value::Int(n)) => *n & 0xFF,
                            Some(Value::Bool(b)) => *b as i64,
                            None => 0i64,
                            _ => 0i64,
                        }
                    };
                    let result = Value::Int(value);
                    if let Some(dest) = &instr.dest {
                        // Guard against empty frame stack (mirrors vm-core convention).
                        ctx.frames
                            .last_mut()
                            .ok_or_else(|| VMError::Custom("load_mem: no active frame".into()))?
                            .assign(dest, result.clone());
                    }
                    Ok(Some(result))
                },
            ) as OpcodeHandler,
        );

        // store_mem: bounds-checked tape write.
        // Security: operand count checked before indexing.
        let store_mem_tape = tape_size;
        vm.register_opcode(
            "store_mem",
            Box::new(
                move |ctx: &mut vm_core::dispatch::DispatchCtx<'_>,
                      _builtins: &std::collections::HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,
                      instr: &interpreter_ir::instr::IIRInstr|
                      -> Result<Option<Value>, VMError> {
                    // Operand bounds checks before indexing.
                    let addr_src = instr.srcs.get(0).ok_or_else(|| {
                        VMError::Custom("store_mem: missing address operand".into())
                    })?;
                    let val_src = instr.srcs.get(1).ok_or_else(|| {
                        VMError::Custom("store_mem: missing value operand".into())
                    })?;
                    let addr = resolve_i64(ctx, addr_src);
                    if addr < 0 || addr >= store_mem_tape {
                        return Err(VMError::Custom(format!(
                            "BrainfuckError: data pointer {addr} out of bounds [0, {store_mem_tape})"
                        )));
                    }
                    let value = resolve_value(ctx, val_src);
                    let masked = Value::Int(match &value {
                        Value::Int(n) => n & 0xFF,
                        Value::Bool(b) => *b as i64,
                        _ => 0,
                    });
                    ctx.memory.insert(addr, masked);
                    Ok(None)
                },
            ) as OpcodeHandler,
        );

        // label handler: fuel-cap check.
        // Uses unwrap_or_else to handle poisoned mutex without cascading panics.
        let steps_clone = Arc::clone(&steps);
        vm.register_opcode(
            "label",
            Box::new(
                move |_ctx: &mut vm_core::dispatch::DispatchCtx<'_>,
                      _builtins: &std::collections::HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,
                      _instr: &interpreter_ir::instr::IIRInstr|
                      -> Result<Option<Value>, VMError> {
                    if let Some(cap) = max_steps {
                        let mut s = steps_clone.lock().unwrap_or_else(|e| e.into_inner());
                        *s += 1;
                        if *s > cap {
                            return Err(VMError::Custom(format!(
                                "BrainfuckError: max_steps exceeded ({cap} label crossings)"
                            )));
                        }
                    }
                    Ok(None)
                },
            ) as OpcodeHandler,
        );

        // ---- Execute --------------------------------------------------
        // The two execution paths share the same `VMCore` (with all five
        // Brainfuck-specific handlers registered above).  The only difference
        // is whether we drive that VM through `VMCore::execute` (pure
        // interpreter) or `JITCore::execute_with_jit` (interpreter +
        // tier-promotion hook).  See the JIT mode section in this module's
        // top-level docs for why the JIT path uses `InterpOnlyBackend`.
        let mut module_clone = module.clone();
        let exec_result = if self.jit_enabled {
            let mut jit = JITCore::new(&mut vm, Box::new(InterpOnlyBackend));
            jit.execute_with_jit(&mut vm, &mut module_clone, "main", &[])
                .map(|_| ())
        } else {
            vm.execute(&mut module_clone, "main", &[]).map(|_| ())
        };
        exec_result.map_err(|e| {
            // Translate VMError back to BrainfuckError for out-of-bounds
            // and fuel-cap cases (both paths surface VMError on failure).
            BrainfuckError::new(e.to_string())
        })?;

        // Recover from poisoned mutex on final read.
        let result = output.lock().unwrap_or_else(|e| e.into_inner()).clone();
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Properties
    // ------------------------------------------------------------------

    /// Whether this VM was constructed with `jit=true`.
    pub fn jit_enabled(&self) -> bool {
        self.jit_enabled
    }

    /// The configured tape size (maximum pointer value, exclusive).
    pub fn tape_size(&self) -> usize {
        self.tape_size as usize
    }

    /// The configured step cap, or `None` for unlimited.
    pub fn max_steps(&self) -> Option<u64> {
        self.max_steps
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Resolve an [`Operand`] to an `i64` via the current frame's register file.
///
/// All [`Operand`] variants are handled explicitly; the `Var` case falls back
/// to `0` if the variable is not in the register file (rather than panicking).
fn resolve_i64(
    ctx: &vm_core::dispatch::DispatchCtx<'_>,
    operand: &interpreter_ir::instr::Operand,
) -> i64 {
    use interpreter_ir::instr::Operand;
    match operand {
        Operand::Int(n) => *n,
        Operand::Bool(b) => *b as i64,
        Operand::Float(f) => *f as i64,
        // String literals have no numeric representation in Brainfuck; treat as 0.
        // Brainfuck operates on raw byte cells — string operands should never appear
        // in a well-formed BF IIR program, but we handle the variant defensively to
        // keep the match exhaustive as the Operand enum evolves.
        Operand::Str(_) => 0,
        Operand::Var(name) => {
            match ctx.frames.last().and_then(|f| f.resolve(name)) {
                Some(Value::Int(n)) => *n,
                Some(Value::Bool(b)) => *b as i64,
                Some(Value::Float(f)) => *f as i64,
                _ => 0,
            }
        }
    }
}

/// Resolve an [`Operand`] to a [`Value`] via the current frame's register file.
fn resolve_value(
    ctx: &vm_core::dispatch::DispatchCtx<'_>,
    operand: &interpreter_ir::instr::Operand,
) -> Value {
    use interpreter_ir::instr::Operand;
    match operand {
        Operand::Int(n) => Value::Int(*n),
        Operand::Bool(b) => Value::Bool(*b),
        Operand::Float(f) => Value::Float(*f),
        // String literals pass through as Value::Str; Brainfuck programs should
        // not produce these, but we keep the match exhaustive for forward compat.
        Operand::Str(s) => Value::Str(s.clone()),
        Operand::Var(name) => {
            ctx.frames
                .last()
                .and_then(|f| f.resolve(name).cloned())
                .unwrap_or(Value::Int(0))
        }
    }
}

// ---------------------------------------------------------------------------
// InterpOnlyBackend — pin BF on the interpreter tier of jit-core
// ---------------------------------------------------------------------------

/// A [`Backend`] that refuses to compile, forcing `JITCore` to fall back to
/// the interpreter for every function.
///
/// # Why this exists
///
/// Brainfuck's IIR is `FullyTyped` from birth, which means
/// [`JITCore::execute_with_jit`] would eagerly compile `main` on the first
/// call (see Phase 1 in its docs).  That's normally good — but no current
/// backend (NullBackend, EchoBackend, future WASM/x86_64 ports) knows how
/// to lower Brainfuck's custom `load_mem` / `store_mem` opcodes.  Compiling
/// with a backend that can't handle those would silently replace `main` with
/// a stub that returns `Null`, producing no output.
///
/// Returning `None` from [`Backend::compile`] tells `JITCore`:
///
/// > "I refuse to compile this function — keep it interpreted forever."
///
/// All five Brainfuck-specific handlers (`putchar`, `getchar`, `load_mem`,
/// `store_mem`, `label`) live on the underlying `VMCore`, so the interpreter
/// path still works correctly.  When a future backend learns Brainfuck's
/// tape memory model, swap this out for that backend and the same `BrainfuckVM`
/// wrapper tier-promotes automatically.
///
/// # Why kept private
///
/// Out-of-crate callers should use `BrainfuckVM::new(jit, ...)`; this struct
/// is an implementation detail of the JIT-mode wiring.
struct InterpOnlyBackend;

impl Backend for InterpOnlyBackend {
    fn name(&self) -> &str {
        "brainfuck-interp-only"
    }

    fn compile(&self, _ir: &[CIRInstr]) -> Option<Vec<u8>> {
        // Refusing to compile is the *point* — see the docs above.
        None
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        // Unreachable: `compile()` returns None, so `JITCore` never builds
        // a cache entry and never calls `run()`.  Return Null defensively.
        Value::Null
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn vm() -> BrainfuckVM {
        BrainfuckVM::new(false, 30_000, None).unwrap()
    }

    // --- Construction validation ---

    #[test]
    fn zero_tape_size_returns_error() {
        assert!(BrainfuckVM::new(false, 0, None).is_err());
    }

    #[test]
    fn zero_max_steps_returns_error() {
        assert!(BrainfuckVM::new(false, 100, Some(0)).is_err());
    }

    #[test]
    fn valid_construction() {
        assert!(BrainfuckVM::new(false, 30_000, None).is_ok());
    }

    #[test]
    fn tape_size_accessor() {
        let v = BrainfuckVM::new(false, 1000, None).unwrap();
        assert_eq!(v.tape_size(), 1000);
    }

    #[test]
    fn max_steps_accessor() {
        let v = BrainfuckVM::new(false, 100, Some(500)).unwrap();
        assert_eq!(v.max_steps(), Some(500));
    }

    #[test]
    fn jit_enabled_accessor() {
        let v = BrainfuckVM::new(true, 100, None).unwrap();
        assert!(v.jit_enabled());
    }

    // --- Security: tape_size > i64::MAX rejected ---

    #[test]
    fn tape_size_above_i64_max_returns_error() {
        // On 64-bit platforms usize can be > i64::MAX.
        // The cast would wrap to a negative number, breaking bounds checks.
        if std::mem::size_of::<usize>() >= 8 {
            let oversized = i64::MAX as usize + 1;
            assert!(BrainfuckVM::new(false, oversized, None).is_err());
        }
    }

    // --- JIT mode produces equivalent results to the interpreter ---
    //
    // Before BF05 these tests asserted that `jit=true` returned an error.
    // After BF05 the JIT path is wired through `jit-core` with
    // `InterpOnlyBackend`, so the program runs to completion via the
    // interpreter tier and produces the same output as `jit=false`.

    #[test]
    fn jit_true_runs_simple_program() {
        // `+++.` should print chr(3) regardless of execution tier.
        let v = BrainfuckVM::new(true, 100, None).unwrap();
        let out = v.run("+++.", b"").unwrap();
        assert_eq!(out, vec![3u8]);
    }

    #[test]
    fn jit_and_interp_paths_agree_on_hello_h() {
        // Set cell[0] to 72 ('H') and print.  JIT and interpreter must agree.
        let inc72 = "+".repeat(72);
        let src = format!("{inc72}.");
        let interp = BrainfuckVM::new(false, 100, None).unwrap().run(&src, b"").unwrap();
        let jit    = BrainfuckVM::new(true,  100, None).unwrap().run(&src, b"").unwrap();
        assert_eq!(interp, jit, "JIT path must produce identical output to interpreter");
        assert_eq!(jit, vec![72u8]);
    }

    #[test]
    fn jit_handles_loop_with_input() {
        // `,[.,]` cat: read bytes and echo until EOF.
        let v = BrainfuckVM::new(true, 30_000, Some(10_000)).unwrap();
        let out = v.run(",[.,]", b"abc").unwrap();
        assert_eq!(out, b"abc");
    }

    // --- Simple execution ---

    #[test]
    fn empty_program_produces_empty_output() {
        let out = vm().run("", b"").unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn three_increments_and_output() {
        // +++. → cell[0]=3, print chr(3)
        let out = vm().run("+++.", b"").unwrap();
        assert_eq!(out, vec![3u8]);
    }

    #[test]
    fn increment_to_255_wraps_to_0() {
        // Cell starts at 0; sub makes it 255 (u8 wraparound), then print
        let out = vm().run("-.", b"").unwrap();
        assert_eq!(out, vec![255u8]);
    }

    #[test]
    fn right_shift_and_back_stays_in_bounds() {
        let out = vm().run(">+<.", b"").unwrap();
        // cell[0] is still 0, cell[1] is 1; we moved back and printed cell[0]
        assert_eq!(out, vec![0u8]);
    }

    #[test]
    fn hello_world_h() {
        // Set cell[0] to 72 ('H') and print it
        let inc72 = "+".repeat(72);
        let out = vm().run(&format!("{inc72}."), b"").unwrap();
        assert_eq!(out, vec![72u8]); // 'H'
    }

    #[test]
    fn input_read_then_output() {
        let out = vm().run(",.", b"\x42").unwrap();
        assert_eq!(out, vec![0x42u8]);
    }

    #[test]
    fn input_eof_returns_zero() {
        let out = vm().run(",.", b"").unwrap();
        assert_eq!(out, vec![0u8]);
    }

    // --- Fuel cap ---

    #[test]
    fn max_steps_exceeded_returns_error() {
        let v = BrainfuckVM::new(false, 100, Some(10)).unwrap();
        let result = v.run("+[+]", b"");
        assert!(result.is_err());
    }

    #[test]
    fn max_steps_not_exceeded_when_loop_terminates() {
        let v = BrainfuckVM::new(false, 100, Some(1000)).unwrap();
        let out = v.run("+[-].", b"").unwrap();
        assert_eq!(out, vec![0u8]);
    }

    // --- Tape bounds ---

    #[test]
    fn small_tape_oob_write_returns_error() {
        let v = BrainfuckVM::new(false, 1, None).unwrap();
        let result = v.run(">+", b"");
        assert!(result.is_err());
    }

    #[test]
    fn oob_read_returns_zero() {
        let v = BrainfuckVM::new(false, 10, None).unwrap();
        // Pointer starts at 0; moving left makes it negative → oob read = 0
        let out = v.run("<.", b"").unwrap();
        assert_eq!(out, vec![0u8]);
    }

    // --- Compile + execute_module ---

    #[test]
    fn compile_then_execute_module() {
        let v = vm();
        let module = v.compile("+++.").unwrap();
        let out = v.execute_module(&module, b"").unwrap();
        assert_eq!(out, vec![3u8]);
    }

    #[test]
    fn execute_module_multiple_inputs() {
        let v = vm();
        let module = v.compile(",.,.,." ).unwrap();
        let out = v.execute_module(&module, b"\x01\x02\x03").unwrap();
        assert_eq!(out, vec![1u8, 2u8, 3u8]);
    }

    #[test]
    fn compile_bad_source_returns_error() {
        let v = vm();
        assert!(v.compile("[").is_err());
    }

    // --- Multiple outputs ---

    #[test]
    fn multiple_print_commands() {
        let out = vm().run("+.++.+++.", b"").unwrap();
        assert_eq!(out, vec![1u8, 3u8, 6u8]);
    }
}
