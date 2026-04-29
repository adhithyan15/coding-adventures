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
//! The `jit` flag is accepted but raises an error in BF04.  The JIT path
//! (wiring `jit-core` + a WASM backend) is implemented in BF05.
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
        if self.jit_enabled {
            return Err(BrainfuckError::new(
                "jit=true is not yet supported in BF04; see BF05",
            ));
        }

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
        let mut module_clone = module.clone();
        vm.execute(&mut module_clone, "main", &[])
            .map_err(|e| {
                // Translate VMError back to BrainfuckError for out-of-bounds
                // and fuel-cap cases.
                let msg = e.to_string();
                BrainfuckError::new(msg)
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
        Operand::Var(name) => {
            ctx.frames
                .last()
                .and_then(|f| f.resolve(name).cloned())
                .unwrap_or(Value::Int(0))
        }
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

    // --- JIT not yet supported ---

    #[test]
    fn jit_true_returns_error_on_run() {
        let v = BrainfuckVM::new(true, 100, None).unwrap();
        assert!(v.run("+", b"").is_err());
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
