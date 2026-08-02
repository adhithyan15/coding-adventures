//! `GenericCirJit` — a universal bytecode JIT for any typed CIR.
//!
//! # Why this exists
//!
//! Before this module, every language that wanted a JIT had to write its
//! own `Backend` impl (`BrainfuckCirJit`, `BasicCirJit`, …) duplicating
//! ~70% of the same logic: register allocation, bytecode encoding for
//! typed CIR opcodes (`const_i64`, `add_i64`, `cmp_lt_i64`, `jmp`, etc.),
//! branch fixups, and the dispatch loop.  Only the language-specific
//! builtin vocabulary (`putchar` vs `print_i64` vs `cons`) actually
//! differed per language.
//!
//! `GenericCirJit` collapses that duplication.  It handles all the
//! standard typed CIR opcodes natively, plus a few common extensions
//! (`load_mem` / `store_mem` for tape-like memory models).  Per-language
//! customization is two things:
//!
//! 1. **A builtin callback registry** (`Arc<Mutex<HashMap<String,
//!    Arc<dyn Fn(&[Value]) -> Value + Send + Sync>>>>`).  Each language
//!    constructs a `GenericCirJit`, then registers its builtins
//!    (`print_i64`, `putchar`, `make_cons`, …).  The JIT's
//!    `CALL_BUILTIN` opcode dispatches through the registry.
//!
//! 2. **Optional linear memory** for languages that use the tape model
//!    (Brainfuck).  `with_linear_memory(tape_size)` allocates a
//!    `Vec<u8>` per call and routes `load_mem` / `store_mem` ops there.
//!
//! Every language with a typed IIR (`FullyTyped` `IIRFunction`s) now
//! gets a real JIT essentially for free — just register the builtins.
//!
//! # Bytecode format
//!
//! Linear sequence of variable-length instructions.  Opcode tags use
//! the values in the [`opcode`] sub-module.  Register indices are 1
//! byte (256 i64 registers).  i64 constants are 8 bytes LE.  Branch
//! offsets are i16 LE, relative to the byte position immediately after
//! the offset bytes.  Builtin-name lookups are pre-interned into 2-byte
//! LE indices into a per-binary builtin table (encoded as a prefix to
//! the bytecode).
//!
//! # Bytecode prefix: builtin table
//!
//! The compiled `Vec<u8>` starts with a length-prefixed builtin name
//! table so the runtime knows which builtin to invoke for each
//! `CALL_BUILTIN <idx>` opcode:
//!
//! ```text
//! [u16 LE: n_builtins]
//! [n_builtins × (u16 LE name_len, name bytes)]
//! [bytecode payload]
//! ```
//!
//! This keeps `run()` self-contained — no per-call hash lookups by
//! string.  The 2-byte index supports up to 65 535 distinct builtins
//! per function, which is plenty.
//!
//! # Supported CIR opcodes
//!
//! - **Constants**: `const_{i8|i16|i32|i64|u8|u16|u32|u64|bool}` →
//!   `CONST_I64` (8-byte payload, sign/zero-extended at encode time).
//! - **Move**: `mov` → `MOV`.
//! - **Arithmetic** (i64 family): `add_{i*|u*}`, `sub_*`, `mul_*`,
//!   `div_*`, `neg_*` → `ADD_I64`, `SUB_I64`, etc.
//! - **Comparisons** (i64 family): `cmp_{eq|ne|lt|le|gt|ge}_{i*|u*|bool}`
//!   → `CMP_*_I64`.
//! - **Control flow**: `label`, `jmp`, `jmp_if_true`, `jmp_if_false`.
//! - **Returns**: `ret_{i*|u*}`, `ret_bool`, and `ret_void` preserve their
//!   runtime value category through `RET_I64`, `RET_BOOL`, and `RET_VOID`.
//! - **Builtins**: `call_builtin` → `CALL_BUILTIN <idx>`.
//! - **Optional linear memory**: `load_mem`, `store_mem` (when the
//!   GenericCirJit was constructed with a non-zero tape size).
//!
//! Anything else makes `compile()` return `None`, falling back to the
//! interpreter.
//!
//! # Threading
//!
//! All shared state (output, input, error slot, builtin registry) is
//! behind `Arc`.  `GenericCirJit` implements `Send + Sync`, satisfying
//! the `Backend` trait bound.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use vm_core::value::Value;

use crate::backend::{Backend, FunctionContext};
use crate::cir::{CIRInstr, CIROperand};

// ---------------------------------------------------------------------------
// Bytecode opcode tags
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) mod opcode {
    pub const CONST_I64:    u8 = 0x01;
    pub const MOV:          u8 = 0x02;
    pub const ADD_I64:      u8 = 0x10;
    pub const SUB_I64:      u8 = 0x11;
    pub const MUL_I64:      u8 = 0x12;
    pub const DIV_I64:      u8 = 0x13;
    pub const NEG_I64:      u8 = 0x14;
    /// `MASK_WIDTH <reg> <bits>` — mask `regs[reg]` to its low `bits` bits
    /// (`& ((1<<bits)-1)`).  Emitted right after a narrow-width arithmetic op
    /// (`add_u8`, `mul_u4`, …) so the compiled tier wraps mod-2ⁿ the same way
    /// vm-core's `mask_result` does for the interpreter tier (LANG-FULL E2).
    pub const MASK_WIDTH:   u8 = 0x15;
    pub const CMP_EQ_I64:   u8 = 0x20;
    pub const CMP_NE_I64:   u8 = 0x21;
    pub const CMP_LT_I64:   u8 = 0x22;
    pub const CMP_LE_I64:   u8 = 0x23;
    pub const CMP_GT_I64:   u8 = 0x24;
    pub const CMP_GE_I64:   u8 = 0x25;
    pub const JMP:          u8 = 0x30;
    pub const JMP_IF_FALSE: u8 = 0x31;
    pub const JMP_IF_TRUE:  u8 = 0x32;
    pub const LOAD_MEM:     u8 = 0x40;
    pub const STORE_MEM:    u8 = 0x41;
    pub const CALL_BUILTIN: u8 = 0x50;
    pub const RET_I64:      u8 = 0x60;
    pub const RET_VOID:     u8 = 0x61;
    pub const RET_BOOL:     u8 = 0x62;
}

/// Maximum bytes the compiled bytecode can grow to.  1 MiB is enough
/// for any reasonable function and protects against pathological
/// compile inputs.
const MAX_BYTECODE_BYTES: usize = 1 << 20;

/// Default fuel cap: 100 million backward jumps.
pub const DEFAULT_STEP_CAP: u64 = 100_000_000;

// ---------------------------------------------------------------------------
// Builtin callback type
// ---------------------------------------------------------------------------

/// A builtin callback registered with [`GenericCirJit::register_builtin`].
///
/// Receives the resolved argument values (everything in `srcs` after
/// the first slot, which is the builtin name) and returns the result
/// value (`Value::Null` for void builtins).
pub type BuiltinFn = Arc<dyn Fn(&[Value]) -> Value + Send + Sync + 'static>;

// ---------------------------------------------------------------------------
// GenericCirJit
// ---------------------------------------------------------------------------

/// A universal bytecode JIT backend that any language with a typed IIR
/// can plug into.  See the module-level documentation for design notes.
pub struct GenericCirJit {
    /// Builtin name → callback.  Each language registers its own
    /// builtins (`print_i64`, `putchar`, etc.) here before handing
    /// the backend to `JITCore`.
    builtins: Arc<Mutex<HashMap<String, BuiltinFn>>>,

    /// Optional linear memory size.  When non-zero, the JIT allocates a
    /// fresh `Vec<u8>` of this size per `run()` call and routes
    /// `load_mem` / `store_mem` opcodes there.  Brainfuck uses this for
    /// its tape; languages without linear memory (BASIC, Twig, …) leave
    /// it at zero and the load_mem / store_mem opcodes are refused at
    /// compile time.
    tape_size: usize,

    /// Step counter — bumped on each backward jump.  Shared with the
    /// surrounding VM wrapper so the interpreter and JIT paths agree on
    /// how much fuel was spent.
    pub steps: Arc<Mutex<u64>>,

    /// Error slot — set by the bytecode interpreter when a malformed
    /// bytecode read, out-of-bounds memory access, division-by-zero,
    /// or step-cap exhaustion occurs.  `Backend::run` cannot return a
    /// `Result`, so callers inspect this slot after execution.
    pub error: Arc<Mutex<Option<String>>>,

    /// Optional step cap.  `None` means unlimited.
    max_steps: Option<u64>,
}

impl GenericCirJit {
    /// Construct a fresh `GenericCirJit` with no builtins, no tape, and
    /// the default step cap.  Callers typically chain
    /// `.register_builtin(...)` / `.with_linear_memory(n)` etc.
    pub fn new() -> Self {
        GenericCirJit {
            builtins: Arc::new(Mutex::new(HashMap::new())),
            tape_size: 0,
            steps: Arc::new(Mutex::new(0)),
            error: Arc::new(Mutex::new(None)),
            max_steps: Some(DEFAULT_STEP_CAP),
        }
    }

    /// Register a builtin callback under `name`.  Replaces any existing
    /// callback with the same name.  The callback receives the
    /// resolved argument values (the original `call_builtin` srcs
    /// minus the first builtin-name slot).
    pub fn register_builtin<F>(&self, name: impl Into<String>, callback: F)
    where
        F: Fn(&[Value]) -> Value + Send + Sync + 'static,
    {
        let mut map = self.builtins.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(name.into(), Arc::new(callback));
    }

    /// Enable linear memory of the given size for `load_mem` /
    /// `store_mem` opcodes.  Returns `self` for chaining.
    pub fn with_linear_memory(mut self, tape_size: usize) -> Self {
        self.tape_size = tape_size;
        self
    }

    /// Replace the default step cap.  `None` disables the cap entirely
    /// (use with care — infinite loops will hang the VM).
    pub fn with_step_cap(mut self, max_steps: Option<u64>) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Get a clone of the step-counter handle so the wrapper VM can
    /// inspect it after execution.
    pub fn steps_handle(&self) -> Arc<Mutex<u64>> {
        Arc::clone(&self.steps)
    }

    /// Get a clone of the error-slot handle so the wrapper VM can
    /// inspect it after execution.
    pub fn error_handle(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.error)
    }

    fn set_error(&self, msg: impl Into<String>) {
        let mut slot = self.error.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            *slot = Some(msg.into());
        }
    }

    fn tick_step(&self) -> Result<(), String> {
        let mut s = self.steps.lock().unwrap_or_else(|e| e.into_inner());
        *s = s.saturating_add(1);
        if let Some(cap) = self.max_steps {
            if *s > cap {
                return Err(format!(
                    "GenericCirJit: step cap exceeded ({cap} backward jumps)"
                ));
            }
        }
        Ok(())
    }

    /// Snapshot the current builtin name table.  Used by `compile()` to
    /// freeze the registry into the bytecode prefix so `run()` doesn't
    /// re-hash names on each call.
    fn snapshot_builtins(&self) -> Vec<(String, BuiltinFn)> {
        let map = self.builtins.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<(String, BuiltinFn)> = map
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect();
        // Sort for deterministic indices — makes bytecode reproducible.
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }
}

impl Default for GenericCirJit {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for GenericCirJit {
    fn name(&self) -> &str {
        "generic-cir-jit"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        // Resolve builtins used by this function to indices.  Only the
        // names that appear in `call_builtin` srcs get assigned indices,
        // keeping the prefix small.
        //
        // The bare `compile` has no parameter context, so it compiles the
        // body with an empty param list (the function takes no arguments).
        // Callers that have an `IIRFunction` in hand should use
        // `compile_function` below so a function's parameters are bound.
        let snapshot = self.snapshot_builtins();
        compile_to_bytecode(ir, &snapshot, self.tape_size > 0, &[])
    }

    fn compile_function(&self, ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        // Same as `compile`, but the function's parameters are pre-bound to
        // registers `0..params.len()` in declaration order.  At `run` time
        // the incoming call arguments are seeded into exactly those
        // registers (see `run`), so a compiled function with parameters —
        // e.g. Nib's `double(x) -> x + x` — reads its arguments correctly.
        // This is what makes the JIT *generic*: any frontend whose functions
        // take arguments compiles and runs here with no per-language code.
        let snapshot = self.snapshot_builtins();
        compile_to_bytecode(ir, &snapshot, self.tape_size > 0, ctx.params)
    }

    fn run(&self, binary: &[u8], args: &[Value]) -> Value {
        // Decode the builtin name table.
        let mut pc = 0usize;
        if binary.len() < 2 {
            self.set_error("malformed bytecode: missing builtin table header");
            return Value::Null;
        }
        let n_builtins = u16::from_le_bytes([binary[0], binary[1]]) as usize;
        pc += 2;
        // Build the index → callback table by looking each name up in
        // the live registry.  We do the lookup once per run() rather
        // than baking it into the bytecode so that builtins registered
        // after compile() but before run() are still visible (rare,
        // but useful for tests).
        let mut builtin_table: Vec<BuiltinFn> = Vec::with_capacity(n_builtins);
        {
            let map = self.builtins.lock().unwrap_or_else(|e| e.into_inner());
            for _ in 0..n_builtins {
                if pc + 2 > binary.len() {
                    self.set_error("malformed bytecode: truncated name header");
                    return Value::Null;
                }
                let name_len = u16::from_le_bytes([binary[pc], binary[pc + 1]]) as usize;
                pc += 2;
                if pc + name_len > binary.len() {
                    self.set_error("malformed bytecode: truncated name bytes");
                    return Value::Null;
                }
                let name = match std::str::from_utf8(&binary[pc..pc + name_len]) {
                    Ok(s) => s.to_string(),
                    Err(_) => {
                        self.set_error("malformed bytecode: non-UTF-8 builtin name");
                        return Value::Null;
                    }
                };
                pc += name_len;
                match map.get(&name) {
                    Some(cb) => builtin_table.push(Arc::clone(cb)),
                    None => {
                        self.set_error(format!(
                            "GenericCirJit: builtin {name:?} not registered"
                        ));
                        return Value::Null;
                    }
                }
            }
        }

        let code_start = pc;
        let code = &binary[code_start..];

        // Per-call state.
        let mut regs: [i64; 256] = [0i64; 256];

        // Seed the parameter registers from the incoming call arguments.
        // `compile_function` pre-binds parameters to registers
        // `0..params.len()` in declaration order, and `compile_fn` passes the
        // arguments in that same order, so argument `i` lands in register `i`.
        // Non-integer values collapse to their `i64` view (the register file
        // is i64-only), and `.take(256)` keeps a long argument list from
        // walking off the fixed-size file — a defensive bound, never reached
        // in practice (a function cannot declare more than 256 registers).
        for (i, a) in args.iter().enumerate().take(regs.len()) {
            regs[i] = a.as_i64().unwrap_or(0);
        }

        let mut tape: Vec<u8> = if self.tape_size > 0 {
            vec![0u8; self.tape_size]
        } else {
            Vec::new()
        };

        let mut pc = 0usize; // pc into `code`, not `binary`

        while pc < code.len() {
            let op = code[pc];
            pc += 1;

            match op {
                opcode::CONST_I64 => {
                    if pc + 9 > code.len() {
                        self.set_error("malformed bytecode: truncated CONST_I64");
                        return Value::Null;
                    }
                    let reg = code[pc] as usize;
                    let val = i64::from_le_bytes([
                        code[pc + 1], code[pc + 2], code[pc + 3], code[pc + 4],
                        code[pc + 5], code[pc + 6], code[pc + 7], code[pc + 8],
                    ]);
                    pc += 9;
                    regs[reg] = val;
                }
                opcode::MOV => {
                    if pc + 2 > code.len() {
                        self.set_error("malformed bytecode: truncated MOV");
                        return Value::Null;
                    }
                    regs[code[pc] as usize] = regs[code[pc + 1] as usize];
                    pc += 2;
                }
                opcode::ADD_I64 | opcode::SUB_I64
                | opcode::MUL_I64 | opcode::DIV_I64 => {
                    if pc + 3 > code.len() {
                        self.set_error("malformed bytecode: truncated arith");
                        return Value::Null;
                    }
                    let dst = code[pc] as usize;
                    let a = regs[code[pc + 1] as usize];
                    let b = regs[code[pc + 2] as usize];
                    pc += 3;
                    let r = match op {
                        opcode::ADD_I64 => a.wrapping_add(b),
                        opcode::SUB_I64 => a.wrapping_sub(b),
                        opcode::MUL_I64 => a.wrapping_mul(b),
                        opcode::DIV_I64 => {
                            if b == 0 {
                                self.set_error("GenericCirJit: division by zero");
                                return Value::Null;
                            }
                            a.wrapping_div(b)
                        }
                        _ => unreachable!(),
                    };
                    regs[dst] = r;
                }
                opcode::NEG_I64 => {
                    if pc + 2 > code.len() {
                        self.set_error("malformed bytecode: truncated NEG_I64");
                        return Value::Null;
                    }
                    regs[code[pc] as usize] = regs[code[pc + 1] as usize].wrapping_neg();
                    pc += 2;
                }
                opcode::MASK_WIDTH => {
                    if pc + 2 > code.len() {
                        self.set_error("malformed bytecode: truncated MASK_WIDTH");
                        return Value::Null;
                    }
                    let reg = code[pc] as usize;
                    let bits = code[pc + 1] as u32;
                    pc += 2;
                    // `compile_to_bytecode` only ever emits a width in 1..=63
                    // (8/16/32), but `run` is a public entry on an opaque byte
                    // blob — validate before shifting so a hostile `bits >= 64`
                    // fails gracefully (like every other arm) instead of
                    // panicking on `1 << bits`.
                    if !(1..=63).contains(&bits) {
                        self.set_error("malformed bytecode: MASK_WIDTH bits out of range");
                        return Value::Null;
                    }
                    let mask = (1i64 << bits) - 1;
                    regs[reg] &= mask;
                }
                opcode::CMP_EQ_I64 | opcode::CMP_NE_I64
                | opcode::CMP_LT_I64 | opcode::CMP_LE_I64
                | opcode::CMP_GT_I64 | opcode::CMP_GE_I64 => {
                    if pc + 3 > code.len() {
                        self.set_error("malformed bytecode: truncated cmp");
                        return Value::Null;
                    }
                    let dst = code[pc] as usize;
                    let a = regs[code[pc + 1] as usize];
                    let b = regs[code[pc + 2] as usize];
                    pc += 3;
                    let cond = match op {
                        opcode::CMP_EQ_I64 => a == b,
                        opcode::CMP_NE_I64 => a != b,
                        opcode::CMP_LT_I64 => a < b,
                        opcode::CMP_LE_I64 => a <= b,
                        opcode::CMP_GT_I64 => a > b,
                        opcode::CMP_GE_I64 => a >= b,
                        _ => unreachable!(),
                    };
                    regs[dst] = if cond { 1 } else { 0 };
                }
                opcode::JMP => {
                    if pc + 2 > code.len() {
                        self.set_error("malformed bytecode: truncated JMP");
                        return Value::Null;
                    }
                    let off = i16::from_le_bytes([code[pc], code[pc + 1]]) as isize;
                    pc += 2;
                    if off < 0 {
                        if let Err(m) = self.tick_step() {
                            self.set_error(m);
                            return Value::Null;
                        }
                    }
                    pc = (pc as isize + off) as usize;
                }
                opcode::JMP_IF_FALSE | opcode::JMP_IF_TRUE => {
                    if pc + 3 > code.len() {
                        self.set_error("malformed bytecode: truncated cond jmp");
                        return Value::Null;
                    }
                    let cond = code[pc] as usize;
                    let off = i16::from_le_bytes([code[pc + 1], code[pc + 2]]) as isize;
                    pc += 3;
                    let truthy = regs[cond] != 0;
                    let take = if op == opcode::JMP_IF_FALSE { !truthy } else { truthy };
                    if take {
                        if off < 0 {
                            if let Err(m) = self.tick_step() {
                                self.set_error(m);
                                return Value::Null;
                            }
                        }
                        pc = (pc as isize + off) as usize;
                    }
                }
                opcode::LOAD_MEM => {
                    if pc + 2 > code.len() {
                        self.set_error("malformed bytecode: truncated LOAD_MEM");
                        return Value::Null;
                    }
                    let dst = code[pc] as usize;
                    let addr = regs[code[pc + 1] as usize];
                    pc += 2;
                    // OOB reads return 0 (lazy-infinite-tape convention,
                    // matches Brainfuck's interpreter).
                    let v = if addr < 0 || (addr as usize) >= tape.len() {
                        0i64
                    } else {
                        tape[addr as usize] as i64
                    };
                    regs[dst] = v;
                }
                opcode::STORE_MEM => {
                    if pc + 2 > code.len() {
                        self.set_error("malformed bytecode: truncated STORE_MEM");
                        return Value::Null;
                    }
                    let addr = regs[code[pc] as usize];
                    let v = regs[code[pc + 1] as usize];
                    pc += 2;
                    if addr < 0 || (addr as usize) >= tape.len() {
                        self.set_error(format!(
                            "GenericCirJit: store_mem address {addr} out of bounds [0, {})",
                            tape.len()
                        ));
                        return Value::Null;
                    }
                    tape[addr as usize] = (v & 0xFF) as u8;
                }
                opcode::CALL_BUILTIN => {
                    // [builtin_idx:u16_le, dst:u8 (255 = no dest), n_args:u8, arg_regs:n_args bytes]
                    if pc + 4 > code.len() {
                        self.set_error("malformed bytecode: truncated CALL_BUILTIN header");
                        return Value::Null;
                    }
                    let idx = u16::from_le_bytes([code[pc], code[pc + 1]]) as usize;
                    let dst = code[pc + 2];
                    let n_args = code[pc + 3] as usize;
                    pc += 4;
                    if pc + n_args > code.len() {
                        self.set_error("malformed bytecode: truncated CALL_BUILTIN args");
                        return Value::Null;
                    }
                    let mut args: Vec<Value> = Vec::with_capacity(n_args);
                    for i in 0..n_args {
                        args.push(Value::Int(regs[code[pc + i] as usize]));
                    }
                    pc += n_args;
                    let Some(cb) = builtin_table.get(idx) else {
                        self.set_error(format!(
                            "GenericCirJit: builtin index {idx} out of range"
                        ));
                        return Value::Null;
                    };
                    let result = cb(&args);
                    if dst != 0xFF {
                        // Coerce result to i64 — generic JIT carries
                        // everything as i64 in regs.  Non-i64 results
                        // are flattened via `as_i64()` (which handles
                        // Int / Bool).
                        let n = result.as_i64().unwrap_or(0);
                        regs[dst as usize] = n;
                    }
                }
                opcode::RET_I64 => {
                    if pc + 1 > code.len() {
                        self.set_error("malformed bytecode: truncated RET_I64");
                        return Value::Null;
                    }
                    return Value::Int(regs[code[pc] as usize]);
                }
                opcode::RET_BOOL => {
                    if pc + 1 > code.len() {
                        self.set_error("malformed bytecode: truncated RET_BOOL");
                        return Value::Null;
                    }
                    return Value::Bool(regs[code[pc] as usize] != 0);
                }
                opcode::RET_VOID => {
                    return Value::Null;
                }
                _ => {
                    self.set_error(format!(
                        "GenericCirJit: unknown opcode 0x{op:02x} at pc {}",
                        pc - 1
                    ));
                    return Value::Null;
                }
            }
        }

        Value::Null
    }
}

// ---------------------------------------------------------------------------
// Bytecode compiler
// ---------------------------------------------------------------------------

fn lookup_or_alloc_var(
    name: &str,
    reg_map: &mut HashMap<String, u8>,
    next_reg: &mut u16,
) -> Option<u8> {
    if let Some(&idx) = reg_map.get(name) {
        return Some(idx);
    }
    if *next_reg >= 256 {
        return None;
    }
    let idx = *next_reg as u8;
    *next_reg += 1;
    reg_map.insert(name.to_string(), idx);
    Some(idx)
}

fn resolve_operand(
    op: &CIROperand,
    reg_map: &mut HashMap<String, u8>,
    next_reg: &mut u16,
    out: &mut Vec<u8>,
) -> Option<u8> {
    match op {
        CIROperand::Var(name) => lookup_or_alloc_var(name, reg_map, next_reg),
        CIROperand::Int(n) => {
            if *next_reg >= 256 {
                return None;
            }
            let idx = *next_reg as u8;
            *next_reg += 1;
            out.push(opcode::CONST_I64);
            out.push(idx);
            out.extend_from_slice(&n.to_le_bytes());
            Some(idx)
        }
        CIROperand::Bool(b) => {
            if *next_reg >= 256 {
                return None;
            }
            let idx = *next_reg as u8;
            *next_reg += 1;
            out.push(opcode::CONST_I64);
            out.push(idx);
            out.extend_from_slice(&(*b as i64).to_le_bytes());
            Some(idx)
        }
        CIROperand::Float(_) => None,
    }
}

/// Compile a CIR instruction stream into the JIT bytecode.
///
/// The output buffer has the shape: `[builtin_table | code]` where
/// `builtin_table` is a length-prefixed list of names used by this
/// function (each `call_builtin` op's first src), and `code` is the
/// linear bytecode.
///
/// `params` is the function's parameter list (`(name, type)` in declaration
/// order).  Each parameter name is pre-allocated a register *before* the body
/// is walked, so the parameters deterministically occupy registers
/// `0..params.len()`.  `run` relies on this: it seeds those registers from the
/// incoming call arguments.  Pass `&[]` for a function that takes no arguments.
/// If a CIR op's width suffix is a narrow unsigned width the compiled tier can
/// represent (`_u8`/`_u16`/`_u32`), return the bit-width its result must be
/// masked to.  `_i64`/`_u64`/`_i32`/bool/… and anything else return `None` (full
/// machine width).  `u4` is not in the CIR allowlist, so a `u4`-typed op never
/// reaches the JIT — it specialises to the generic path and runs on the
/// interpreter tier (vm-core), which masks it; the observable wrap is identical.
/// Signed narrow types are intentionally not masked (two's-complement wrap needs
/// sign-extension; the LANG-FULL frontends use the unsigned widths).
fn narrow_width_bits(op: &str) -> Option<u8> {
    if op.ends_with("_u8") {
        Some(8)
    } else if op.ends_with("_u16") {
        Some(16)
    } else if op.ends_with("_u32") {
        Some(32)
    } else {
        None
    }
}

/// Emit a `MASK_WIDTH <reg> <bits>` after a narrow-width arithmetic op so the
/// compiled tier wraps mod-2ⁿ, mirroring vm-core's `mask_result`.
fn emit_width_mask(code: &mut Vec<u8>, reg: u8, op: &str) {
    if let Some(bits) = narrow_width_bits(op) {
        code.push(opcode::MASK_WIDTH);
        code.push(reg);
        code.push(bits);
    }
}

fn compile_to_bytecode(
    ir: &[CIRInstr],
    builtin_snapshot: &[(String, BuiltinFn)],
    has_linear_memory: bool,
    params: &[(String, String)],
) -> Option<Vec<u8>> {
    let mut code: Vec<u8> = Vec::with_capacity(ir.len() * 4);
    let mut reg_map: HashMap<String, u8> = HashMap::new();
    let mut next_reg: u16 = 0;
    let mut label_pos: HashMap<String, usize> = HashMap::new();
    let mut fixups: Vec<(usize, String)> = Vec::new();

    // Pre-bind the parameters to registers 0, 1, 2, … in declaration order.
    // Doing this before the instruction walk guarantees `run` can map
    // argument `i` to register `i`.  A duplicate parameter name (malformed
    // IR) or a function with more than 256 parameters makes the whole
    // function uncompilable — `run` would not be able to seed it consistently.
    for (name, _ty) in params {
        if reg_map.contains_key(name) {
            return None;
        }
        lookup_or_alloc_var(name, &mut reg_map, &mut next_reg)?;
    }

    // Track which builtin names this function actually uses; we'll emit
    // only those in the bytecode prefix.  Maps name → local index.
    let mut used_builtins: HashMap<String, u16> = HashMap::new();
    let mut builtin_order: Vec<String> = Vec::new();

    for instr in ir {
        if code.len() > MAX_BYTECODE_BYTES {
            return None;
        }
        let op = instr.op.as_str();

        // Const family (any integer width or bool) → CONST_I64.
        if op.starts_with("const_") {
            let dest = instr.dest.as_deref()?;
            let didx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let n = match instr.srcs.first()? {
                CIROperand::Int(n) => *n,
                CIROperand::Bool(b) => *b as i64,
                _ => return None,
            };
            code.push(opcode::CONST_I64);
            code.push(didx);
            code.extend_from_slice(&n.to_le_bytes());
            continue;
        }

        if op == "mov" {
            let dest = instr.dest.as_deref()?;
            let didx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let s = resolve_operand(instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut code)?;
            code.push(opcode::MOV);
            code.push(didx);
            code.push(s);
            continue;
        }

        // Arithmetic — accept any integer-width suffix.
        if op.starts_with("add_") || op.starts_with("sub_")
            || op.starts_with("mul_") || op.starts_with("div_")
        {
            let bin_opc = if op.starts_with("add_") {
                opcode::ADD_I64
            } else if op.starts_with("sub_") {
                opcode::SUB_I64
            } else if op.starts_with("mul_") {
                opcode::MUL_I64
            } else {
                opcode::DIV_I64
            };
            // Refuse float arithmetic — we carry everything as i64.
            if op.ends_with("_f32") || op.ends_with("_f64") {
                return None;
            }
            let dest = instr.dest.as_deref()?;
            let didx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let a = resolve_operand(instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut code)?;
            let b = resolve_operand(instr.srcs.get(1)?, &mut reg_map, &mut next_reg, &mut code)?;
            code.push(bin_opc);
            code.push(didx);
            code.push(a);
            code.push(b);
            emit_width_mask(&mut code, didx, op);
            continue;
        }

        if op.starts_with("neg_") {
            if op.ends_with("_f32") || op.ends_with("_f64") {
                return None;
            }
            let dest = instr.dest.as_deref()?;
            let didx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let s = resolve_operand(instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut code)?;
            code.push(opcode::NEG_I64);
            code.push(didx);
            code.push(s);
            emit_width_mask(&mut code, didx, op);
            continue;
        }

        // Comparisons — accept any integer-width or bool suffix.
        if let Some(cmp_op) = parse_cmp_op(op) {
            let dest = instr.dest.as_deref()?;
            let didx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let a = resolve_operand(instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut code)?;
            let b = resolve_operand(instr.srcs.get(1)?, &mut reg_map, &mut next_reg, &mut code)?;
            code.push(cmp_op);
            code.push(didx);
            code.push(a);
            code.push(b);
            continue;
        }

        if op == "label" {
            let name = instr.srcs.first()?.as_var()?.to_string();
            label_pos.insert(name, code.len());
            continue;
        }

        if op == "jmp" {
            let target = instr.srcs.first()?.as_var()?.to_string();
            code.push(opcode::JMP);
            let placeholder = code.len();
            code.extend_from_slice(&[0u8, 0u8]);
            fixups.push((placeholder, target));
            continue;
        }

        if op == "jmp_if_true" || op == "jmp_if_false" {
            let cond = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut code,
            )?;
            let target = instr.srcs.get(1)?.as_var()?.to_string();
            code.push(if op == "jmp_if_true" {
                opcode::JMP_IF_TRUE
            } else {
                opcode::JMP_IF_FALSE
            });
            code.push(cond);
            let placeholder = code.len();
            code.extend_from_slice(&[0u8, 0u8]);
            fixups.push((placeholder, target));
            continue;
        }

        // load_mem / store_mem — only valid when a tape is configured.
        if op == "load_mem" {
            if !has_linear_memory {
                return None;
            }
            let dest = instr.dest.as_deref()?;
            let didx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let addr = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut code,
            )?;
            code.push(opcode::LOAD_MEM);
            code.push(didx);
            code.push(addr);
            continue;
        }
        if op == "store_mem" {
            if !has_linear_memory {
                return None;
            }
            let addr = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut code,
            )?;
            let v = resolve_operand(
                instr.srcs.get(1)?, &mut reg_map, &mut next_reg, &mut code,
            )?;
            code.push(opcode::STORE_MEM);
            code.push(addr);
            code.push(v);
            continue;
        }

        if op == "call_builtin" {
            let name = instr.srcs.first()?.as_var()?.to_string();
            // Look up the builtin in the snapshot to ensure it's
            // registered.  If not, refuse to compile so the interpreter
            // path can handle it.
            let registered = builtin_snapshot.iter().any(|(n, _)| n == &name);
            if !registered {
                return None;
            }
            // Assign a local index for this name.
            let idx = if let Some(&i) = used_builtins.get(&name) {
                i
            } else {
                let i = builtin_order.len() as u16;
                used_builtins.insert(name.clone(), i);
                builtin_order.push(name.clone());
                i
            };

            // Encode args (everything after srcs[0] = name).
            let mut arg_regs: Vec<u8> = Vec::with_capacity(instr.srcs.len().saturating_sub(1));
            for arg in instr.srcs.iter().skip(1) {
                let r = resolve_operand(arg, &mut reg_map, &mut next_reg, &mut code)?;
                arg_regs.push(r);
            }
            let dst_byte = match instr.dest.as_deref() {
                Some(d) => lookup_or_alloc_var(d, &mut reg_map, &mut next_reg)?,
                None => 0xFFu8,
            };
            if arg_regs.len() > 255 {
                return None;
            }
            code.push(opcode::CALL_BUILTIN);
            code.extend_from_slice(&idx.to_le_bytes());
            code.push(dst_byte);
            code.push(arg_regs.len() as u8);
            code.extend_from_slice(&arg_regs);
            continue;
        }

        // Returns — keep bool returns distinct from integer returns so a JIT
        // call remains type-compatible with VM comparisons and branches.
        if op == "ret_void" {
            code.push(opcode::RET_VOID);
            continue;
        }
        if op.starts_with("ret_") {
            if op.ends_with("_f32") || op.ends_with("_f64") {
                return None;
            }
            let v = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut code,
            )?;
            code.push(if op == "ret_bool" {
                opcode::RET_BOOL
            } else {
                opcode::RET_I64
            });
            code.push(v);
            continue;
        }

        // Unknown opcode — refuse.
        return None;
    }

    // Patch label fixups.
    for (placeholder, target) in fixups {
        let target_pos = label_pos.get(&target)?;
        let from = (placeholder + 2) as isize;
        let to = *target_pos as isize;
        let off = to - from;
        if !(i16::MIN as isize..=i16::MAX as isize).contains(&off) {
            return None;
        }
        let off16 = (off as i16).to_le_bytes();
        code[placeholder] = off16[0];
        code[placeholder + 1] = off16[1];
    }

    // Prepend the builtin name table.
    let mut out: Vec<u8> = Vec::with_capacity(code.len() + 2 + builtin_order.len() * 16);
    out.extend_from_slice(&(builtin_order.len() as u16).to_le_bytes());
    for name in &builtin_order {
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
    }
    out.extend_from_slice(&code);

    Some(out)
}

/// Parse a `cmp_*_*` mnemonic into its opcode, accepting any integer
/// width or bool suffix.
fn parse_cmp_op(op: &str) -> Option<u8> {
    if !op.starts_with("cmp_") {
        return None;
    }
    // Must end with one of the supported type suffixes.
    let supported_suffix = ["_i8", "_i16", "_i32", "_i64",
                            "_u8", "_u16", "_u32", "_u64",
                            "_bool"];
    if !supported_suffix.iter().any(|s| op.ends_with(s)) {
        return None;
    }
    if op.starts_with("cmp_eq_") { return Some(opcode::CMP_EQ_I64); }
    if op.starts_with("cmp_ne_") { return Some(opcode::CMP_NE_I64); }
    if op.starts_with("cmp_lt_") { return Some(opcode::CMP_LT_I64); }
    if op.starts_with("cmp_le_") { return Some(opcode::CMP_LE_I64); }
    if op.starts_with("cmp_gt_") { return Some(opcode::CMP_GT_I64); }
    if op.starts_with("cmp_ge_") { return Some(opcode::CMP_GE_I64); }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cir::CIRInstr;

    fn jit_no_builtins() -> GenericCirJit {
        GenericCirJit::new()
    }

    #[test]
    fn name_is_generic_cir_jit() {
        assert_eq!(jit_no_builtins().name(), "generic-cir-jit");
    }

    #[test]
    fn compile_const_i64_then_ret_i64() {
        let cir = vec![
            CIRInstr::new("const_i64", Some("v0"), vec![CIROperand::Int(42)], "i64"),
            CIRInstr::new("ret_i64",   None::<&str>, vec![CIROperand::Var("v0".into())], "i64"),
        ];
        let j = jit_no_builtins();
        let bin = j.compile(&cir).unwrap();
        assert_eq!(j.run(&bin, &[]).as_i64(), Some(42));
    }

    #[test]
    fn add_i64_works() {
        let cir = vec![
            CIRInstr::new("const_i64", Some("a"), vec![CIROperand::Int(30)], "i64"),
            CIRInstr::new("const_i64", Some("b"), vec![CIROperand::Int(12)], "i64"),
            CIRInstr::new("add_i64",   Some("c"),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i64"),
            CIRInstr::new("ret_i64",   None::<&str>, vec![CIROperand::Var("c".into())], "i64"),
        ];
        let j = jit_no_builtins();
        let bin = j.compile(&cir).unwrap();
        assert_eq!(j.run(&bin, &[]).as_i64(), Some(42));
    }

    // ---- E2: narrow-width register arithmetic wraps in the compiled tier ----

    /// Compile `op a b` (binary) at the given CIR width and run it.
    fn run_binop(op: &str, a: i64, b: i64) -> i64 {
        let cir = vec![
            CIRInstr::new("const_i64", Some("a"), vec![CIROperand::Int(a)], "i64"),
            CIRInstr::new("const_i64", Some("b"), vec![CIROperand::Int(b)], "i64"),
            CIRInstr::new(op, Some("c"),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i64"),
            CIRInstr::new("ret_i64", None::<&str>, vec![CIROperand::Var("c".into())], "i64"),
        ];
        let j = jit_no_builtins();
        let bin = j.compile(&cir).expect("compiles");
        j.run(&bin, &[]).as_i64().expect("i64 result")
    }

    #[test]
    fn u8_arithmetic_wraps_in_compiled_jit() {
        assert_eq!(run_binop("add_u8", 200, 100), 44);   // 300 & 0xFF
        assert_eq!(run_binop("mul_u8", 16, 16), 0);      // 256 & 0xFF
        assert_eq!(run_binop("sub_u8", 0, 1), 255);      // -1 & 0xFF
        assert_eq!(run_binop("add_u8", 255, 1), 0);      // cell wrap
    }

    #[test]
    fn u16_and_u32_wrap_in_compiled_jit() {
        assert_eq!(run_binop("add_u16", 60000, 10000), 70000 & 0xFFFF); // 4464
        assert_eq!(run_binop("mul_u32", 0x1_0000, 0x1_0000), 0);        // 2^32 & 0xFFFF_FFFF
    }

    #[test]
    fn neg_u8_wraps_in_compiled_jit() {
        // neg is unary: -a masked to a byte.  -5 & 0xFF == 251.
        let cir = vec![
            CIRInstr::new("const_i64", Some("a"), vec![CIROperand::Int(5)], "i64"),
            CIRInstr::new("neg_u8", Some("c"), vec![CIROperand::Var("a".into())], "i64"),
            CIRInstr::new("ret_i64", None::<&str>, vec![CIROperand::Var("c".into())], "i64"),
        ];
        let j = jit_no_builtins();
        let bin = j.compile(&cir).expect("compiles");
        assert_eq!(j.run(&bin, &[]).as_i64(), Some(251));
    }

    #[test]
    fn i64_width_does_not_mask_in_compiled_jit() {
        // The mask only fires for narrow unsigned suffixes; i64 keeps full width.
        assert_eq!(run_binop("add_i64", 200, 100), 300);
        assert_eq!(run_binop("mul_i64", 16, 16), 256);
    }

    /// `double(x) -> x + x`, compiled via `compile_function` so its single
    /// parameter is bound to register 0, then run with `x = 21`.  This is the
    /// regression test for the param-seeding fix: before it, `run` ignored its
    /// `args` and `x` read as the zero-initialised register, so the function
    /// returned 0 instead of 42.  (Nib's `double(21)` on the JIT was the
    /// real-world symptom.)
    #[test]
    fn compiled_function_reads_its_argument() {
        let cir = vec![
            CIRInstr::new("add_i64", Some("r"),
                vec![CIROperand::Var("x".into()), CIROperand::Var("x".into())], "i64"),
            CIRInstr::new("ret_i64", None::<&str>, vec![CIROperand::Var("r".into())], "i64"),
        ];
        let params = vec![("x".to_string(), "i64".to_string())];
        let ctx = FunctionContext { name: "double", params: &params, return_type: "i64" };
        let j = jit_no_builtins();
        let bin = j.compile_function(&ctx, &cir).unwrap();
        assert_eq!(j.run(&bin, &[Value::Int(21)]).as_i64(), Some(42));
        // A bare `compile` (no param context) leaves `x` an unbound register,
        // so the same body returns 0 — the contrast the fix turns on.
        let bin0 = j.compile(&cir).unwrap();
        assert_eq!(j.run(&bin0, &[Value::Int(21)]).as_i64(), Some(0));
    }

    #[test]
    fn compiled_boolean_function_preserves_boolean_return_type() {
        // A caller can compare a procedure result with a boolean literal only
        // when the compiled tier returns `Value::Bool`, not its integer carrier.
        let cir = vec![
            CIRInstr::new("const_bool", Some("result"), vec![CIROperand::Bool(false)], "bool"),
            CIRInstr::new(
                "cmp_eq_bool",
                Some("inverted"),
                vec![CIROperand::Var("p".into()), CIROperand::Bool(false)],
                "bool",
            ),
            CIRInstr::new(
                "mov",
                Some("result"),
                vec![CIROperand::Var("inverted".into())],
                "bool",
            ),
            CIRInstr::new(
                "ret_bool",
                None::<&str>,
                vec![CIROperand::Var("result".into())],
                "bool",
            ),
        ];
        let params = vec![("p".to_string(), "bool".to_string())];
        let ctx = FunctionContext { name: "neg", params: &params, return_type: "bool" };
        let j = jit_no_builtins();
        let bin = j.compile_function(&ctx, &cir).expect("compiles");

        assert_eq!(j.run(&bin, &[Value::Bool(false)]), Value::Bool(true));
        assert_eq!(j.run(&bin, &[Value::Bool(true)]), Value::Bool(false));
    }

    /// Two parameters must land in declaration order — argument `i` → register
    /// `i` — even when the body references the *second* parameter first.
    /// `sub(a, b) -> a - b` referenced as `b`-first would scramble the result
    /// if `run` keyed off first-appearance instead of the pre-bound param regs.
    #[test]
    fn two_params_map_to_args_in_declaration_order() {
        let cir = vec![
            // Reference `b` before `a` to prove ordering comes from the param
            // pre-binding, not from first-use in the body.
            CIRInstr::new("sub_i64", Some("r"),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i64"),
            CIRInstr::new("ret_i64", None::<&str>, vec![CIROperand::Var("r".into())], "i64"),
        ];
        let params = vec![("a".to_string(), "i64".to_string()),
                          ("b".to_string(), "i64".to_string())];
        let ctx = FunctionContext { name: "sub", params: &params, return_type: "i64" };
        let j = jit_no_builtins();
        let bin = j.compile_function(&ctx, &cir).unwrap();
        // sub(50, 8) = 42, not 8 - 50.
        assert_eq!(j.run(&bin, &[Value::Int(50), Value::Int(8)]).as_i64(), Some(42));
    }

    /// A duplicate parameter name is malformed IR — `compile_function` must
    /// refuse it rather than silently alias two params to one register.
    #[test]
    fn duplicate_parameter_name_is_uncompilable() {
        let cir = vec![
            CIRInstr::new("ret_i64", None::<&str>, vec![CIROperand::Var("x".into())], "i64"),
        ];
        let params = vec![("x".to_string(), "i64".to_string()),
                          ("x".to_string(), "i64".to_string())];
        let ctx = FunctionContext { name: "bad", params: &params, return_type: "i64" };
        assert!(jit_no_builtins().compile_function(&ctx, &cir).is_none());
    }

    #[test]
    fn builtin_dispatch_to_registered_callback() {
        let captured: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
        let j = GenericCirJit::new();
        {
            let captured = Arc::clone(&captured);
            j.register_builtin("record", move |args| {
                let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
                captured.lock().unwrap().push(n);
                Value::Null
            });
        }
        let cir = vec![
            CIRInstr::new("const_i64", Some("x"), vec![CIROperand::Int(7)], "i64"),
            CIRInstr::new("call_builtin", None::<&str>,
                vec![CIROperand::Var("record".into()), CIROperand::Var("x".into())],
                "void"),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let bin = j.compile(&cir).unwrap();
        j.run(&bin, &[]);
        assert_eq!(*captured.lock().unwrap(), vec![7]);
    }

    #[test]
    fn unregistered_builtin_refuses_compile() {
        let j = jit_no_builtins();
        let cir = vec![CIRInstr::new(
            "call_builtin",
            None::<&str>,
            vec![CIROperand::Var("nope".into())],
            "void",
        )];
        assert!(j.compile(&cir).is_none());
    }

    #[test]
    fn float_arith_refuses_compile() {
        let j = jit_no_builtins();
        let cir = vec![CIRInstr::new(
            "add_f64",
            Some("v"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "f64",
        )];
        assert!(j.compile(&cir).is_none());
    }

    #[test]
    fn load_mem_refused_without_linear_memory() {
        let j = jit_no_builtins();
        let cir = vec![CIRInstr::new(
            "load_mem",
            Some("v"),
            vec![CIROperand::Var("addr".into())],
            "u8",
        )];
        assert!(j.compile(&cir).is_none());
    }

    #[test]
    fn load_mem_works_with_linear_memory() {
        // Build: addr=0; store_mem(addr, 65); load_mem(addr) -> v; ret v
        let cir = vec![
            CIRInstr::new("const_i64", Some("addr"), vec![CIROperand::Int(0)], "u32"),
            CIRInstr::new("const_i64", Some("val"),  vec![CIROperand::Int(65)], "u8"),
            CIRInstr::new("store_mem", None::<&str>,
                vec![CIROperand::Var("addr".into()), CIROperand::Var("val".into())], "void"),
            CIRInstr::new("load_mem",  Some("v"),
                vec![CIROperand::Var("addr".into())], "u8"),
            CIRInstr::new("ret_i64",   None::<&str>,
                vec![CIROperand::Var("v".into())], "i64"),
        ];
        let j = GenericCirJit::new().with_linear_memory(16);
        let bin = j.compile(&cir).unwrap();
        assert_eq!(j.run(&bin, &[]).as_i64(), Some(65));
    }

    #[test]
    fn cmp_lt_then_jmp_if_false() {
        // Build: a=1; b=2; c = (a < b); if !c jmp END; ret 42; END: ret 0
        let cir = vec![
            CIRInstr::new("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
            CIRInstr::new("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
            CIRInstr::new("cmp_lt_i64", Some("c"),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "bool"),
            CIRInstr::new("jmp_if_false", None::<&str>,
                vec![CIROperand::Var("c".into()), CIROperand::Var("END".into())], "void"),
            CIRInstr::new("const_i64", Some("r1"), vec![CIROperand::Int(42)], "i64"),
            CIRInstr::new("ret_i64", None::<&str>, vec![CIROperand::Var("r1".into())], "i64"),
            CIRInstr::new("label", None::<&str>, vec![CIROperand::Var("END".into())], "void"),
            CIRInstr::new("const_i64", Some("r0"), vec![CIROperand::Int(0)], "i64"),
            CIRInstr::new("ret_i64", None::<&str>, vec![CIROperand::Var("r0".into())], "i64"),
        ];
        let j = jit_no_builtins();
        let bin = j.compile(&cir).unwrap();
        assert_eq!(j.run(&bin, &[]).as_i64(), Some(42));
    }

    #[test]
    fn division_by_zero_sets_error_slot() {
        let cir = vec![
            CIRInstr::new("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
            CIRInstr::new("const_i64", Some("b"), vec![CIROperand::Int(0)], "i64"),
            CIRInstr::new("div_i64",   Some("c"),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i64"),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let j = jit_no_builtins();
        let bin = j.compile(&cir).unwrap();
        j.run(&bin, &[]);
        let e = j.error_handle().lock().unwrap().clone();
        assert!(e.as_deref().map(|s| s.contains("division by zero")).unwrap_or(false),
            "expected division-by-zero error, got: {e:?}");
    }
}
