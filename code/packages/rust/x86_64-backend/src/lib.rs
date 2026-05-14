//! # `x86_64-backend` — x86-64 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into x86-64 machine code via
//! [`x86_64_encoder`].  Plugs into both `jit-core` and `aot-core` through
//! the shared [`jit_core::backend::Backend`] trait.  Implements
//! [LANG43](../../../../specs/LANG43-x86_64-backend.md).
//!
//! ## ABI selection
//!
//! V1 supports **both** ABIs in use on x86-64 hosts:
//!
//! - [`X86_64Abi::SysV`] — System V AMD64 (Linux, macOS x86-64, FreeBSD).
//!   Arg regs: RDI, RSI, RDX, RCX, R8, R9.
//! - [`X86_64Abi::MsX64`] — Microsoft x64 (Windows).
//!   Arg regs: RCX, RDX, R8, R9.  32-byte shadow space reserved in
//!   prologue.
//!
//! ```
//! use x86_64_backend::{X86_64Backend, X86_64Abi};
//! let backend = X86_64Backend::with_abi(X86_64Abi::SysV);   // Linux/macOS
//! let backend = X86_64Backend::with_abi(X86_64Abi::MsX64);  // Windows
//! ```
//!
//! ## V1 scope
//!
//! | Family | CIR mnemonics |
//! |---|---|
//! | Constants | `const_<ty>` (int and bool literals) |
//! | Moves | `mov_<ty>` |
//! | Integer arithmetic | `add_<ty>`, `sub_<ty>`, `mul_<ty>` |
//! | Comparisons | `cmp_eq_<ty>`, `cmp_ne_<ty>`, `cmp_lt_<ty>`, `cmp_le_<ty>`, `cmp_gt_<ty>`, `cmp_ge_<ty>` (signed and unsigned) |
//! | Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
//! | Returns | `ret_<ty>`, `ret_void` |
//! | Type guards | `type_assert` (`UD2` trap) |
//!
//! Everything else (`call`, `global_load`/`store`, `io_out`, division,
//! logical, shifts, floats, closures) is **not yet** and is added by
//! later phases.  Unsupported opcodes cause `compile_function` to
//! return `None`, which `aot-core` reports as a backend miss.
//!
//! ## Register allocation
//!
//! Stack spill.  Every virtual lives at `[rbp - 8 - slot_idx*8]`.
//! Three GPRs are reserved as scratch for every instruction emission:
//!
//! - `RAX` — primary scratch + return register.
//! - `RCX` — shift-count register (used in phase 4 for `SHL`/`SHR`/`SAR`).
//! - `RDX` — high half of `RDX:RAX` for `IDIV`/`DIV` (used in phase 4).
//!
//! Reserving all three up front means later phases never have to
//! shuffle scratch.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::HashMap;

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use vm_core::value::Value;
use x86_64_encoder::{
    Assembler, Cond, EncodeError, ExternalReloc, ExternalRelocKind, LabelId, Reg,
};

pub use x86_64_encoder::ExternalReloc as Reloc;

// ===========================================================================
// ABI selection
// ===========================================================================

/// x86-64 calling-convention selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86_64Abi {
    /// System V AMD64 — Linux, FreeBSD, macOS x86-64.
    SysV,
    /// Microsoft x64 — Windows.
    MsX64,
}

impl X86_64Abi {
    /// Integer argument registers in order.
    fn arg_regs(self) -> &'static [Reg] {
        match self {
            X86_64Abi::SysV  => &[Reg::Rdi, Reg::Rsi, Reg::Rdx, Reg::Rcx, Reg::R8, Reg::R9],
            X86_64Abi::MsX64 => &[Reg::Rcx, Reg::Rdx, Reg::R8, Reg::R9],
        }
    }

    /// Maximum number of GPR-passed args supported by this ABI.
    fn max_args(self) -> usize { self.arg_regs().len() }

    /// 32-byte shadow space the caller must reserve.  Microsoft x64
    /// only.  V1 reserves it unconditionally in every non-trivial
    /// function's prologue so call sites don't have to do it
    /// per-call.
    fn shadow_space(self) -> u32 {
        match self {
            X86_64Abi::SysV  => 0,
            X86_64Abi::MsX64 => 32,
        }
    }
}

// ===========================================================================
// Backend implementation
// ===========================================================================

/// x86-64 native-code backend.
#[derive(Debug, Default, Clone, Copy)]
pub struct X86_64Backend {
    abi: X86_64AbiOption,
}

// Default::default() must yield Default for X86_64Backend; X86_64Abi
// doesn't derive Default to make the choice explicit at the call site.
#[derive(Debug, Default, Clone, Copy)]
struct X86_64AbiOption(Option<X86_64Abi>);

impl X86_64AbiOption {
    fn unwrap_or_sysv(self) -> X86_64Abi { self.0.unwrap_or(X86_64Abi::SysV) }
}

impl X86_64Backend {
    /// Construct a backend defaulting to System V (Linux/macOS).
    pub fn new() -> Self { X86_64Backend { abi: X86_64AbiOption(Some(X86_64Abi::SysV)) } }

    /// Construct a backend with an explicit ABI choice.
    pub fn with_abi(abi: X86_64Abi) -> Self {
        X86_64Backend { abi: X86_64AbiOption(Some(abi)) }
    }

    /// The ABI this backend will use.
    pub fn abi(&self) -> X86_64Abi { self.abi.unwrap_or_sysv() }
}

impl Backend for X86_64Backend {
    fn name(&self) -> &str {
        match self.abi() {
            X86_64Abi::SysV  => "x86_64-sysv",
            X86_64Abi::MsX64 => "x86_64-msx64",
        }
    }

    /// Without function context the prologue can't be laid out
    /// (param-arrival registers are unknown).  Mirrors aarch64-backend.
    fn compile(&self, _ir: &[CIRInstr]) -> Option<Vec<u8>> { None }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        // Native-code execution requires a JIT loader; not in this crate.
        Value::Null
    }

    fn compile_function(&self, ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_function(ctx, ir, self.abi()).ok()
    }
}

// ===========================================================================
// Public entry points (used by tests and the AOT linker)
// ===========================================================================

/// Compile a single function with the given ABI.  Returns the
/// function's machine-code bytes on success, or a diagnostic string on
/// failure (for surfaceable errors from tests).
///
/// External relocations (cross-function calls, runtime helpers) are
/// silently discarded.  Use [`compile_function_with_relocs`] when the
/// AOT linker needs the relocation list.
pub fn compile_function(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
    abi: X86_64Abi,
) -> Result<Vec<u8>, String> {
    compile_inner(ctx, ir, abi)
        .map(|(bytes, _relocs)| bytes)
        .map_err(|e| format!("x86_64-backend: {e:?}"))
}

/// Like [`compile_function`] but also returns the list of external
/// relocations the AOT linker must patch after concatenating all
/// function bodies into a single text section.
///
/// Each [`Reloc`] points at a 32-bit slot in the function's bytes
/// (`patch_offset`) plus the symbol name and reloc kind the packager
/// (LANG45) translates to an OS-specific relocation record.
pub fn compile_function_with_relocs(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
    abi: X86_64Abi,
) -> Result<(Vec<u8>, Vec<Reloc>), String> {
    compile_inner(ctx, ir, abi)
        .map_err(|e| format!("x86_64-backend: {e:?}"))
}

// ===========================================================================
// Stack-spill register allocator
// ===========================================================================

/// Assigns each CIR virtual register a stack slot.
///
/// Slots are addressed as `[rbp - 8 - slot_idx*8]`, growing downward
/// from the saved RBP.  This means slot 0 lives at `[rbp - 8]`, slot 1
/// at `[rbp - 16]`, etc.
#[derive(Debug, Default)]
struct RegAlloc {
    slots: HashMap<String, u32>,
    /// Next slot index to hand out (0-based).
    next_slot: u32,
}

impl RegAlloc {
    fn slot_of(&mut self, name: &str) -> u32 {
        if let Some(&s) = self.slots.get(name) { return s; }
        let s = self.next_slot;
        self.next_slot = self.next_slot.checked_add(1).expect("slot overflow");
        self.slots.insert(name.to_string(), s);
        s
    }

    /// Number of slots in use.
    fn slot_count(&self) -> u32 { self.next_slot }

    /// Byte offset from RBP for the given slot index (always negative).
    fn rbp_offset(slot: u32) -> i32 { -8i32 - 8 * (slot as i32) }
}

// ===========================================================================
// Errors (internal)
// ===========================================================================

#[derive(Debug)]
#[allow(dead_code)]
enum BackendError {
    /// CIR contains an opcode this backend doesn't yet support.
    UnsupportedOp(String),
    /// CIR uses more parameters than the ABI has GPR slots.
    TooManyParams { abi: &'static str, got: usize, max: usize },
    /// An instruction is missing a required `dest` or `srcs` field.
    MalformedInstr(String),
    /// Encoder rejected an immediate or branch.
    Encoder(EncodeError),
}

impl From<EncodeError> for BackendError {
    fn from(e: EncodeError) -> Self { BackendError::Encoder(e) }
}

// ===========================================================================
// Top-level compile
// ===========================================================================

fn compile_inner(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
    abi: X86_64Abi,
) -> Result<(Vec<u8>, Vec<ExternalReloc>), BackendError> {
    if ctx.params.len() > abi.max_args() {
        return Err(BackendError::TooManyParams {
            abi: match abi { X86_64Abi::SysV => "SysV", X86_64Abi::MsX64 => "MsX64" },
            got: ctx.params.len(),
            max: abi.max_args(),
        });
    }

    // ---- Pre-pass: assign slots --------------------------------------------
    //
    // Params first, in arg-register order, so the prologue's stores
    // line up with the slot offsets.  Then walk CIR to allocate
    // dest slots and source-var slots deterministically.
    let mut alloc = RegAlloc::default();
    for (name, _ty) in ctx.params {
        alloc.slot_of(name);
    }
    for instr in ir {
        if let Some(d) = &instr.dest {
            alloc.slot_of(d);
        }
        for src in &instr.srcs {
            if let CIROperand::Var(s) = src {
                alloc.slot_of(s);
            }
        }
    }

    // ---- Frame size --------------------------------------------------------
    //
    // Reserve 8 bytes per virtual slot, plus the ABI shadow space, and
    // round up to a 16-byte multiple so that after `push rbp; sub rsp,
    // frame` the stack stays 16-byte aligned at every CALL site.
    let raw_frame = alloc.slot_count() * 8 + abi.shadow_space();
    // After `push rbp` (8 bytes pushed), RSP is at a 0-mod-16 boundary.
    // We need rsp + frame ≡ 0 (mod 16), so frame ≡ 0 (mod 16).
    let frame: u32 = (raw_frame + 15) & !15;

    // ---- Pre-pass: pre-create labels --------------------------------------
    let mut asm = Assembler::new();
    let mut labels: HashMap<String, LabelId> = HashMap::new();
    for instr in ir {
        if instr.op == "label" {
            if let Some(name) = label_name(instr) {
                labels.entry(name.to_string())
                    .or_insert_with(|| asm.create_label());
            }
        }
    }
    for instr in ir {
        if matches!(instr.op.as_str(), "jmp" | "jmp_if_true" | "jmp_if_false") {
            if let Some(target) = label_name(instr) {
                labels.entry(target.to_string())
                    .or_insert_with(|| asm.create_label());
            }
        }
        // Self-recursive call: pre-create a label for the callee name so
        // it can be bound at the function entry below.
        if instr.op == "call" {
            if let Some(CIROperand::Var(name)) = instr.srcs.first() {
                labels.entry(name.clone())
                    .or_insert_with(|| asm.create_label());
            }
        }
    }

    // Bind the current function's own name to the very start of the
    // prologue.  `call <fn_name>` instructions in the body emit
    // `call_label(entry_label)` which re-enters the function here,
    // re-executing the full prologue (push rbp + spill args) for each
    // new call frame.
    if let Some(&entry_label) = labels.get(ctx.name) {
        asm.bind(entry_label).map_err(BackendError::from)?;
    }

    // ---- Prologue ----------------------------------------------------------
    asm.push(Reg::Rbp);
    asm.mov_r64_r64(Reg::Rbp, Reg::Rsp);
    if frame != 0 {
        asm.sub_imm32(Reg::Rsp, frame as i32);
    }

    // Spill incoming arg registers to their slots.
    for (i, (name, _ty)) in ctx.params.iter().enumerate() {
        let slot = alloc.slot_of(name);
        let off = RegAlloc::rbp_offset(slot);
        let arg_reg = abi.arg_regs()[i];
        asm.mov_mem_r64(Reg::Rbp, off, arg_reg);
    }

    // ---- Body --------------------------------------------------------------
    for instr in ir {
        emit_instr(&mut asm, instr, &mut alloc, &labels, frame, ctx.name, abi)?;
    }

    // ---- Defensive epilogue (in case CIR falls off the end) ----------------
    emit_epilogue(&mut asm, frame);

    let external_relocs = std::mem::take(&mut asm.external_relocs);
    let bytes = asm.finish().map_err(BackendError::from)?;
    Ok((bytes, external_relocs))
}

fn emit_epilogue(asm: &mut Assembler, _frame: u32) {
    // `mov rsp, rbp` deallocates the frame regardless of size — one byte
    // shorter and simpler than `add rsp, frame`.  Works because the
    // prologue established `mov rbp, rsp` *after* `push rbp`, so
    // restoring RSP to RBP undoes the `sub rsp, frame` exactly.
    asm.mov_r64_r64(Reg::Rsp, Reg::Rbp);
    asm.pop(Reg::Rbp);
    asm.ret();
}

// ===========================================================================
// Per-instruction lowering
// ===========================================================================

fn emit_instr(
    asm: &mut Assembler,
    instr: &CIRInstr,
    alloc: &mut RegAlloc,
    labels: &HashMap<String, LabelId>,
    frame: u32,
    fn_name: &str,
    abi: X86_64Abi,
) -> Result<(), BackendError> {
    let op = instr.op.as_str();

    // --- label ---
    if op == "label" {
        let name = label_name(instr)
            .ok_or_else(|| BackendError::MalformedInstr("label needs srcs[0]=Var(name)".into()))?;
        let id = *labels.get(name)
            .ok_or_else(|| BackendError::MalformedInstr(format!("undefined label {name}")))?;
        asm.bind(id).map_err(BackendError::from)?;
        return Ok(());
    }

    // --- jmp ---
    if op == "jmp" {
        let name = label_name(instr)
            .ok_or_else(|| BackendError::MalformedInstr("jmp needs target".into()))?;
        let id = *labels.get(name)
            .ok_or_else(|| BackendError::MalformedInstr(format!("unknown label {name}")))?;
        asm.jmp(id);
        return Ok(());
    }

    // --- jmp_if_true / jmp_if_false ---
    if op == "jmp_if_true" || op == "jmp_if_false" {
        let cond_var = instr.srcs.first().and_then(CIROperand::as_var)
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]=cond")))?;
        let target = instr.srcs.get(1).and_then(CIROperand::as_var)
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[1]=label")))?;
        let target_id = *labels.get(target)
            .ok_or_else(|| BackendError::MalformedInstr(format!("unknown label {target}")))?;
        let slot = alloc.slot_of(cond_var);
        asm.mov_r64_mem(Reg::Rax, Reg::Rbp, RegAlloc::rbp_offset(slot));
        asm.test_(Reg::Rax, Reg::Rax);
        let cc = if op == "jmp_if_true" { Cond::Ne } else { Cond::E };
        asm.jcc(cc, target_id);
        return Ok(());
    }

    // --- type_assert: trap on guard failure ---
    if op == "type_assert" {
        asm.ud2();
        return Ok(());
    }

    // --- ret_void ---
    if op == "ret_void" {
        emit_epilogue(asm, frame);
        return Ok(());
    }
    // --- ret_<ty>: load value into RAX, then epilogue ---
    if op.starts_with("ret_") {
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::Rax, src);
        emit_epilogue(asm, frame);
        return Ok(());
    }

    // --- mov_<ty>: typed copy ---
    if op.starts_with("mov_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::Rax, src);
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // --- const_<ty>: literal -> slot ---
    if op.starts_with("const_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        let imm: u64 = match src {
            CIROperand::Int(n)   => *n as u64,
            CIROperand::Bool(b)  => if *b { 1 } else { 0 },
            CIROperand::Float(_) => return Err(BackendError::UnsupportedOp("const_f64".into())),
            CIROperand::Var(_)   => return Err(BackendError::MalformedInstr(format!("{op} needs literal source"))),
        };
        // Prefer the shorter `mov r/m64, imm32` (sign-extended) when it
        // fits — same as aarch64-backend's compact-immediate path.
        if (imm as i64) >= i32::MIN as i64 && (imm as i64) <= i32::MAX as i64 {
            asm.mov_r64_imm32(Reg::Rax, imm as i32);
        } else {
            asm.mov_r64_imm64(Reg::Rax, imm);
        }
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // --- add_<ty> / sub_<ty> / mul_<ty> ---
    if op.starts_with("add_") { return emit_binop(asm, alloc, instr, BinOp::Add); }
    if op.starts_with("sub_") { return emit_binop(asm, alloc, instr, BinOp::Sub); }
    if op.starts_with("mul_") { return emit_binop(asm, alloc, instr, BinOp::Mul); }

    // --- cmp_<rel>_<ty> ---
    if let Some(rest) = op.strip_prefix("cmp_") {
        let (rel, signed) = parse_cmp_suffix(rest)
            .ok_or_else(|| BackendError::MalformedInstr(format!("bad cmp mnemonic: {op}")))?;
        return emit_cmp(asm, alloc, instr, rel, signed);
    }

    // --- call callee_name, arg0, ..., argN ---
    //
    // CIR encoding: srcs[0] = Var(callee_name), srcs[1..] = arguments.
    // The dest slot (if any) receives the return value from RAX.
    //
    // Self-recursive calls (callee == fn_name) emit `call_label(entry_label)`
    // resolved within this function's bytes; cross-function calls emit
    // `call_rel32(callee_name, PltRel32)` and rely on the AOT linker to
    // patch the displacement once all function bodies are concatenated.
    if op == "call" {
        let callee_name = match instr.srcs.first() {
            Some(CIROperand::Var(name)) => name.as_str(),
            _ => return Err(BackendError::MalformedInstr(
                "call: srcs[0] must be Var(function_name)".into(),
            )),
        };
        let arg_srcs = &instr.srcs[1..];
        if arg_srcs.len() > abi.max_args() {
            return Err(BackendError::UnsupportedOp(format!(
                "call: too many arguments ({}) for {:?} ABI — max {}",
                arg_srcs.len(), abi, abi.max_args()
            )));
        }

        // Load each argument from its stack slot into the ABI's arg register.
        // With stack-spill allocation, all values are already on the stack,
        // so loading left-to-right is correct — no aliasing between an arg's
        // source slot and another arg's destination register.
        let arg_regs = abi.arg_regs();
        for (i, src) in arg_srcs.iter().enumerate() {
            load_operand(asm, alloc, arg_regs[i], src);
        }

        // Emit the call itself.
        if callee_name == fn_name {
            let target_id = *labels.get(callee_name).ok_or_else(|| {
                BackendError::MalformedInstr(format!("call: no label for '{callee_name}'"))
            })?;
            asm.call_label(target_id);
        } else {
            asm.call_rel32(callee_name, ExternalRelocKind::PltRel32);
        }

        // Save the return value from RAX into the destination slot.
        if let Some(dest) = &instr.dest {
            let slot = alloc.slot_of(dest);
            asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        }
        return Ok(());
    }

    // --- LANG38-parity additions ---

    // div_<ty> / mod_<ty> — signed types use IDIV, unsigned use DIV.
    if let Some(ty) = op.strip_prefix("div_") { return emit_divmod(asm, alloc, instr, ty, false); }
    if let Some(ty) = op.strip_prefix("mod_") { return emit_divmod(asm, alloc, instr, ty, true);  }

    // and_<ty> / or_<ty> / xor_<ty>
    if op.starts_with("and_") { return emit_bitwise(asm, alloc, instr, Bitwise::And); }
    if op.starts_with("or_")  { return emit_bitwise(asm, alloc, instr, Bitwise::Or);  }
    if op.starts_with("xor_") { return emit_bitwise(asm, alloc, instr, Bitwise::Xor); }

    // shl_<ty>: logical shift left (same for signed/unsigned).
    if op.starts_with("shl_") { return emit_shift(asm, alloc, instr, ShiftKind::Shl); }
    // shr_<ty>: arithmetic for signed (SAR), logical for unsigned (SHR).
    if let Some(ty) = op.strip_prefix("shr_") {
        let kind = if ty.starts_with('i') { ShiftKind::Sar } else { ShiftKind::Shr };
        return emit_shift(asm, alloc, instr, kind);
    }

    // neg_<ty> dest = -src
    if op.starts_with("neg_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::Rax, src);
        asm.neg_(Reg::Rax);
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // not_<ty> dest = ~src
    if op.starts_with("not_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::Rax, src);
        asm.not_(Reg::Rax);
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    Err(BackendError::UnsupportedOp(op.to_string()))
}

// ===========================================================================
// Division and modulo
// ===========================================================================

/// `div_<ty>` and `mod_<ty>` both go through this helper.
///
/// x86-64 division is awkward: `IDIV r/m64` consumes `RDX:RAX` as the
/// 128-bit dividend and produces quotient in `RAX`, remainder in `RDX`.
/// For signed types we must sign-extend `RAX` into `RDX:RAX` with `CQO`;
/// for unsigned we just zero `RDX`.
///
/// Sequence (signed div):
///
/// ```text
/// mov  rax, [lhs]
/// cqo                     ; rdx:rax = sign-extend(rax)
/// mov  rcx, [rhs]
/// idiv rcx
/// mov  [dst], rax         ; (for mod_, [dst], rdx)
/// ```
fn emit_divmod(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    ty: &str,
    is_mod: bool,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    let lhs = instr.srcs.first()
        .ok_or_else(|| BackendError::MalformedInstr("div/mod needs srcs[0]".into()))?;
    let rhs = instr.srcs.get(1)
        .ok_or_else(|| BackendError::MalformedInstr("div/mod needs srcs[1]".into()))?;
    let signed = ty.starts_with('i');

    load_operand(asm, alloc, Reg::Rax, lhs);
    // Sign-/zero-extend RAX into RDX.
    if signed {
        asm.cqo();
    } else {
        asm.xor_(Reg::Rdx, Reg::Rdx);
    }
    // Load divisor into RCX (IDIV/DIV r/m64 form — divisor in a register).
    load_operand(asm, alloc, Reg::Rcx, rhs);
    if signed { asm.idiv(Reg::Rcx); } else { asm.div(Reg::Rcx); }
    // Result lives in RAX (quotient) or RDX (remainder).
    let result_reg = if is_mod { Reg::Rdx } else { Reg::Rax };
    let slot = alloc.slot_of(dest);
    asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), result_reg);
    Ok(())
}

// ===========================================================================
// Bitwise ops (AND / OR / XOR)
// ===========================================================================

#[derive(Debug, Clone, Copy)]
enum Bitwise { And, Or, Xor }

fn emit_bitwise(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    op: Bitwise,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    let lhs = instr.srcs.first()
        .ok_or_else(|| BackendError::MalformedInstr("bitwise needs srcs[0]".into()))?;
    let rhs = instr.srcs.get(1)
        .ok_or_else(|| BackendError::MalformedInstr("bitwise needs srcs[1]".into()))?;
    load_operand(asm, alloc, Reg::Rax, lhs);
    load_operand(asm, alloc, Reg::Rcx, rhs);
    match op {
        Bitwise::And => asm.and_(Reg::Rax, Reg::Rcx),
        Bitwise::Or  => asm.or_(Reg::Rax,  Reg::Rcx),
        Bitwise::Xor => asm.xor_(Reg::Rax, Reg::Rcx),
    }
    let slot = alloc.slot_of(dest);
    asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
    Ok(())
}

// ===========================================================================
// Variable shifts (SHL / SHR / SAR)
// ===========================================================================

/// Variable shift kind.  Signed-shift-right (SAR) preserves the sign bit;
/// logical-shift-right (SHR) zero-fills.
#[derive(Debug, Clone, Copy)]
enum ShiftKind { Shl, Shr, Sar }

fn emit_shift(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    kind: ShiftKind,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    let lhs = instr.srcs.first()
        .ok_or_else(|| BackendError::MalformedInstr("shift needs srcs[0]".into()))?;
    let rhs = instr.srcs.get(1)
        .ok_or_else(|| BackendError::MalformedInstr("shift needs srcs[1]".into()))?;
    // x86-64 variable shift uses CL as the count.  Load the value to be
    // shifted into RAX and the count into RCX (which has CL in its low
    // byte), then issue `shl/shr/sar rax, cl`.
    load_operand(asm, alloc, Reg::Rax, lhs);
    load_operand(asm, alloc, Reg::Rcx, rhs);
    match kind {
        ShiftKind::Shl => asm.shl_cl(Reg::Rax),
        ShiftKind::Shr => asm.shr_cl(Reg::Rax),
        ShiftKind::Sar => asm.sar_cl(Reg::Rax),
    }
    let slot = alloc.slot_of(dest);
    asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
    Ok(())
}

// ===========================================================================
// Binary integer ops
// ===========================================================================

#[derive(Debug, Clone, Copy)]
enum BinOp { Add, Sub, Mul }

fn emit_binop(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    op: BinOp,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    let lhs = instr.srcs.first()
        .ok_or_else(|| BackendError::MalformedInstr(format!("{:?} needs srcs[0]", op)))?;
    let rhs = instr.srcs.get(1)
        .ok_or_else(|| BackendError::MalformedInstr(format!("{:?} needs srcs[1]", op)))?;
    load_operand(asm, alloc, Reg::Rax, lhs);
    load_operand(asm, alloc, Reg::Rcx, rhs);
    match op {
        BinOp::Add => asm.add(Reg::Rax, Reg::Rcx),
        BinOp::Sub => asm.sub(Reg::Rax, Reg::Rcx),
        BinOp::Mul => asm.imul(Reg::Rax, Reg::Rcx),
    }
    let slot = alloc.slot_of(dest);
    asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
    Ok(())
}

// ===========================================================================
// Comparisons
// ===========================================================================

/// The six relational predicates Twig/CIR uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmpRel { Eq, Ne, Lt, Le, Gt, Ge }

/// Parse the suffix after `cmp_` into a predicate + signedness flag.
///
/// Accepts mnemonics like `eq_u8`, `lt_i64`, `ge_u32`, `ne_bool`.
fn parse_cmp_suffix(rest: &str) -> Option<(CmpRel, bool)> {
    // Split at the first '_': "lt_i64" -> ("lt", "i64").
    let (rel_str, ty) = rest.split_once('_')?;
    let rel = match rel_str {
        "eq" => CmpRel::Eq,
        "ne" => CmpRel::Ne,
        "lt" => CmpRel::Lt,
        "le" => CmpRel::Le,
        "gt" => CmpRel::Gt,
        "ge" => CmpRel::Ge,
        _ => return None,
    };
    // Signedness flag: `i*` types signed; everything else unsigned.
    // `bool` is treated as unsigned (it's 0 or 1).
    let signed = ty.starts_with('i');
    Some((rel, signed))
}

fn emit_cmp(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    rel: CmpRel,
    signed: bool,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    let lhs = instr.srcs.first()
        .ok_or_else(|| BackendError::MalformedInstr("cmp needs srcs[0]".into()))?;
    let rhs = instr.srcs.get(1)
        .ok_or_else(|| BackendError::MalformedInstr("cmp needs srcs[1]".into()))?;
    load_operand(asm, alloc, Reg::Rax, lhs);
    load_operand(asm, alloc, Reg::Rcx, rhs);
    asm.cmp(Reg::Rax, Reg::Rcx);
    // setcc al; movzx rax, al; store
    let cc = match (rel, signed) {
        (CmpRel::Eq, _)     => Cond::E,
        (CmpRel::Ne, _)     => Cond::Ne,
        (CmpRel::Lt, true)  => Cond::L,
        (CmpRel::Lt, false) => Cond::B,
        (CmpRel::Le, true)  => Cond::Le,
        (CmpRel::Le, false) => Cond::Be,
        (CmpRel::Gt, true)  => Cond::G,
        (CmpRel::Gt, false) => Cond::A,
        (CmpRel::Ge, true)  => Cond::Ge,
        (CmpRel::Ge, false) => Cond::Ae,
    };
    asm.setcc(cc, Reg::Rax);
    asm.movzx_r64_r8(Reg::Rax, Reg::Rax);
    let slot = alloc.slot_of(dest);
    asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
    Ok(())
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Load a CIR operand into a register: from a stack slot if `Var`,
/// or materialise the literal if `Int` / `Bool`.
fn load_operand(asm: &mut Assembler, alloc: &mut RegAlloc, dst: Reg, op: &CIROperand) {
    match op {
        CIROperand::Var(name) => {
            let slot = alloc.slot_of(name);
            asm.mov_r64_mem(dst, Reg::Rbp, RegAlloc::rbp_offset(slot));
        }
        CIROperand::Int(n) => {
            let v = *n;
            if (i32::MIN as i64..=i32::MAX as i64).contains(&v) {
                asm.mov_r64_imm32(dst, v as i32);
            } else {
                asm.mov_r64_imm64(dst, v as u64);
            }
        }
        CIROperand::Bool(b) => {
            asm.mov_r64_imm32(dst, if *b { 1 } else { 0 });
        }
        CIROperand::Float(_) => {
            // V1 doesn't support floats; lower as zero to avoid panic.
            // The backend will already have refused the surrounding
            // const_f64 / float op, so this branch is defensive only.
            asm.mov_r64_imm32(dst, 0);
        }
    }
}

fn require_dest(instr: &CIRInstr) -> Result<&str, BackendError> {
    instr.dest.as_deref()
        .ok_or_else(|| BackendError::MalformedInstr(
            format!("{} needs a dest", instr.op)))
}

fn label_name(instr: &CIRInstr) -> Option<&str> {
    match instr.srcs.first()? {
        CIROperand::Var(s) => Some(s.as_str()),
        _ => None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use jit_core::cir::CIROperand as Op;

    fn instr(op: &str, dest: Option<&str>, srcs: Vec<Op>) -> CIRInstr {
        CIRInstr {
            op: op.to_string(),
            dest: dest.map(str::to_string),
            srcs,
            ty: "u64".to_string(),
            deopt_to: None,
        }
    }

    fn fn_ctx<'a>(name: &'a str, params: &'a [(String, String)], return_type: &'a str) -> FunctionContext<'a> {
        FunctionContext { name, params, return_type }
    }

    // ---- Prologue + epilogue shape ----

    #[test]
    fn empty_fn_sysv_prologue_epilogue() {
        // fn no_args_no_body() -> void { }
        // Expected:
        //   push rbp
        //   mov  rbp, rsp
        //   (no sub rsp, frame=0)
        //   mov  rsp, rbp
        //   pop  rbp
        //   ret
        let ctx = fn_ctx("noop", &[], "void");
        let bytes = compile_function(&ctx, &[], X86_64Abi::SysV).unwrap();
        assert_eq!(bytes, vec![
            0x55,                       // push rbp
            0x48, 0x89, 0xE5,           // mov rbp, rsp
            0x48, 0x89, 0xEC,           // mov rsp, rbp
            0x5D,                       // pop rbp
            0xC3,                       // ret
        ]);
    }

    #[test]
    fn empty_fn_msx64_reserves_shadow_space() {
        // Even with no locals, MS x64 reserves 32 bytes of shadow space
        // in the prologue (treating every fn as potentially non-leaf).
        // Frame = round_up(0 + 32, 16) = 32.
        let ctx = fn_ctx("noop", &[], "void");
        let bytes = compile_function(&ctx, &[], X86_64Abi::MsX64).unwrap();
        // push rbp; mov rbp, rsp; sub rsp, 0x20; mov rsp, rbp; pop rbp; ret
        assert_eq!(&bytes[..3], &[0x55, 0x48, 0x89]);            // push rbp; mov ..
        assert_eq!(&bytes[3..7], &[0xE5, 0x48, 0x81, 0xEC]);     // mov rbp, rsp; sub
        assert_eq!(&bytes[7..11], &[0x20, 0x00, 0x00, 0x00]);    // 0x20 = 32
        // Epilogue: mov rsp, rbp; pop rbp; ret
        assert_eq!(&bytes[bytes.len()-5..], &[0x48, 0x89, 0xEC, 0x5D, 0xC3]);
    }

    // ---- Constant + return ----

    #[test]
    fn fn_returns_42_sysv() {
        // fn ret42() -> u64 { return 42; }
        //   const_u64 v0 = 42
        //   ret_u64 v0
        let ir = vec![
            instr("const_u64", Some("v0"), vec![Op::Int(42)]),
            instr("ret_u64", None, vec![Op::Var("v0".into())]),
        ];
        let ctx = fn_ctx("ret42", &[], "u64");
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // Prologue: push rbp; mov rbp, rsp; sub rsp, 0x10
        assert_eq!(&bytes[0..3],  &[0x55, 0x48, 0x89]);
        assert_eq!(&bytes[3..4],  &[0xE5]);
        assert_eq!(&bytes[4..11], &[0x48, 0x81, 0xEC, 0x10, 0x00, 0x00, 0x00]); // sub rsp, 16
        // mov rax, 42 (imm32): 48 C7 C0 2A 00 00 00
        // (rest of the function follows)
        let body_start = 11;
        assert_eq!(&bytes[body_start..body_start + 7],
                   &[0x48, 0xC7, 0xC0, 0x2A, 0x00, 0x00, 0x00]);
    }

    // ---- One-param identity ----

    #[test]
    fn fn_identity_u64_sysv_spills_rdi() {
        // fn id(x: u64) -> u64 { return x; }
        // Prologue should spill RDI (System V arg 0) to [rbp - 8].
        let params = vec![("x".to_string(), "u64".to_string())];
        let ctx = fn_ctx("id", &params, "u64");
        let ir = vec![
            instr("ret_u64", None, vec![Op::Var("x".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // After `push rbp; mov rbp, rsp; sub rsp, 16` we expect:
        //   mov [rbp - 8], rdi   →  48 89 7D F8   (disp8 form)
        // But the encoder always emits disp32 form, so:
        //   mov [rbp - 8], rdi   →  48 89 BD F8 FF FF FF
        // Find the spill — it starts at offset 11.
        assert_eq!(&bytes[11..18], &[0x48, 0x89, 0xBD, 0xF8, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn fn_identity_u64_msx64_spills_rcx() {
        // Same function, MS x64 ABI: arg 0 is RCX.
        let params = vec![("x".to_string(), "u64".to_string())];
        let ctx = fn_ctx("id", &params, "u64");
        let ir = vec![
            instr("ret_u64", None, vec![Op::Var("x".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::MsX64).unwrap();
        // Frame = round_up(8 + 32 shadow, 16) = 48 = 0x30.
        // sub rsp, 0x30: 48 81 EC 30 00 00 00
        assert_eq!(&bytes[4..11], &[0x48, 0x81, 0xEC, 0x30, 0x00, 0x00, 0x00]);
        // Spill rcx (NOT rdi): mov [rbp - 8], rcx → 48 89 8D F8 FF FF FF
        assert_eq!(&bytes[11..18], &[0x48, 0x89, 0x8D, 0xF8, 0xFF, 0xFF, 0xFF]);
    }

    // ---- Add + return ----

    #[test]
    fn fn_add_u64() {
        // fn add(a: u64, b: u64) -> u64 { return a + b; }
        // CIR: add_u64 v2 = a, b; ret_u64 v2
        let params = vec![
            ("a".to_string(), "u64".to_string()),
            ("b".to_string(), "u64".to_string()),
        ];
        let ctx = fn_ctx("add", &params, "u64");
        let ir = vec![
            instr("add_u64", Some("v2"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_u64", None, vec![Op::Var("v2".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // Just sanity-check that the body contains an `add` opcode
        // (48 01 ..) somewhere.
        let has_add = bytes.windows(2).any(|w| w == [0x48, 0x01]);
        assert!(has_add, "expected an `add r/m64, r64` somewhere in {bytes:02X?}");
    }

    // ---- Comparison ----

    #[test]
    fn parse_cmp_suffix_works() {
        assert_eq!(parse_cmp_suffix("eq_u8"),  Some((CmpRel::Eq, false)));
        assert_eq!(parse_cmp_suffix("lt_i64"), Some((CmpRel::Lt, true)));
        assert_eq!(parse_cmp_suffix("ge_u32"), Some((CmpRel::Ge, false)));
        assert_eq!(parse_cmp_suffix("ne_bool"), Some((CmpRel::Ne, false)));
        assert_eq!(parse_cmp_suffix("xx_u8"), None);
        assert_eq!(parse_cmp_suffix("eq"), None);
    }

    #[test]
    fn fn_eq_i64() {
        // fn eq(a: i64, b: i64) -> bool { return a == b; }
        let params = vec![
            ("a".to_string(), "i64".to_string()),
            ("b".to_string(), "i64".to_string()),
        ];
        let ctx = fn_ctx("eq", &params, "bool");
        let ir = vec![
            instr("cmp_eq_i64", Some("r"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_bool", None, vec![Op::Var("r".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // Body must contain `cmp` (48 39 ..) and `sete` (40 0F 94 ..)
        let has_cmp = bytes.windows(2).any(|w| w == [0x48, 0x39]);
        let has_sete = bytes.windows(3).any(|w| w == [0x40, 0x0F, 0x94]);
        assert!(has_cmp,  "expected cmp in {bytes:02X?}");
        assert!(has_sete, "expected sete in {bytes:02X?}");
    }

    // ---- Control flow ----

    #[test]
    fn fn_with_branch() {
        // fn branch(x: u64) -> u64 {
        //   if (x) jmp L1
        //   const_u64 r0 = 0
        //   jmp L2
        // L1:
        //   const_u64 r1 = 1
        // L2:
        //   ... (no merge in V1 — use a mov to a final slot)
        // }
        // CIR shape:
        //   jmp_if_true x, "L1"
        //   const_u64 v0 = 7
        //   ret_u64 v0
        //   label "L1"
        //   const_u64 v1 = 11
        //   ret_u64 v1
        let params = vec![("x".to_string(), "u64".to_string())];
        let ctx = fn_ctx("branch", &params, "u64");
        let ir = vec![
            instr("jmp_if_true", None,
                  vec![Op::Var("x".into()), Op::Var("L1".into())]),
            instr("const_u64", Some("v0"), vec![Op::Int(7)]),
            instr("ret_u64", None, vec![Op::Var("v0".into())]),
            instr("label", None, vec![Op::Var("L1".into())]),
            instr("const_u64", Some("v1"), vec![Op::Int(11)]),
            instr("ret_u64", None, vec![Op::Var("v1".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // Must contain a test rax,rax (48 85 C0) + jne rel32 (0F 85 ..).
        let has_test = bytes.windows(3).any(|w| w == [0x48, 0x85, 0xC0]);
        let has_jne = bytes.windows(2).any(|w| w == [0x0F, 0x85]);
        assert!(has_test, "expected test rax,rax in {bytes:02X?}");
        assert!(has_jne,  "expected jne in {bytes:02X?}");
    }

    // ---- Type assert ----

    #[test]
    fn type_assert_lowers_to_ud2() {
        let ctx = fn_ctx("guarded", &[], "void");
        let ir = vec![
            instr("type_assert", None, vec![]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // UD2 = 0F 0B
        let has_ud2 = bytes.windows(2).any(|w| w == [0x0F, 0x0B]);
        assert!(has_ud2, "expected UD2 (0F 0B) in {bytes:02X?}");
    }

    // ---- Errors ----

    #[test]
    fn too_many_args_sysv() {
        // 7 args > 6 GPR slots → BackendRefused.
        let params: Vec<_> = (0..7)
            .map(|i| (format!("p{i}"), "u64".to_string()))
            .collect();
        let ctx = fn_ctx("toomany", &params, "void");
        let result = compile_function(&ctx, &[], X86_64Abi::SysV);
        assert!(result.is_err());
    }

    #[test]
    fn too_many_args_msx64() {
        // 5 args > 4 GPR slots → BackendRefused on Windows.
        let params: Vec<_> = (0..5)
            .map(|i| (format!("p{i}"), "u64".to_string()))
            .collect();
        let ctx = fn_ctx("toomany", &params, "void");
        let result = compile_function(&ctx, &[], X86_64Abi::MsX64);
        assert!(result.is_err());
    }

    // ---- Backend trait ----

    #[test]
    fn backend_trait_name_reflects_abi() {
        assert_eq!(X86_64Backend::with_abi(X86_64Abi::SysV).name(), "x86_64-sysv");
        assert_eq!(X86_64Backend::with_abi(X86_64Abi::MsX64).name(), "x86_64-msx64");
    }

    // ---- LANG38-parity opcodes ----

    fn body_contains(bytes: &[u8], needle: &[u8]) -> bool {
        bytes.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn fn_div_i64_emits_cqo_idiv() {
        // fn d(a: i64, b: i64) -> i64 { return a / b; }
        let params = vec![
            ("a".to_string(), "i64".to_string()),
            ("b".to_string(), "i64".to_string()),
        ];
        let ctx = fn_ctx("d", &params, "i64");
        let ir = vec![
            instr("div_i64", Some("q"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_i64", None, vec![Op::Var("q".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        assert!(body_contains(&bytes, &[0x48, 0x99]),         "cqo missing");      // CQO
        assert!(body_contains(&bytes, &[0x48, 0xF7, 0xF9]),   "idiv rcx missing"); // IDIV rcx
    }

    #[test]
    fn fn_div_u64_emits_xor_div() {
        let params = vec![
            ("a".to_string(), "u64".to_string()),
            ("b".to_string(), "u64".to_string()),
        ];
        let ctx = fn_ctx("d", &params, "u64");
        let ir = vec![
            instr("div_u64", Some("q"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_u64", None, vec![Op::Var("q".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // Unsigned div: XOR RDX, RDX (48 31 D2) then DIV RCX (48 F7 F1)
        assert!(body_contains(&bytes, &[0x48, 0x31, 0xD2]),   "xor rdx,rdx missing");
        assert!(body_contains(&bytes, &[0x48, 0xF7, 0xF1]),   "div rcx missing");
    }

    #[test]
    fn fn_mod_i64_stores_rdx() {
        // mod: result is the remainder (RDX), not the quotient.
        let params = vec![
            ("a".to_string(), "i64".to_string()),
            ("b".to_string(), "i64".to_string()),
        ];
        let ctx = fn_ctx("m", &params, "i64");
        let ir = vec![
            instr("mod_i64", Some("r"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_i64", None, vec![Op::Var("r".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // After IDIV, the result slot is written from RDX (encoded as
        // ModR/M reg=2): `mov [rbp - off], rdx`  →  48 89 95 .. .. .. ..
        // The reg field's low 3 bits = 2, so ModR/M = (10 << 6) | (010 << 3) | rm = 0x95 when rm=5 (RBP).
        // We assert the `48 89 95` byte trio appears somewhere after IDIV.
        assert!(body_contains(&bytes, &[0x48, 0x89, 0x95]),
                "mov [rbp+disp32], rdx missing — should be storing remainder");
    }

    #[test]
    fn fn_and_or_xor_emit_correct_opcodes() {
        let params = vec![
            ("a".to_string(), "u64".to_string()),
            ("b".to_string(), "u64".to_string()),
        ];
        let ctx = fn_ctx("f", &params, "u64");
        let ir = vec![
            instr("and_u64", Some("v0"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("or_u64",  Some("v1"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("xor_u64", Some("v2"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_u64", None, vec![Op::Var("v2".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        assert!(body_contains(&bytes, &[0x48, 0x21, 0xC8]), "and rax,rcx missing");
        assert!(body_contains(&bytes, &[0x48, 0x09, 0xC8]), "or  rax,rcx missing");
        assert!(body_contains(&bytes, &[0x48, 0x31, 0xC8]), "xor rax,rcx missing");
    }

    #[test]
    fn fn_shl_u64_emits_shl_cl() {
        let params = vec![
            ("a".to_string(), "u64".to_string()),
            ("b".to_string(), "u64".to_string()),
        ];
        let ctx = fn_ctx("s", &params, "u64");
        let ir = vec![
            instr("shl_u64", Some("v"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_u64", None, vec![Op::Var("v".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // shl rax, cl: 48 D3 E0
        assert!(body_contains(&bytes, &[0x48, 0xD3, 0xE0]), "shl rax,cl missing");
    }

    #[test]
    fn fn_shr_signed_emits_sar() {
        // shr_i64 must lower to SAR (arithmetic shift), not SHR (logical).
        let params = vec![
            ("a".to_string(), "i64".to_string()),
            ("b".to_string(), "i64".to_string()),
        ];
        let ctx = fn_ctx("s", &params, "i64");
        let ir = vec![
            instr("shr_i64", Some("v"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_i64", None, vec![Op::Var("v".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // sar rax, cl: 48 D3 F8
        assert!(body_contains(&bytes, &[0x48, 0xD3, 0xF8]), "sar rax,cl missing");
        // And NOT shr: 48 D3 E8
        assert!(!body_contains(&bytes, &[0x48, 0xD3, 0xE8]), "shr unexpectedly present");
    }

    #[test]
    fn fn_shr_unsigned_emits_shr() {
        let params = vec![
            ("a".to_string(), "u64".to_string()),
            ("b".to_string(), "u64".to_string()),
        ];
        let ctx = fn_ctx("s", &params, "u64");
        let ir = vec![
            instr("shr_u64", Some("v"),
                  vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_u64", None, vec![Op::Var("v".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        assert!(body_contains(&bytes, &[0x48, 0xD3, 0xE8]), "shr rax,cl missing");
        assert!(!body_contains(&bytes, &[0x48, 0xD3, 0xF8]), "sar unexpectedly present");
    }

    #[test]
    fn fn_neg_emits_neg() {
        let params = vec![("a".to_string(), "i64".to_string())];
        let ctx = fn_ctx("n", &params, "i64");
        let ir = vec![
            instr("neg_i64", Some("v"), vec![Op::Var("a".into())]),
            instr("ret_i64", None, vec![Op::Var("v".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        assert!(body_contains(&bytes, &[0x48, 0xF7, 0xD8]), "neg rax missing");
    }

    // ---- Calls ----

    #[test]
    fn fn_call_external_records_reloc() {
        // fn caller() -> u64 { return external(42); }
        // CIR: const_u64 v0=42; call external, v0 → r; ret_u64 r
        let ctx = fn_ctx("caller", &[], "u64");
        let ir = vec![
            instr("const_u64", Some("v0"), vec![Op::Int(42)]),
            instr("call", Some("r"),
                  vec![Op::Var("external".into()), Op::Var("v0".into())]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let (bytes, relocs) = compile_function_with_relocs(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // Must contain `mov rdi, [rbp-...]` (System V arg 0)
        // and `call rel32` (E8 ?? ?? ?? ??) with a recorded reloc.
        assert!(body_contains(&bytes, &[0xE8]),
                "expected `call rel32` opcode (E8) in {bytes:02X?}");
        assert_eq!(relocs.len(), 1, "expected exactly one external reloc");
        assert_eq!(relocs[0].symbol, "external");
        assert_eq!(relocs[0].kind, ExternalRelocKind::PltRel32);
        assert_eq!(relocs[0].addend, -4);
    }

    #[test]
    fn fn_self_recursive_call_no_reloc() {
        // fn fact(n: u64) -> u64 { return fact(n); }  (degenerate but tests the wiring)
        // CIR: call fact, n → r; ret_u64 r
        let params = vec![("n".to_string(), "u64".to_string())];
        let ctx = fn_ctx("fact", &params, "u64");
        let ir = vec![
            instr("call", Some("r"),
                  vec![Op::Var("fact".into()), Op::Var("n".into())]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let (bytes, relocs) = compile_function_with_relocs(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // Self-recursive: no external relocation.
        assert!(relocs.is_empty(),
                "self-recursive call should not record external reloc, got {relocs:?}");
        // Must contain a CALL opcode (E8).
        assert!(body_contains(&bytes, &[0xE8]),
                "expected `call rel32` opcode (E8) in {bytes:02X?}");
    }

    #[test]
    fn fn_call_msx64_loads_into_rcx() {
        // MS x64: arg 0 → RCX, not RDI.
        let ctx = fn_ctx("caller", &[], "u64");
        let ir = vec![
            instr("const_u64", Some("v0"), vec![Op::Int(42)]),
            instr("call", Some("r"),
                  vec![Op::Var("external".into()), Op::Var("v0".into())]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::MsX64).unwrap();
        // Look for `mov rcx, [rbp-disp]` — encoded as 48 8B 8D .. .. .. ..
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0x8D]),
                "expected `mov rcx, [rbp+disp32]` in MS x64 call setup");
        // And NOT `mov rdi, [rbp-disp]` (48 8B BD) — the SysV arg-0 load.
        assert!(!body_contains(&bytes, &[0x48, 0x8B, 0xBD]),
                "should NOT emit `mov rdi, ...` on MS x64");
    }

    #[test]
    fn fn_call_too_many_args_msx64() {
        // 5 args > 4 GPR slots on MS x64.
        let ctx = fn_ctx("caller", &[], "u64");
        let mut srcs = vec![Op::Var("external".into())];
        for i in 0..5 { srcs.push(Op::Int(i)); }
        let ir = vec![
            instr("call", Some("r"), srcs),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let result = compile_function(&ctx, &ir, X86_64Abi::MsX64);
        assert!(result.is_err(), "expected `too many args` error");
    }

    #[test]
    fn fn_not_emits_not() {
        let params = vec![("a".to_string(), "u64".to_string())];
        let ctx = fn_ctx("n", &params, "u64");
        let ir = vec![
            instr("not_u64", Some("v"), vec![Op::Var("a".into())]),
            instr("ret_u64", None, vec![Op::Var("v".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        assert!(body_contains(&bytes, &[0x48, 0xF7, 0xD0]), "not rax missing");
    }

    #[test]
    fn backend_trait_compile_function() {
        let ctx = fn_ctx("noop", &[], "void");
        let backend = X86_64Backend::with_abi(X86_64Abi::SysV);
        let bytes = backend.compile_function(&ctx, &[]).unwrap();
        assert!(bytes.starts_with(&[0x55, 0x48, 0x89, 0xE5])); // push rbp; mov rbp, rsp
    }
}
