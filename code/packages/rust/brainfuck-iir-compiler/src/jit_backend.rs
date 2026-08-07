// The complex tuple type is an internal JIT backend signature; a type alias
// would not make it clearer.
#![allow(clippy::type_complexity)]
//! `BrainfuckCirJit` — a real [`jit_core::backend::Backend`] for Brainfuck.
//!
//! # What this is
//!
//! When `BrainfuckVM::new(true, ...)` is used, this backend is handed to
//! `JITCore`.  Phase 1 of `JITCore::execute_with_jit` calls `compile()`
//! once (Brainfuck IIR is `FullyTyped` from birth, so the threshold-zero
//! eager-compile path fires), which translates the CIR instruction
//! stream of `main` into a packed register-machine **bytecode**.  When
//! `vm-core` later dispatches `main`, the registered JIT handler invokes
//! [`Self::run`], which interprets that bytecode in a tight loop —
//! bypassing `vm-core`'s generic IIR dispatch entirely.
//!
//! # Is this a "real" JIT?
//!
//! In the classic, historical sense — yes.  This is the same shape that
//! the JVM (Ignition tier), Smalltalk-80, V8's Ignition, Lua, and a long
//! list of other production JITs use as their first tier: translate a
//! high-level IR to a compact register-based bytecode, then interpret
//! that bytecode in a specialised inner loop.  The bytecode here is
//! denser than the input CIR (register indices are 1 byte, branch
//! offsets are i16, no string-keyed lookups) and the dispatch loop has
//! one `match` over 14 opcodes instead of `vm-core`'s generic
//! `HashMap<String, OpcodeHandler>` lookup per instruction.
//!
//! What this is **not** is a native-code JIT (Cranelift, hand-rolled
//! x86_64/aarch64).  That's a separate piece of work — once a backend
//! grows real machine-code generation, swapping it in here is the only
//! change needed.
//!
//! # Why ship this backend specifically for Brainfuck?
//!
//! Brainfuck has a tiny, fully-typed CIR vocabulary, so a custom backend
//! is small enough to be a self-contained file (~ 400 lines).  Two
//! Brainfuck-specific bits force this to live here rather than in
//! `jit-core`:
//!
//! 1. **Tape memory model.**  `load_mem` / `store_mem` are
//!    Brainfuck-defined opcodes; `vm-core` only sees them because the
//!    `BrainfuckVM` wrapper registers custom opcode handlers.  This
//!    backend owns its own `Vec<u8>` tape per call, with the same
//!    bounds semantics (oob reads → 0, oob writes → error).
//! 2. **`putchar` / `getchar` builtins.**  The `JITCore`-registered JIT
//!    handler has signature `Fn(&[Value]) -> Value` — no access to the
//!    `VMCore`'s builtin registry.  This backend captures the same
//!    `Arc<Mutex<…>>` I/O buffers that the interpreter path uses, so
//!    both execution paths read from the same input and write to the
//!    same output.
//!
//! # Bytecode format
//!
//! A linear sequence of variable-length instructions.  Opcode tags use
//! the values listed in the [`opcode`] sub-module.  Register indices
//! are 1 byte (256 registers max — Brainfuck only uses 4).  Branch
//! offsets are `i16` little-endian, relative to the byte position
//! immediately after the offset bytes.  Const literals are encoded at
//! their natural width: `u8` as 1 byte, `u32` as 4 little-endian
//! bytes.  The full opcode → operand layout lives in the
//! [`compile_to_bytecode`] function's comments.
//!
//! # Error reporting
//!
//! [`jit_core::backend::Backend::run`] has signature
//! `fn(&self, &[u8], &[Value]) -> Value` — there is **no** way to
//! return a `Result`.  To surface fuel-cap and out-of-bounds errors
//! back to the caller, the backend captures an
//! `Arc<Mutex<Option<String>>>` error slot at construction.  The
//! interpreter loop writes the failure reason there before returning
//! `Value::Null`, and `BrainfuckVM::execute_module` inspects the slot
//! after `JITCore::execute_with_jit` returns to translate it back to a
//! [`crate::errors::BrainfuckError`].
//!
//! # Threading
//!
//! All shared state (`output`, `input`, `steps`, `error`) is behind
//! `Arc<Mutex<…>>`.  `BrainfuckCirJit` is `Send + Sync`, which `Backend`
//! requires.  Poisoned mutexes are recovered via `unwrap_or_else(|e|
//! e.into_inner())`, mirroring the interpreter path.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use jit_core::backend::Backend;
use jit_core::cir::{CIRInstr, CIROperand};
use vm_core::value::Value;

// ---------------------------------------------------------------------------
// Bytecode opcode tags
// ---------------------------------------------------------------------------

/// Opcode tags for the Brainfuck JIT bytecode.
///
/// Tags are grouped by category in the high nibble so a future disassembler
/// can pretty-print families uniformly (`CONST_*` = 0x0x, arithmetic = 0x1x,
/// memory = 0x2x, I/O = 0x3x, control flow = 0x4x, returns = 0x5x).
mod opcode {
    pub const CONST_U8:     u8 = 0x01;
    pub const CONST_U32:    u8 = 0x02;
    pub const ADD_U8:       u8 = 0x10;
    pub const SUB_U8:       u8 = 0x11;
    pub const ADD_U32:      u8 = 0x12;
    pub const SUB_U32:      u8 = 0x13;
    pub const LOAD_MEM:     u8 = 0x20;
    pub const STORE_MEM:    u8 = 0x21;
    pub const PUTCHAR:      u8 = 0x30;
    pub const GETCHAR:      u8 = 0x31;
    pub const JMP:          u8 = 0x40;
    pub const JMP_IF_FALSE: u8 = 0x41;
    pub const JMP_IF_TRUE:  u8 = 0x42;
    pub const RET:          u8 = 0x50;
}

/// Maximum bytes the compiled bytecode can grow to before we refuse —
/// 1 MiB is hugely generous for Brainfuck (typical programs compile to a
/// few KiB) and protects against pathological compile inputs.
const MAX_BYTECODE_BYTES: usize = 1 << 20;

// ---------------------------------------------------------------------------
// BrainfuckCirJit — the Backend implementation
// ---------------------------------------------------------------------------

/// A real-bytecode JIT backend for Brainfuck.  See the module-level
/// documentation for design notes.
pub(crate) struct BrainfuckCirJit {
    /// Output buffer — written by `PUTCHAR` opcodes, shared with the
    /// `BrainfuckVM` wrapper that returns it to the caller.
    output: Arc<Mutex<Vec<u8>>>,

    /// Input buffer — read by `GETCHAR` opcodes.  EOF returns `0`,
    /// mirroring the lazy-infinite-tape convention used by the
    /// interpreter path.
    input: Arc<Mutex<VecDeque<u8>>>,

    /// Step counter — bumped on each backward jump (loop iteration).  The
    /// interpreter path increments on each `label` crossing; backward
    /// jumps are the JIT-bytecode equivalent (labels are erased during
    /// compile, but every iteration of a loop must hit a backward
    /// `JMP` to re-test the guard).
    steps: Arc<Mutex<u64>>,

    /// Error slot — set by the bytecode interpreter on fuel-cap exhaustion
    /// or out-of-bounds writes.  Read by the `BrainfuckVM` wrapper after
    /// `JITCore::execute_with_jit` returns.
    error: Arc<Mutex<Option<String>>>,

    /// Maximum pointer value (exclusive).  Mirrors `BrainfuckVM::tape_size`.
    tape_size: i64,

    /// Optional fuel cap.  When `Some(n)`, the bytecode interpreter
    /// errors out after `n` backward jumps.
    max_steps: Option<u64>,

    /// Output-buffer size cap.  When the JIT path's `PUTCHAR` would push
    /// past this, it silently drops the byte (matching the interpreter
    /// path's behaviour — see `BrainfuckVM::execute_module`).
    output_cap: usize,
}

impl BrainfuckCirJit {
    /// Construct a JIT backend that shares I/O buffers and the error slot
    /// with the surrounding `BrainfuckVM::execute_module` run.
    pub(crate) fn new(
        output: Arc<Mutex<Vec<u8>>>,
        input: Arc<Mutex<VecDeque<u8>>>,
        steps: Arc<Mutex<u64>>,
        error: Arc<Mutex<Option<String>>>,
        tape_size: i64,
        max_steps: Option<u64>,
        output_cap: usize,
    ) -> Self {
        BrainfuckCirJit { output, input, steps, error, tape_size, max_steps, output_cap }
    }

    /// Set the shared error slot if it hasn't been set yet.  Multiple errors
    /// during the same run report the first; later ones are dropped because
    /// the interpreter aborts via `RET` after writing.
    fn set_error(&self, msg: impl Into<String>) {
        let mut slot = self.error.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            *slot = Some(msg.into());
        }
    }
}

impl Backend for BrainfuckCirJit {
    fn name(&self) -> &str {
        "brainfuck-cir-jit"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_to_bytecode(ir)
    }

    fn run(&self, binary: &[u8], _args: &[Value]) -> Value {
        // The dispatch loop owns a fresh tape and register file for the
        // duration of this call.  Both are zero-initialised; this matches
        // BF semantics (cells start at 0, all four registers — ptr, v, c,
        // k — start at 0 before the prologue sets ptr).
        let tape_len: usize = self.tape_size as usize; // tape_size is range-checked to <= i64::MAX
        let mut tape: Vec<u8> = vec![0u8; tape_len];

        // Register file: 256 registers is plenty (BF uses 4).  Using i64
        // keeps the type wide enough for u32 ptr arithmetic without
        // promoting on every op.
        let mut regs: [i64; 256] = [0i64; 256];

        let mut pc: usize = 0;

        // Tight dispatch loop.  Bounds-checked once on `binary.len()` per
        // opcode — Rust's bounds checks on individual byte reads are then
        // elided by LLVM when fed `pc < binary.len()` upstream.
        while pc < binary.len() {
            let op = binary[pc];
            pc += 1;

            match op {
                opcode::CONST_U8 => {
                    // [reg:u8, val:u8]
                    if pc + 2 > binary.len() {
                        self.set_error("malformed bytecode: truncated CONST_U8");
                        return Value::Null;
                    }
                    let reg = binary[pc] as usize;
                    let val = binary[pc + 1] as i64;
                    pc += 2;
                    regs[reg] = val;
                }
                opcode::CONST_U32 => {
                    // [reg:u8, val:u32_le]
                    if pc + 5 > binary.len() {
                        self.set_error("malformed bytecode: truncated CONST_U32");
                        return Value::Null;
                    }
                    let reg = binary[pc] as usize;
                    let val = u32::from_le_bytes([
                        binary[pc + 1], binary[pc + 2], binary[pc + 3], binary[pc + 4],
                    ]) as i64;
                    pc += 5;
                    regs[reg] = val;
                }
                opcode::ADD_U8 | opcode::SUB_U8 => {
                    // [dst:u8, a:u8, b:u8]
                    if pc + 3 > binary.len() {
                        self.set_error("malformed bytecode: truncated ADD/SUB_U8");
                        return Value::Null;
                    }
                    let dst = binary[pc] as usize;
                    let a   = binary[pc + 1] as usize;
                    let b   = binary[pc + 2] as usize;
                    pc += 3;
                    // u8 wraparound by masking after the operation: BF's
                    // cells wrap on overflow (`-` on 0 yields 255).
                    let a_u8 = regs[a] as u8;
                    let b_u8 = regs[b] as u8;
                    let r_u8 = if op == opcode::ADD_U8 {
                        a_u8.wrapping_add(b_u8)
                    } else {
                        a_u8.wrapping_sub(b_u8)
                    };
                    regs[dst] = r_u8 as i64;
                }
                opcode::ADD_U32 | opcode::SUB_U32 => {
                    // [dst:u8, a:u8, b:u8]  — pointer arithmetic.
                    if pc + 3 > binary.len() {
                        self.set_error("malformed bytecode: truncated ADD/SUB_U32");
                        return Value::Null;
                    }
                    let dst = binary[pc] as usize;
                    let a   = binary[pc + 1] as usize;
                    let b   = binary[pc + 2] as usize;
                    pc += 3;
                    // Pointers use wrapping_add/sub to match the
                    // interpreter path (which stores pointers as i64 and
                    // lets the bounds check catch negative / oversized
                    // values rather than panicking on overflow).
                    let a32 = regs[a] as u32;
                    let b32 = regs[b] as u32;
                    let r32 = if op == opcode::ADD_U32 {
                        a32.wrapping_add(b32)
                    } else {
                        a32.wrapping_sub(b32)
                    };
                    regs[dst] = r32 as i64;
                }
                opcode::LOAD_MEM => {
                    // [dst:u8, addr_reg:u8]
                    if pc + 2 > binary.len() {
                        self.set_error("malformed bytecode: truncated LOAD_MEM");
                        return Value::Null;
                    }
                    let dst = binary[pc] as usize;
                    let addr_reg = binary[pc + 1] as usize;
                    pc += 2;
                    let addr = regs[addr_reg];
                    // Match the interpreter's load_mem handler: oob reads
                    // return 0 (lazy-infinite-tape convention), not an
                    // error.  This matters for `<.` on a fresh tape.
                    let val = if addr < 0 || addr >= self.tape_size {
                        0i64
                    } else {
                        tape[addr as usize] as i64
                    };
                    regs[dst] = val;
                }
                opcode::STORE_MEM => {
                    // [addr_reg:u8, val_reg:u8]
                    if pc + 2 > binary.len() {
                        self.set_error("malformed bytecode: truncated STORE_MEM");
                        return Value::Null;
                    }
                    let addr_reg = binary[pc] as usize;
                    let val_reg  = binary[pc + 1] as usize;
                    pc += 2;
                    let addr = regs[addr_reg];
                    if addr < 0 || addr >= self.tape_size {
                        self.set_error(format!(
                            "BrainfuckError: data pointer {addr} out of bounds [0, {})",
                            self.tape_size
                        ));
                        return Value::Null;
                    }
                    // u8 masking matches the interpreter's store_mem handler.
                    tape[addr as usize] = (regs[val_reg] & 0xFF) as u8;
                }
                opcode::PUTCHAR => {
                    // [src:u8]
                    if pc + 1 > binary.len() {
                        self.set_error("malformed bytecode: truncated PUTCHAR");
                        return Value::Null;
                    }
                    let src = binary[pc] as usize;
                    pc += 1;
                    let byte = (regs[src] & 0xFF) as u8;
                    let mut buf = self.output.lock().unwrap_or_else(|e| e.into_inner());
                    if buf.len() < self.output_cap {
                        buf.push(byte);
                    }
                    // Match the interpreter path: silently drop bytes beyond
                    // the cap rather than erroring out.
                }
                opcode::GETCHAR => {
                    // [dst:u8]
                    if pc + 1 > binary.len() {
                        self.set_error("malformed bytecode: truncated GETCHAR");
                        return Value::Null;
                    }
                    let dst = binary[pc] as usize;
                    pc += 1;
                    let mut buf = self.input.lock().unwrap_or_else(|e| e.into_inner());
                    let byte = buf.pop_front().map(|b| b as i64).unwrap_or(0);
                    regs[dst] = byte;
                }
                opcode::JMP => {
                    // [offset:i16_le]  — pc points at the offset bytes
                    if pc + 2 > binary.len() {
                        self.set_error("malformed bytecode: truncated JMP");
                        return Value::Null;
                    }
                    let off = i16::from_le_bytes([binary[pc], binary[pc + 1]]) as isize;
                    pc += 2;
                    // Backward jump = loop iteration: bump the step counter
                    // and check the fuel cap.  Forward jumps don't count
                    // (they're just out-of-loop branches).
                    if off < 0 {
                        if let Err(msg) = self.tick_step() {
                            self.set_error(msg);
                            return Value::Null;
                        }
                    }
                    pc = (pc as isize + off) as usize;
                }
                opcode::JMP_IF_FALSE | opcode::JMP_IF_TRUE => {
                    // [cond:u8, offset:i16_le]
                    if pc + 3 > binary.len() {
                        self.set_error("malformed bytecode: truncated conditional jump");
                        return Value::Null;
                    }
                    let cond = binary[pc] as usize;
                    let off  = i16::from_le_bytes([binary[pc + 1], binary[pc + 2]]) as isize;
                    pc += 3;
                    let truthy = regs[cond] != 0;
                    let take = if op == opcode::JMP_IF_FALSE { !truthy } else { truthy };
                    if take {
                        if off < 0 {
                            if let Err(msg) = self.tick_step() {
                                self.set_error(msg);
                                return Value::Null;
                            }
                        }
                        pc = (pc as isize + off) as usize;
                    }
                }
                opcode::RET => {
                    // No operands; the function returns void.
                    return Value::Null;
                }
                _ => {
                    self.set_error(format!("BrainfuckJIT: unknown opcode 0x{op:02x} at pc {}", pc - 1));
                    return Value::Null;
                }
            }
        }

        // Falling off the end of the bytecode is equivalent to ret_void.
        Value::Null
    }
}

impl BrainfuckCirJit {
    /// Increment the shared step counter and check the fuel cap.  Returns
    /// `Err(msg)` when the cap is exceeded.
    fn tick_step(&self) -> Result<(), String> {
        if let Some(cap) = self.max_steps {
            let mut s = self.steps.lock().unwrap_or_else(|e| e.into_inner());
            *s += 1;
            if *s > cap {
                return Err(format!(
                    "BrainfuckError: max_steps exceeded ({cap} label crossings)"
                ));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Compile pass — CIR → bytecode (helpers)
// ---------------------------------------------------------------------------

/// Width of an inline `CONST_*` materialization.
///
/// Picked per-context based on the consuming instruction's natural type:
/// `add_u8`/`sub_u8`/`store_mem` value/`putchar` arg/`jmp_if_*` cond use
/// `U8`; `add_u32`/`sub_u32`/`load_mem`/`store_mem` addr use `U32`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ConstWidth { U8, U32 }

/// Look up an existing register index for `name` or allocate a new one.
/// Returns `None` if the 256-register namespace is exhausted (BF uses 4,
/// so this is a paranoia limit).
fn lookup_or_alloc_var(
    name: &str,
    reg_map: &mut HashMap<String, u8>,
    next_reg: &mut u16,
) -> Option<u8> {
    if let Some(&idx) = reg_map.get(name) {
        return Some(idx);
    }
    if *next_reg >= 256 { return None; }
    let idx = *next_reg as u8;
    *next_reg += 1;
    reg_map.insert(name.to_string(), idx);
    Some(idx)
}

/// Resolve a CIR operand to a register index.
///
/// For `Var` operands, looks up or allocates an index via
/// [`lookup_or_alloc_var`].
///
/// For literal operands (`Int`, `Bool`), **emits a `CONST_*` instruction
/// inline** that materializes the literal into a fresh anonymous
/// register, then returns that register's index.  The CONST is emitted
/// *before* the consuming instruction, which is what the interpreter
/// expects (the literal must be loaded before the op that reads it).
///
/// Literal operands appear in BF's CIR after `CIROptimizer`'s
/// constant-propagation pass folds `const k 1; add v v k` →
/// `add v v 1`.  Without this helper, we'd refuse every optimized BF
/// program and silently fall back to the interpreter.
///
/// Returns `None` if the operand kind is unsupported (`Float`, `Str`
/// passed as `Var` for non-name uses) or the register file is full.
fn resolve_operand(
    op: &CIROperand,
    reg_map: &mut HashMap<String, u8>,
    next_reg: &mut u16,
    out: &mut Vec<u8>,
    width: ConstWidth,
) -> Option<u8> {
    match op {
        CIROperand::Var(name) => lookup_or_alloc_var(name, reg_map, next_reg),
        CIROperand::Int(n) => {
            if *next_reg >= 256 { return None; }
            let idx = *next_reg as u8;
            *next_reg += 1;
            match width {
                ConstWidth::U8 => {
                    out.push(opcode::CONST_U8);
                    out.push(idx);
                    out.push((*n & 0xFF) as u8);
                }
                ConstWidth::U32 => {
                    out.push(opcode::CONST_U32);
                    out.push(idx);
                    let bytes = (*n as u32).to_le_bytes();
                    out.extend_from_slice(&bytes);
                }
            }
            Some(idx)
        }
        CIROperand::Bool(b) => {
            if *next_reg >= 256 { return None; }
            let idx = *next_reg as u8;
            *next_reg += 1;
            out.push(opcode::CONST_U8);
            out.push(idx);
            out.push(if *b { 1u8 } else { 0u8 });
            Some(idx)
        }
        // Float operands are rejected — Brainfuck has no float ops.
        CIROperand::Float(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Compile pass — CIR → bytecode
// ---------------------------------------------------------------------------

/// Translate a CIR instruction sequence into the bytecode the
/// [`BrainfuckCirJit::run`] interpreter expects.
///
/// Two-pass:
///   1. Walk CIR linearly, assigning register indices on first use and
///      emitting bytes.  Record `label_pos: HashMap<String, usize>` for
///      each `label` opcode and a fixup list `(byte_offset, target_label)`
///      for each branch with a label target.
///   2. Patch each fixup with the correct relative `i16` offset.
///
/// Returns `None` when:
///   - any opcode is outside the supported set (callers fall back to the
///     interpreter via the standard `JITCore` no-cache-entry path);
///   - the function uses more than 256 distinct register names (BF uses
///     4 — this is paranoia for forward compat);
///   - a branch target's offset doesn't fit in `i16` (very large
///     programs).  BF programs that compile to >32 KiB are absurd, but
///     we surface the error cleanly anyway.
///
/// The pass is O(n + m) where n is the CIR length and m is the number of
/// branches; both passes are linear.
fn compile_to_bytecode(ir: &[CIRInstr]) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(ir.len() * 4);
    let mut reg_map: HashMap<String, u8> = HashMap::new();
    let mut next_reg: u16 = 0;  // u16 so we can detect 256-overflow
    let mut label_pos: HashMap<String, usize> = HashMap::new();
    // (byte_offset_of_placeholder, target_label)
    let mut fixups: Vec<(usize, String)> = Vec::new();

    // ---- Pass 1: emit bytecode + collect fixups ---------------------
    //
    // Each instr handler uses the free functions below
    // (`lookup_or_alloc_var`, `resolve_operand`) to map names → register
    // indices.  Literal operands (which appear after the optimizer's
    // constant-propagation pass — e.g. `add_u8 v, 1` after `const k 1`
    // is folded into the use) are materialized inline: we allocate a
    // fresh anonymous register, emit a `CONST_U8` / `CONST_U32` op to
    // load the literal value into it, and use that register index in
    // the consuming instruction.  See `resolve_operand` below.
    for instr in ir {
        // Defensive: refuse to grow beyond MAX_BYTECODE_BYTES.  This
        // protects against pathological compile inputs without
        // crashing the host.
        if out.len() > MAX_BYTECODE_BYTES {
            return None;
        }

        match instr.op.as_str() {
            // ---- label: record position, emit nothing ---------------
            "label" => {
                let name = match instr.srcs.first() {
                    Some(CIROperand::Var(s)) => s.clone(),
                    _ => return None,
                };
                label_pos.insert(name, out.len());
            }

            // ---- const_u8 / const_u32 -------------------------------
            //
            // The optimizer often folds these into their consumers, so
            // they appear less often after optimisation.  We still
            // support them for the rare case where DCE doesn't remove
            // a const whose dest is also re-used via a `Var` reference
            // elsewhere.
            "const_u8" => {
                let dest = instr.dest.as_ref()?;
                let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
                let val = match instr.srcs.first()? {
                    CIROperand::Int(n) => (*n & 0xFF) as u8,
                    CIROperand::Bool(b) => *b as u8,
                    _ => return None,
                };
                out.push(opcode::CONST_U8);
                out.push(dest_idx);
                out.push(val);
            }
            "const_u32" => {
                let dest = instr.dest.as_ref()?;
                let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
                let val = match instr.srcs.first()? {
                    CIROperand::Int(n) => (*n & 0xFFFF_FFFF) as u32,
                    CIROperand::Bool(b) => *b as u32,
                    _ => return None,
                };
                out.push(opcode::CONST_U32);
                out.push(dest_idx);
                out.extend_from_slice(&val.to_le_bytes());
            }

            // ---- typed binary ops -----------------------------------
            "add_u8" | "sub_u8" | "add_u32" | "sub_u32" => {
                if instr.srcs.len() < 2 { return None; }
                let width = if instr.op.ends_with("_u8") { ConstWidth::U8 } else { ConstWidth::U32 };
                // Operand materialization first (may emit CONST_*),
                // then the dest allocation, then the binary op.  Doing
                // dest first would mis-order the bytecode because the
                // literal-materializing CONST_* must precede the consumer.
                let a_idx = resolve_operand(&instr.srcs[0], &mut reg_map, &mut next_reg, &mut out, width)?;
                let b_idx = resolve_operand(&instr.srcs[1], &mut reg_map, &mut next_reg, &mut out, width)?;
                let dest = instr.dest.as_ref()?;
                let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
                let tag = match instr.op.as_str() {
                    "add_u8"  => opcode::ADD_U8,
                    "sub_u8"  => opcode::SUB_U8,
                    "add_u32" => opcode::ADD_U32,
                    "sub_u32" => opcode::SUB_U32,
                    _ => unreachable!(),
                };
                out.push(tag);
                out.push(dest_idx);
                out.push(a_idx);
                out.push(b_idx);
            }

            // ---- memory ops -----------------------------------------
            "load_mem" => {
                // Address operand is a pointer — materialize as U32.
                let addr_idx = resolve_operand(instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut out, ConstWidth::U32)?;
                let dest = instr.dest.as_ref()?;
                let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
                out.push(opcode::LOAD_MEM);
                out.push(dest_idx);
                out.push(addr_idx);
            }
            "store_mem" => {
                if instr.srcs.len() < 2 { return None; }
                let addr_idx = resolve_operand(&instr.srcs[0], &mut reg_map, &mut next_reg, &mut out, ConstWidth::U32)?;
                let val_idx  = resolve_operand(&instr.srcs[1], &mut reg_map, &mut next_reg, &mut out, ConstWidth::U8)?;
                out.push(opcode::STORE_MEM);
                out.push(addr_idx);
                out.push(val_idx);
            }

            // ---- builtins (putchar / getchar) -----------------------
            "call_builtin" => {
                // srcs[0] is the builtin name (Var); the rest are args.
                let name = match instr.srcs.first()? {
                    CIROperand::Var(s) => s.clone(),
                    _ => return None,
                };
                match name.as_str() {
                    "putchar" => {
                        // putchar takes one arg.
                        if instr.srcs.len() < 2 { return None; }
                        let src_idx = resolve_operand(&instr.srcs[1], &mut reg_map, &mut next_reg, &mut out, ConstWidth::U8)?;
                        out.push(opcode::PUTCHAR);
                        out.push(src_idx);
                    }
                    "getchar" => {
                        // getchar returns into dest.
                        let dest = instr.dest.as_ref()?;
                        let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
                        out.push(opcode::GETCHAR);
                        out.push(dest_idx);
                    }
                    // Unknown builtin: refuse to compile, fall back to interpreter.
                    _ => return None,
                }
            }

            // ---- control flow ---------------------------------------
            "jmp" => {
                let target = match instr.srcs.first()? {
                    CIROperand::Var(s) => s.clone(),
                    _ => return None,
                };
                out.push(opcode::JMP);
                let placeholder = out.len();
                out.extend_from_slice(&[0u8, 0u8]);  // i16_le placeholder
                fixups.push((placeholder, target));
            }
            "jmp_if_false" | "jmp_if_true" => {
                if instr.srcs.len() < 2 { return None; }
                let cond_idx = resolve_operand(&instr.srcs[0], &mut reg_map, &mut next_reg, &mut out, ConstWidth::U8)?;
                let target = match &instr.srcs[1] {
                    CIROperand::Var(s) => s.clone(),
                    _ => return None,
                };
                let tag = if instr.op == "jmp_if_false" {
                    opcode::JMP_IF_FALSE
                } else {
                    opcode::JMP_IF_TRUE
                };
                out.push(tag);
                out.push(cond_idx);
                let placeholder = out.len();
                out.extend_from_slice(&[0u8, 0u8]);
                fixups.push((placeholder, target));
            }

            // ---- returns --------------------------------------------
            "ret_void" => {
                out.push(opcode::RET);
            }

            // Any other op (e.g. cmp_*, mul_*, type_assert, call_runtime,
            // ret_<type>) is not part of BF's emitted CIR vocabulary.
            // Refuse to compile rather than producing incorrect bytecode;
            // JITCore will keep the function on the interpreter tier.
            _ => return None,
        }
    }

    // ---- Pass 2: resolve branch fixups ------------------------------
    for (placeholder, target) in fixups {
        let target_pos = *label_pos.get(&target)?;
        // PC after the 2-byte offset = placeholder + 2.  Relative offset
        // = target - (placeholder + 2).  Fits in i16 if |offset| < 32768.
        let rel: isize = target_pos as isize - (placeholder as isize + 2);
        if rel < i16::MIN as isize || rel > i16::MAX as isize {
            return None;
        }
        let bytes = (rel as i16).to_le_bytes();
        out[placeholder]     = bytes[0];
        out[placeholder + 1] = bytes[1];
    }

    Some(out)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn arcs() -> (
        Arc<Mutex<Vec<u8>>>,
        Arc<Mutex<VecDeque<u8>>>,
        Arc<Mutex<u64>>,
        Arc<Mutex<Option<String>>>,
    ) {
        (
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(VecDeque::new())),
            Arc::new(Mutex::new(0)),
            Arc::new(Mutex::new(None)),
        )
    }

    fn jit() -> BrainfuckCirJit {
        let (o, i, s, e) = arcs();
        BrainfuckCirJit::new(o, i, s, e, 30_000, None, 1024 * 1024)
    }

    #[test]
    fn name_matches() {
        assert_eq!(jit().name(), "brainfuck-cir-jit");
    }

    #[test]
    fn compile_empty_program_returns_some_empty_bytecode() {
        let bytes = compile_to_bytecode(&[]);
        assert_eq!(bytes, Some(Vec::new()));
    }

    #[test]
    fn compile_const_u32_emits_op_const_u32_and_le_bytes() {
        // const_u32 ptr 0 [u32]  → [CONST_U32, 0, 0, 0, 0, 0]
        let cir = vec![CIRInstr::new(
            "const_u32", Some("ptr"), vec![CIROperand::Int(0)], "u32",
        )];
        let bytes = compile_to_bytecode(&cir).unwrap();
        assert_eq!(bytes, vec![opcode::CONST_U32, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn compile_unknown_op_returns_none() {
        // No such opcode as `mul_u8` in BF's emitted CIR vocabulary.
        let cir = vec![CIRInstr::new(
            "mul_u8", Some("x"), vec![
                CIROperand::Var("a".into()),
                CIROperand::Var("b".into()),
            ], "u8",
        )];
        assert!(compile_to_bytecode(&cir).is_none());
    }

    #[test]
    fn compile_label_emits_nothing_but_records_position() {
        let cir = vec![
            CIRInstr::new("label", None::<&str>, vec![
                CIROperand::Var("L0".into()),
            ], "void"),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let bytes = compile_to_bytecode(&cir).unwrap();
        // Label emits zero bytes; ret_void emits one byte.
        assert_eq!(bytes, vec![opcode::RET]);
    }

    #[test]
    fn compile_jmp_resolves_offset() {
        // label L0; ret_void; jmp L0
        // bytecode: [RET (pc 0)] [JMP (pc 1)] [off_lo (pc 2)] [off_hi (pc 3)]
        // After JMP at pc 1 + 1 byte op + 2 byte off = pc 4.  offset to
        // target_pos 0 = 0 - 4 = -4.
        let cir = vec![
            CIRInstr::new("label", None::<&str>, vec![
                CIROperand::Var("L0".into()),
            ], "void"),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
            CIRInstr::new("jmp", None::<&str>, vec![
                CIROperand::Var("L0".into()),
            ], "void"),
        ];
        let bytes = compile_to_bytecode(&cir).unwrap();
        // [RET, JMP, lo, hi]
        assert_eq!(bytes[0], opcode::RET);
        assert_eq!(bytes[1], opcode::JMP);
        let off = i16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(off, -4);
    }

    #[test]
    fn compile_jmp_to_unknown_label_returns_none() {
        let cir = vec![
            CIRInstr::new("jmp", None::<&str>, vec![
                CIROperand::Var("nowhere".into()),
            ], "void"),
        ];
        assert!(compile_to_bytecode(&cir).is_none());
    }

    /// End-to-end: compile + run the minimal BF program `+++.` and
    /// verify the output buffer ends up with `[3]`.
    #[test]
    fn run_three_increments_then_putchar() {
        // Hand-built CIR (in the shape BF's compiler emits, post-specialise):
        //   const_u32 ptr 0
        //   load_mem v ptr
        //   const_u8 k 1
        //   add_u8 v v k
        //   store_mem ptr v
        //   load_mem v ptr
        //   const_u8 k 1
        //   add_u8 v v k
        //   store_mem ptr v
        //   load_mem v ptr
        //   const_u8 k 1
        //   add_u8 v v k
        //   store_mem ptr v
        //   load_mem v ptr
        //   call_builtin putchar v
        //   ret_void
        let cir = vec![
            CIRInstr::new("const_u32", Some("ptr"), vec![CIROperand::Int(0)], "u32"),
            // +
            CIRInstr::new("load_mem", Some("v"), vec![CIROperand::Var("ptr".into())], "u8"),
            CIRInstr::new("const_u8", Some("k"), vec![CIROperand::Int(1)], "u8"),
            CIRInstr::new("add_u8", Some("v"), vec![
                CIROperand::Var("v".into()), CIROperand::Var("k".into()),
            ], "u8"),
            CIRInstr::new("store_mem", None::<&str>, vec![
                CIROperand::Var("ptr".into()), CIROperand::Var("v".into()),
            ], "u8"),
            // +
            CIRInstr::new("load_mem", Some("v"), vec![CIROperand::Var("ptr".into())], "u8"),
            CIRInstr::new("const_u8", Some("k"), vec![CIROperand::Int(1)], "u8"),
            CIRInstr::new("add_u8", Some("v"), vec![
                CIROperand::Var("v".into()), CIROperand::Var("k".into()),
            ], "u8"),
            CIRInstr::new("store_mem", None::<&str>, vec![
                CIROperand::Var("ptr".into()), CIROperand::Var("v".into()),
            ], "u8"),
            // +
            CIRInstr::new("load_mem", Some("v"), vec![CIROperand::Var("ptr".into())], "u8"),
            CIRInstr::new("const_u8", Some("k"), vec![CIROperand::Int(1)], "u8"),
            CIRInstr::new("add_u8", Some("v"), vec![
                CIROperand::Var("v".into()), CIROperand::Var("k".into()),
            ], "u8"),
            CIRInstr::new("store_mem", None::<&str>, vec![
                CIROperand::Var("ptr".into()), CIROperand::Var("v".into()),
            ], "u8"),
            // .
            CIRInstr::new("load_mem", Some("v"), vec![CIROperand::Var("ptr".into())], "u8"),
            CIRInstr::new("call_builtin", None::<&str>, vec![
                CIROperand::Var("putchar".into()),
                CIROperand::Var("v".into()),
            ], "void"),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let bytes = compile_to_bytecode(&cir).unwrap();
        assert!(!bytes.is_empty(), "bytecode should be non-empty");

        let (out_a, in_a, st_a, er_a) = arcs();
        let jit = BrainfuckCirJit::new(
            Arc::clone(&out_a), in_a, st_a, Arc::clone(&er_a),
            30_000, None, 1024 * 1024,
        );
        let result = jit.run(&bytes, &[]);
        assert_eq!(result, Value::Null, "ret_void must return Null");
        assert!(er_a.lock().unwrap().is_none(), "no error should be set");
        let out = out_a.lock().unwrap().clone();
        assert_eq!(out, vec![3u8], "expected output [3] from three increments");
    }
}
