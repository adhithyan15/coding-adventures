//! `BasicCirJit` — a real [`jit_core::backend::Backend`] for Dartmouth BASIC.
//!
//! # What this is
//!
//! Mirrors the [`brainfuck_iir_compiler::jit_backend::BrainfuckCirJit`]
//! pattern: a bytecode JIT for the LANG VM's `jit-core` tier system.
//!
//! When BASIC's `main` function is `FullyTyped` (the default for V1 — every
//! IIR instruction carries `"i64"`, `"bool"`, or `"void"`), the
//! threshold-zero compile path fires before the first interpreted call.
//! `JITCore::execute_with_jit` calls [`Self::compile`], which translates
//! the CIR instruction stream into a packed register-machine bytecode.
//! When `vm-core` dispatches `main`, the registered JIT handler invokes
//! [`Self::run`], which interprets that bytecode in a tight loop —
//! bypassing `vm-core`'s generic IIR dispatch entirely.
//!
//! # Is this a real JIT?
//!
//! Yes, in the classic sense — same shape as the JVM Ignition tier,
//! Smalltalk-80, Lua, and V8 Ignition: a high-level IR translated to a
//! compact register-based bytecode and interpreted in a specialised
//! inner loop.  The bytecode here uses single-byte register indices and
//! `i16` little-endian branch offsets, so dispatch is one match arm per
//! opcode instead of `vm-core`'s generic `HashMap<String, OpcodeHandler>`
//! lookup per instruction.
//!
//! This is **not** a native-code JIT (Cranelift, hand-rolled x86_64).
//! Swapping in a native-code backend later is the only change needed —
//! the `JITCore` integration stays the same.
//!
//! # Why BASIC-specific?
//!
//! Two BASIC-specific bits force this to live here rather than in
//! `jit-core`:
//!
//! 1. **`print_i64` / `input_i64` builtins.**  The JIT handler
//!    registered by `jit-core` has signature `Fn(&[Value]) -> Value` —
//!    no access to `VMCore`'s builtin registry.  This backend captures
//!    the same `Arc<Mutex<…>>` I/O buffers that the interpreter path
//!    uses (via [`BasicCirJit::new`]'s parameters), so both execution
//!    paths read from and write to the same logical streams.
//! 2. **i64 register semantics.**  Brainfuck used u8 (cells) and u32
//!    (pointer).  BASIC's V1 vocabulary is integer-only with `i64`
//!    semantics across all arithmetic, comparison, and `ret` ops.
//!
//! # Bytecode format
//!
//! A linear sequence of variable-length instructions.  Opcode tags use
//! the values listed in the [`opcode`] sub-module.  Register indices
//! are 1 byte (256 registers max).  Branch offsets are `i16` little-
//! endian, relative to the byte position immediately after the offset
//! bytes.  i64 const literals are encoded as 8 little-endian bytes.
//!
//! # Error reporting
//!
//! [`jit_core::backend::Backend::run`] has signature
//! `fn(&self, &[u8], &[Value]) -> Value` — there is no way to return a
//! `Result`.  To surface errors (malformed bytecode, division by zero,
//! step-cap exhaustion) back to the caller, the backend captures an
//! `Arc<Mutex<Option<String>>>` error slot at construction.
//!
//! # Threading
//!
//! All shared state (`output`, `input`, `steps`, `error`) is behind
//! `Arc<Mutex<…>>`.  `BasicCirJit` is `Send + Sync`, satisfying the
//! `Backend` trait bound.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use jit_core::backend::Backend;
use jit_core::cir::{CIRInstr, CIROperand};
use vm_core::value::Value;

// ---------------------------------------------------------------------------
// Bytecode opcode tags
// ---------------------------------------------------------------------------

/// Opcode tags for the BASIC JIT bytecode.
///
/// Tags are grouped by category in the high nibble for diagnostic /
/// disassembler convenience: constants 0x0x, arithmetic 0x1x,
/// comparisons 0x2x, control flow 0x3x, builtins 0x4x, returns 0x5x.
#[allow(dead_code)] // each opcode is referenced in the match arms below
pub(crate) mod opcode {
    pub const CONST_I64:    u8 = 0x01;
    pub const MOV:          u8 = 0x02;
    pub const ADD_I64:      u8 = 0x10;
    pub const SUB_I64:      u8 = 0x11;
    pub const MUL_I64:      u8 = 0x12;
    pub const DIV_I64:      u8 = 0x13;
    pub const NEG_I64:      u8 = 0x14;
    pub const CMP_EQ_I64:   u8 = 0x20;
    pub const CMP_NE_I64:   u8 = 0x21;
    pub const CMP_LT_I64:   u8 = 0x22;
    pub const CMP_LE_I64:   u8 = 0x23;
    pub const CMP_GT_I64:   u8 = 0x24;
    pub const CMP_GE_I64:   u8 = 0x25;
    pub const JMP:          u8 = 0x30;
    pub const JMP_IF_FALSE: u8 = 0x31;
    pub const JMP_IF_TRUE:  u8 = 0x32;
    pub const PRINT_I64:    u8 = 0x40;
    pub const INPUT_I64:    u8 = 0x41;
    pub const RET_I64:      u8 = 0x50;
    pub const RET_VOID:     u8 = 0x51;
}

/// Maximum bytes the compiled bytecode can grow to before [`compile_to_bytecode`]
/// refuses — 1 MiB is hugely generous for V1 BASIC programs and protects
/// against pathological compile inputs.
const MAX_BYTECODE_BYTES: usize = 1 << 20;

/// Default fuel cap: 100 million backward jumps.  Hit by infinite-loop
/// programs; gives ordinary programs roughly four orders of magnitude
/// of headroom over typical run-counts.
pub const DEFAULT_STEP_CAP: u64 = 100_000_000;

/// Default output cap: 64 KiB worth of i64 entries (8192 entries).
pub const DEFAULT_OUTPUT_CAP: usize = 8_192;

// ---------------------------------------------------------------------------
// BasicCirJit — the Backend implementation
// ---------------------------------------------------------------------------

/// A real-bytecode JIT backend for Dartmouth BASIC.  See the module-level
/// documentation for design notes.
pub struct BasicCirJit {
    /// Output buffer — written by `PRINT_I64` opcodes, shared with the
    /// test harness or VM wrapper that returns the printed sequence to
    /// the caller.
    output: Arc<Mutex<Vec<i64>>>,

    /// Input buffer — read by `INPUT_I64` opcodes.  EOF returns 0,
    /// mirroring BASIC's traditional "treat blank input as zero" rule.
    input: Arc<Mutex<VecDeque<i64>>>,

    /// Step counter — bumped on each backward jump (loop iteration).
    /// Cross-references the fuel cap to bound runaway loops.
    steps: Arc<Mutex<u64>>,

    /// Error slot — set by the bytecode interpreter on malformed
    /// bytecode, divide-by-zero, or step-cap exhaustion.  Read by the
    /// VM wrapper after `JITCore::execute_with_jit` returns.
    error: Arc<Mutex<Option<String>>>,

    /// Optional fuel cap.  When `Some(n)`, the interpreter errors out
    /// after `n` backward jumps.
    max_steps: Option<u64>,

    /// Output-buffer cap.  When the JIT path's `PRINT_I64` would push
    /// past this, the byte is silently dropped (matching common BASIC
    /// runtime conventions for capped TTYs).
    output_cap: usize,
}

impl BasicCirJit {
    /// Construct a JIT backend that shares I/O buffers and the error
    /// slot with a surrounding harness.
    ///
    /// Defaults: [`DEFAULT_STEP_CAP`] and [`DEFAULT_OUTPUT_CAP`] when
    /// passed `None`s.
    pub fn new(
        output: Arc<Mutex<Vec<i64>>>,
        input: Arc<Mutex<VecDeque<i64>>>,
        steps: Arc<Mutex<u64>>,
        error: Arc<Mutex<Option<String>>>,
        max_steps: Option<u64>,
        output_cap: Option<usize>,
    ) -> Self {
        BasicCirJit {
            output,
            input,
            steps,
            error,
            max_steps: max_steps.or(Some(DEFAULT_STEP_CAP)),
            output_cap: output_cap.unwrap_or(DEFAULT_OUTPUT_CAP),
        }
    }

    /// Set the shared error slot if it hasn't already been set.  Later
    /// errors during the same run are dropped because the interpreter
    /// aborts via `RET_*` after writing.
    fn set_error(&self, msg: impl Into<String>) {
        let mut slot = self.error.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            *slot = Some(msg.into());
        }
    }

    /// Bump the step counter and enforce the fuel cap.  Returns an error
    /// string if the cap is exceeded.
    fn tick_step(&self) -> Result<(), String> {
        let mut s = self.steps.lock().unwrap_or_else(|e| e.into_inner());
        *s = s.saturating_add(1);
        if let Some(cap) = self.max_steps {
            if *s > cap {
                return Err(format!("BasicJIT: step cap exceeded ({cap} backward jumps)"));
            }
        }
        Ok(())
    }
}

impl Backend for BasicCirJit {
    fn name(&self) -> &str {
        "basic-cir-jit"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_to_bytecode(ir)
    }

    fn run(&self, binary: &[u8], _args: &[Value]) -> Value {
        // Register file: 256 i64 registers is plenty for V1 BASIC
        // programs (which use one register per named variable + a few
        // temporaries).
        let mut regs: [i64; 256] = [0i64; 256];
        let mut pc: usize = 0;

        // Tight dispatch loop.
        while pc < binary.len() {
            let op = binary[pc];
            pc += 1;

            match op {
                opcode::CONST_I64 => {
                    // [reg:u8, val:i64_le]
                    if pc + 9 > binary.len() {
                        self.set_error("malformed bytecode: truncated CONST_I64");
                        return Value::Null;
                    }
                    let reg = binary[pc] as usize;
                    let val = i64::from_le_bytes([
                        binary[pc + 1], binary[pc + 2], binary[pc + 3], binary[pc + 4],
                        binary[pc + 5], binary[pc + 6], binary[pc + 7], binary[pc + 8],
                    ]);
                    pc += 9;
                    regs[reg] = val;
                }
                opcode::MOV => {
                    // [dst:u8, src:u8]
                    if pc + 2 > binary.len() {
                        self.set_error("malformed bytecode: truncated MOV");
                        return Value::Null;
                    }
                    let dst = binary[pc] as usize;
                    let src = binary[pc + 1] as usize;
                    pc += 2;
                    regs[dst] = regs[src];
                }
                opcode::ADD_I64 | opcode::SUB_I64
                | opcode::MUL_I64 | opcode::DIV_I64 => {
                    // [dst:u8, a:u8, b:u8]
                    if pc + 3 > binary.len() {
                        self.set_error("malformed bytecode: truncated binary arith");
                        return Value::Null;
                    }
                    let dst = binary[pc] as usize;
                    let a   = binary[pc + 1] as usize;
                    let b   = binary[pc + 2] as usize;
                    pc += 3;
                    let av = regs[a];
                    let bv = regs[b];
                    // Use wrapping arithmetic to match BASIC's
                    // historical "ignore overflow" behaviour on
                    // first-generation hardware.  Division by zero is
                    // a runtime error (BASIC's traditional "?/0 ERROR").
                    let result = match op {
                        opcode::ADD_I64 => av.wrapping_add(bv),
                        opcode::SUB_I64 => av.wrapping_sub(bv),
                        opcode::MUL_I64 => av.wrapping_mul(bv),
                        opcode::DIV_I64 => {
                            if bv == 0 {
                                self.set_error("BasicJIT: division by zero");
                                return Value::Null;
                            }
                            // i64::MIN / -1 overflows; use wrapping_div
                            // to match interpreter behaviour.
                            av.wrapping_div(bv)
                        }
                        _ => unreachable!(),
                    };
                    regs[dst] = result;
                }
                opcode::NEG_I64 => {
                    // [dst:u8, src:u8]
                    if pc + 2 > binary.len() {
                        self.set_error("malformed bytecode: truncated NEG_I64");
                        return Value::Null;
                    }
                    let dst = binary[pc] as usize;
                    let src = binary[pc + 1] as usize;
                    pc += 2;
                    regs[dst] = regs[src].wrapping_neg();
                }
                opcode::CMP_EQ_I64 | opcode::CMP_NE_I64
                | opcode::CMP_LT_I64 | opcode::CMP_LE_I64
                | opcode::CMP_GT_I64 | opcode::CMP_GE_I64 => {
                    // [dst:u8, a:u8, b:u8]
                    if pc + 3 > binary.len() {
                        self.set_error("malformed bytecode: truncated compare");
                        return Value::Null;
                    }
                    let dst = binary[pc] as usize;
                    let a   = binary[pc + 1] as usize;
                    let b   = binary[pc + 2] as usize;
                    pc += 3;
                    let av = regs[a];
                    let bv = regs[b];
                    let cond = match op {
                        opcode::CMP_EQ_I64 => av == bv,
                        opcode::CMP_NE_I64 => av != bv,
                        opcode::CMP_LT_I64 => av <  bv,
                        opcode::CMP_LE_I64 => av <= bv,
                        opcode::CMP_GT_I64 => av >  bv,
                        opcode::CMP_GE_I64 => av >= bv,
                        _ => unreachable!(),
                    };
                    regs[dst] = if cond { 1 } else { 0 };
                }
                opcode::JMP => {
                    // [offset:i16_le]
                    if pc + 2 > binary.len() {
                        self.set_error("malformed bytecode: truncated JMP");
                        return Value::Null;
                    }
                    let off = i16::from_le_bytes([binary[pc], binary[pc + 1]]) as isize;
                    pc += 2;
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
                opcode::PRINT_I64 => {
                    // [src:u8]
                    if pc + 1 > binary.len() {
                        self.set_error("malformed bytecode: truncated PRINT_I64");
                        return Value::Null;
                    }
                    let src = binary[pc] as usize;
                    pc += 1;
                    let mut buf = self.output.lock().unwrap_or_else(|e| e.into_inner());
                    if buf.len() < self.output_cap {
                        buf.push(regs[src]);
                    }
                }
                opcode::INPUT_I64 => {
                    // [dst:u8]
                    if pc + 1 > binary.len() {
                        self.set_error("malformed bytecode: truncated INPUT_I64");
                        return Value::Null;
                    }
                    let dst = binary[pc] as usize;
                    pc += 1;
                    let mut buf = self.input.lock().unwrap_or_else(|e| e.into_inner());
                    let v = buf.pop_front().unwrap_or(0);
                    regs[dst] = v;
                }
                opcode::RET_I64 => {
                    // [src:u8]
                    if pc + 1 > binary.len() {
                        self.set_error("malformed bytecode: truncated RET_I64");
                        return Value::Null;
                    }
                    let src = binary[pc] as usize;
                    return Value::Int(regs[src]);
                }
                opcode::RET_VOID => {
                    return Value::Null;
                }
                _ => {
                    self.set_error(format!(
                        "BasicJIT: unknown opcode 0x{op:02x} at pc {}", pc - 1));
                    return Value::Null;
                }
            }
        }

        // Fell off the end without a RET — treat as void return.
        Value::Null
    }
}

// ---------------------------------------------------------------------------
// Bytecode encoder helpers
// ---------------------------------------------------------------------------

/// Look up or allocate a u8 register index for a CIR variable name.
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

/// Resolve a CIR operand to a register index.
///
/// For `Var` operands, looks up or allocates an index via
/// [`lookup_or_alloc_var`].
///
/// For literal operands (`Int`, `Bool`), emits a `CONST_I64` instruction
/// inline that materialises the literal into a fresh anonymous register,
/// then returns that register's index.  This handles the optimised case
/// where the CIROptimizer's constant-propagation pass has folded
/// `const k 1; add v v k` → `add v v 1`.
///
/// Returns `None` for `Float` operands (BASIC V1 is integer-only) or if
/// the register file is full.
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
        // V1 BASIC is integer-only — refuse floats.
        CIROperand::Float(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Compile pass — CIR → bytecode
// ---------------------------------------------------------------------------

/// Translate a CIR instruction sequence into the bytecode the
/// [`BasicCirJit::run`] interpreter expects.
///
/// Two-pass design:
///   1. Walk CIR linearly, assigning register indices on first use and
///      emitting bytes.  Record `label_pos: HashMap<String, usize>` for
///      each `label` opcode and a fixup list `(byte_offset, target_label)`
///      for each branch with a label target.
///   2. Patch each fixup with the correct relative `i16` offset.
///
/// Returns `None` when:
///   - any opcode is outside the supported set (callers fall back to the
///     interpreter via the standard `JITCore` no-cache-entry path);
///   - the function uses more than 256 distinct register names;
///   - a branch target's offset doesn't fit in `i16`.
fn compile_to_bytecode(ir: &[CIRInstr]) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(ir.len() * 4);
    let mut reg_map: HashMap<String, u8> = HashMap::new();
    let mut next_reg: u16 = 0;
    let mut label_pos: HashMap<String, usize> = HashMap::new();
    // (byte_offset_of_placeholder, target_label)
    let mut fixups: Vec<(usize, String)> = Vec::new();

    // ---- Pass 1: emit bytecode + collect fixups ---------------------
    for instr in ir {
        if out.len() > MAX_BYTECODE_BYTES {
            return None;
        }
        let op = instr.op.as_str();

        // Constants: `const_i64 v <- Int(n)` — type carries width.
        // We accept any const_* mnemonic the specialiser may emit
        // (const_i64 is the common case; const_u32 / const_bool also
        // appear after optimisation passes).
        if op == "const_i64" || op == "const_u8" || op == "const_u16"
            || op == "const_u32" || op == "const_u64"
            || op == "const_i8"  || op == "const_i16" || op == "const_i32"
            || op == "const_bool"
        {
            let dest = instr.dest.as_deref()?;
            let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let n = match instr.srcs.first()? {
                CIROperand::Int(n) => *n,
                CIROperand::Bool(b) => *b as i64,
                _ => return None,
            };
            out.push(opcode::CONST_I64);
            out.push(dest_idx);
            out.extend_from_slice(&n.to_le_bytes());
            continue;
        }

        // mov dest <- src
        if op == "mov" {
            let dest = instr.dest.as_deref()?;
            let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let src_idx = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut out,
            )?;
            out.push(opcode::MOV);
            out.push(dest_idx);
            out.push(src_idx);
            continue;
        }

        // Binary arithmetic: add_i64 / sub_i64 / mul_i64 / div_i64
        let bin_opc = match op {
            "add_i64" => Some(opcode::ADD_I64),
            "sub_i64" => Some(opcode::SUB_I64),
            "mul_i64" => Some(opcode::MUL_I64),
            "div_i64" => Some(opcode::DIV_I64),
            _ => None,
        };
        if let Some(opc) = bin_opc {
            let dest = instr.dest.as_deref()?;
            let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let a = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut out,
            )?;
            let b = resolve_operand(
                instr.srcs.get(1)?, &mut reg_map, &mut next_reg, &mut out,
            )?;
            out.push(opc);
            out.push(dest_idx);
            out.push(a);
            out.push(b);
            continue;
        }

        // Unary: neg_i64
        if op == "neg_i64" {
            let dest = instr.dest.as_deref()?;
            let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let src_idx = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut out,
            )?;
            out.push(opcode::NEG_I64);
            out.push(dest_idx);
            out.push(src_idx);
            continue;
        }

        // Comparisons: cmp_eq_i64 / cmp_ne_i64 / cmp_lt_i64 / cmp_le_i64
        // / cmp_gt_i64 / cmp_ge_i64.  Also accept cmp_*_bool when the
        // specialiser observed a boolean type (BASIC's IIR types relops
        // as "bool"; the specialiser may emit cmp_eq_bool etc.).
        let cmp_opc = match op {
            "cmp_eq_i64" | "cmp_eq_bool" => Some(opcode::CMP_EQ_I64),
            "cmp_ne_i64" | "cmp_ne_bool" => Some(opcode::CMP_NE_I64),
            "cmp_lt_i64" | "cmp_lt_bool" => Some(opcode::CMP_LT_I64),
            "cmp_le_i64" | "cmp_le_bool" => Some(opcode::CMP_LE_I64),
            "cmp_gt_i64" | "cmp_gt_bool" => Some(opcode::CMP_GT_I64),
            "cmp_ge_i64" | "cmp_ge_bool" => Some(opcode::CMP_GE_I64),
            _ => None,
        };
        if let Some(opc) = cmp_opc {
            let dest = instr.dest.as_deref()?;
            let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
            let a = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut out,
            )?;
            let b = resolve_operand(
                instr.srcs.get(1)?, &mut reg_map, &mut next_reg, &mut out,
            )?;
            out.push(opc);
            out.push(dest_idx);
            out.push(a);
            out.push(b);
            continue;
        }

        // label name — record position; no bytes emitted.
        if op == "label" {
            let name = instr.srcs.first()?.as_var()?.to_string();
            label_pos.insert(name, out.len());
            continue;
        }

        // jmp target_label — placeholder offset, fix up later.
        if op == "jmp" {
            let target = instr.srcs.first()?.as_var()?.to_string();
            out.push(opcode::JMP);
            let placeholder = out.len();
            out.extend_from_slice(&[0u8, 0u8]); // i16 placeholder
            fixups.push((placeholder, target));
            continue;
        }

        // jmp_if_true / jmp_if_false cond, target_label
        if op == "jmp_if_true" || op == "jmp_if_false" {
            let cond = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut out,
            )?;
            let target = instr.srcs.get(1)?.as_var()?.to_string();
            out.push(if op == "jmp_if_true" {
                opcode::JMP_IF_TRUE
            } else {
                opcode::JMP_IF_FALSE
            });
            out.push(cond);
            let placeholder = out.len();
            out.extend_from_slice(&[0u8, 0u8]); // i16 placeholder
            fixups.push((placeholder, target));
            continue;
        }

        // call_builtin "print_i64" v / call_builtin "input_i64" -> dest
        if op == "call_builtin" {
            let name = instr.srcs.first()?.as_var()?;
            if name == "print_i64" {
                // srcs[1] is the value to print.
                let v = resolve_operand(
                    instr.srcs.get(1)?, &mut reg_map, &mut next_reg, &mut out,
                )?;
                out.push(opcode::PRINT_I64);
                out.push(v);
                continue;
            }
            if name == "input_i64" {
                let dest = instr.dest.as_deref()?;
                let dest_idx = lookup_or_alloc_var(dest, &mut reg_map, &mut next_reg)?;
                out.push(opcode::INPUT_I64);
                out.push(dest_idx);
                continue;
            }
            // Unknown builtin — refuse to compile; jit-core falls back
            // to the interpreter, which has the full builtin registry.
            return None;
        }

        // ret_i64 v / ret_void
        if op == "ret_i64" {
            let v = resolve_operand(
                instr.srcs.first()?, &mut reg_map, &mut next_reg, &mut out,
            )?;
            out.push(opcode::RET_I64);
            out.push(v);
            continue;
        }
        if op == "ret_void" {
            out.push(opcode::RET_VOID);
            continue;
        }

        // Anything else: refuse.  jit-core falls back to interpreter.
        return None;
    }

    // ---- Pass 2: patch fixups --------------------------------------
    for (placeholder, target) in fixups {
        let target_pos = label_pos.get(&target)?;
        // Offset is from the byte AFTER the i16 (placeholder + 2).
        let from = (placeholder + 2) as isize;
        let to = *target_pos as isize;
        let off = to - from;
        if off < i16::MIN as isize || off > i16::MAX as isize {
            return None;
        }
        let off16 = (off as i16).to_le_bytes();
        out[placeholder] = off16[0];
        out[placeholder + 1] = off16[1];
    }

    Some(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use jit_core::cir::CIRInstr;

    /// Helper: build a tiny CIR sequence that returns 42.
    fn cir_return_42() -> Vec<CIRInstr> {
        vec![
            CIRInstr::new(
                "const_i64",
                Some("v0".to_string()),
                vec![CIROperand::Int(42)],
                "i64",
            ),
            CIRInstr::new(
                "ret_i64",
                None::<&str>,
                vec![CIROperand::Var("v0".into())],
                "i64",
            ),
        ]
    }

    fn make_jit() -> BasicCirJit {
        BasicCirJit::new(
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(VecDeque::new())),
            Arc::new(Mutex::new(0)),
            Arc::new(Mutex::new(None)),
            None,
            None,
        )
    }

    #[test]
    fn compile_simple_const_ret() {
        let jit = make_jit();
        let bytecode = jit.compile(&cir_return_42()).expect("compile must succeed");
        // CONST_I64 + reg + 8 bytes = 10 bytes; RET_I64 + reg = 2 bytes.
        assert_eq!(bytecode.len(), 12);
        assert_eq!(bytecode[0], opcode::CONST_I64);
        assert_eq!(bytecode[10], opcode::RET_I64);
    }

    #[test]
    fn run_simple_const_ret_yields_42() {
        let jit = make_jit();
        let bytecode = jit.compile(&cir_return_42()).expect("compile must succeed");
        let result = jit.run(&bytecode, &[]);
        assert_eq!(result.as_i64(), Some(42));
    }

    #[test]
    fn run_print_i64_pushes_into_shared_output() {
        // CIR for: print_i64(7); ret_void
        let cir = vec![
            CIRInstr::new(
                "const_i64",
                Some("v0".to_string()),
                vec![CIROperand::Int(7)],
                "i64",
            ),
            CIRInstr::new(
                "call_builtin",
                None::<&str>,
                vec![
                    CIROperand::Var("print_i64".into()),
                    CIROperand::Var("v0".into()),
                ],
                "void",
            ),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let output = Arc::new(Mutex::new(Vec::new()));
        let jit = BasicCirJit::new(
            Arc::clone(&output),
            Arc::new(Mutex::new(VecDeque::new())),
            Arc::new(Mutex::new(0)),
            Arc::new(Mutex::new(None)),
            None,
            None,
        );
        let bytecode = jit.compile(&cir).expect("compile");
        jit.run(&bytecode, &[]);
        assert_eq!(*output.lock().unwrap(), vec![7]);
    }

    #[test]
    fn run_add_i64_then_print() {
        // CIR for: a = 30; b = 12; c = add(a, b); print(c); ret_void
        let cir = vec![
            CIRInstr::new(
                "const_i64",
                Some("a".to_string()),
                vec![CIROperand::Int(30)],
                "i64",
            ),
            CIRInstr::new(
                "const_i64",
                Some("b".to_string()),
                vec![CIROperand::Int(12)],
                "i64",
            ),
            CIRInstr::new(
                "add_i64",
                Some("c".to_string()),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
                "i64",
            ),
            CIRInstr::new(
                "call_builtin",
                None::<&str>,
                vec![
                    CIROperand::Var("print_i64".into()),
                    CIROperand::Var("c".into()),
                ],
                "void",
            ),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let output = Arc::new(Mutex::new(Vec::new()));
        let jit = BasicCirJit::new(
            Arc::clone(&output),
            Arc::new(Mutex::new(VecDeque::new())),
            Arc::new(Mutex::new(0)),
            Arc::new(Mutex::new(None)),
            None,
            None,
        );
        let bytecode = jit.compile(&cir).expect("compile");
        jit.run(&bytecode, &[]);
        assert_eq!(*output.lock().unwrap(), vec![42]);
    }

    #[test]
    fn compile_returns_none_on_unsupported_op() {
        // float arithmetic is unsupported in V1 BASIC.
        let cir = vec![CIRInstr::new(
            "add_f64",
            Some("v0".to_string()),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "f64",
        )];
        let jit = make_jit();
        assert!(jit.compile(&cir).is_none());
    }

    #[test]
    fn division_by_zero_sets_error() {
        let cir = vec![
            CIRInstr::new(
                "const_i64",
                Some("a".to_string()),
                vec![CIROperand::Int(1)],
                "i64",
            ),
            CIRInstr::new(
                "const_i64",
                Some("b".to_string()),
                vec![CIROperand::Int(0)],
                "i64",
            ),
            CIRInstr::new(
                "div_i64",
                Some("c".to_string()),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
                "i64",
            ),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let error = Arc::new(Mutex::new(None));
        let jit = BasicCirJit::new(
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(VecDeque::new())),
            Arc::new(Mutex::new(0)),
            Arc::clone(&error),
            None,
            None,
        );
        let bytecode = jit.compile(&cir).expect("compile");
        jit.run(&bytecode, &[]);
        let err = error.lock().unwrap().clone();
        assert!(err.as_ref().is_some_and(|m| m.contains("division by zero")),
            "expected division-by-zero error, got: {err:?}");
    }
}
