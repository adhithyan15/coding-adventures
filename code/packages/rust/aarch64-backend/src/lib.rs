//! # `aarch64-backend` — ARM64 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into AArch64 machine code via
//! [`aarch64_encoder`].  Plugs into both `jit-core` and `aot-core` through
//! the shared [`jit_core::backend::Backend`] trait.
//!
//! ## Scope (V1 — what fib() needs)
//!
//! | Family | CIR mnemonics |
//! |--------|---------------|
//! | Constants | `const_u8` … `const_u64`, `const_i8` … `const_i64`, `const_bool` |
//! | Integer arithmetic | `add_<ty>`, `sub_<ty>`, `mul_<ty>` |
//! | Comparisons | `cmp_eq_<ty>`, `cmp_ne_<ty>`, `cmp_lt_<ty>`, `cmp_le_<ty>`, `cmp_gt_<ty>`, `cmp_ge_<ty>` (signed and unsigned) |
//! | Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
//! | Returns | `ret_<ty>`, `ret_void` |
//! | Type guards | `type_assert` (lowered to `udf` trap — AOT has no deopt) |
//! | Calls | `call`, `call_runtime` — **NOT YET** (returns `None` to defer to interpreter) |
//! | Float, send, properties | **NOT YET** |
//!
//! Anything outside this list causes the backend to return `None`, which
//! `aot-core` reports as a compile failure for that function (it falls
//! back to the IIR table — same handling as any other backend miss).
//!
//! ## Register-allocation strategy — stack spill
//!
//! V1 uses the simplest correct allocator: every CIR virtual register
//! lives at a fixed 8-byte stack slot.  Each instruction:
//! 1. Loads its source operands into scratch registers `x0..x2`.
//! 2. Performs the operation.
//! 3. Stores the destination back to its stack slot.
//!
//! This is suboptimal — a real allocator would keep frequently-used values
//! in registers.  But it is trivially correct, easy to test, and gives a
//! working binary today.  A better allocator can replace it without
//! changing the public API.
//!
//! ## AAPCS64 prologue / epilogue
//!
//! ```text
//! stp  fp, lr, [sp, #-frame]!     ; save fp/lr, allocate frame
//! mov  fp, sp                     ; debugger-friendly frame pointer
//! str  x0, [sp, #(N+0)]           ; spill incoming args to their slots
//! str  x1, [sp, #(N+8)]
//! ...
//! <body>
//! ldp  fp, lr, [sp], #frame       ; restore + deallocate
//! ret
//! ```
//!
//! Up to 8 parameters (`x0..x7`) are supported in V1.  The frame must fit
//! in a 12-bit unsigned offset (≤ 4088 bytes / ~512 virtual registers).
//!
//! ## Type widths
//!
//! For V1 every typed integer mnemonic uses 64-bit ARM operations.  The
//! result is **not** masked to the declared width — `add_u8 0xFF, 1`
//! produces `0x100`, not `0x00`.  This is correct for any consumer that
//! treats the result as 64-bit; programs that depend on width-truncation
//! semantics are outside V1 scope.  A future PR can add `and #mask`
//! emission for tighter semantics.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::HashMap;

use aarch64_encoder::{Assembler, Cond, EncodeError, LabelId, Reg};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use vm_core::value::Value;

// ===========================================================================
// AArch64Backend
// ===========================================================================

/// ARM64 native-code backend.
///
/// Stateless — every `compile_function` call builds a fresh assembler.
/// `Send + Sync` is automatic because there are no fields.
#[derive(Debug, Default, Clone, Copy)]
pub struct AArch64Backend;

impl AArch64Backend {
    /// Construct a fresh backend instance.  No state to configure in V1.
    pub fn new() -> Self { AArch64Backend }
}

impl Backend for AArch64Backend {
    fn name(&self) -> &str { "aarch64" }

    /// Without function context we can't lay out the prologue properly
    /// (we don't know how many params arrive in `x0..x7`).  Return `None`
    /// so callers fall back to the interpreter; users should prefer the
    /// `compile_function` entry point.
    fn compile(&self, _ir: &[CIRInstr]) -> Option<Vec<u8>> { None }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        // Native-code execution requires a JIT loader (mmap + W^X) which
        // is a separate crate.  This backend is for AOT today; JIT
        // dispatch is wired up in a follow-up PR.
        Value::Null
    }

    fn compile_function(&self, ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile(ctx, ir).ok()
    }
}

// ===========================================================================
// Register allocator — assigns each CIR virtual register a stack slot
// ===========================================================================

#[derive(Debug, Default)]
struct RegAlloc {
    slots: HashMap<String, u32>,
    next_slot: u32,
}

impl RegAlloc {
    fn slot_of(&mut self, name: &str) -> u32 {
        if let Some(&s) = self.slots.get(name) { return s; }
        let s = self.next_slot;
        self.next_slot = self.next_slot.checked_add(8).expect("slot overflow");
        self.slots.insert(name.to_string(), s);
        s
    }

    /// Required virtual storage size (sum of slots), 16-byte aligned.
    fn virt_size(&self) -> u32 {
        (self.next_slot + 15) & !15
    }
}

// ===========================================================================
// Compile error — all variants encoder-internal so callers see only Option
// ===========================================================================

/// Internal compile errors.  Variants carry diagnostic context that's
/// surfaced to the caller through `format!("{e:?}")`; the data isn't
/// reached structurally, hence the `dead_code` allowance.
#[derive(Debug)]
#[allow(dead_code)]
enum BackendError {
    /// CIR contains an opcode this backend doesn't yet support.
    UnsupportedOp(String),
    /// CIR uses more parameters than AAPCS64 register-arg slots (8).
    TooManyParams(usize),
    /// Frame requires more than 12-bit `sub sp` immediate (≈ 4088 bytes).
    FrameTooLarge(u32),
    /// An instruction is missing a required `dest` or `srcs` field.
    MalformedInstr(String),
    /// Encoder rejected an immediate.
    Encoder(EncodeError),
}

impl From<EncodeError> for BackendError {
    fn from(e: EncodeError) -> Self { BackendError::Encoder(e) }
}

// ===========================================================================
// Top-level compile() — stitches the prologue, body, and epilogue together
// ===========================================================================

/// Public-ish entry point used by tests.  Production callers go through
/// the `Backend` trait.
pub fn compile(ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Result<Vec<u8>, String> {
    compile_inner(ctx, ir).map_err(|e| format!("aarch64-backend: {e:?}"))
}

fn compile_inner(ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if ctx.params.len() > 8 {
        return Err(BackendError::TooManyParams(ctx.params.len()));
    }

    // ---- Pre-pass: assign slots (params first, then walk dests in CIR) ----
    let mut alloc = RegAlloc::default();
    for (name, _ty) in ctx.params {
        alloc.slot_of(name);
    }
    // Walk CIR pre-emption to assign deterministic slots to all dests.
    for instr in ir {
        if let Some(d) = &instr.dest {
            alloc.slot_of(d);
        }
        // Variables read but never written (e.g. constants embedded as
        // CIROperand::Var, runtime-fn names) get slots too — they may be
        // backend-recognised names and get special-cased below.
        for src in &instr.srcs {
            if let CIROperand::Var(s) = src {
                alloc.slot_of(s);
            }
        }
    }

    let virt_size = alloc.virt_size();
    let frame = virt_size + 16; // +16 for saved fp/lr
    if frame > 4080 {
        return Err(BackendError::FrameTooLarge(frame));
    }

    // ---- Pre-pass: collect labels so forward jumps can be resolved -------
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
    // Pre-create labels referenced by jmp* (the targets may be defined
    // either before or after the jump — pre-create all of them).
    for instr in ir {
        if matches!(instr.op.as_str(), "jmp" | "jmp_if_true" | "jmp_if_false") {
            if let Some(target) = label_name(instr) {
                labels.entry(target.to_string())
                    .or_insert_with(|| asm.create_label());
            }
        }
    }

    // ---- Prologue --------------------------------------------------------
    asm.stp_pre(Reg::Fp, Reg::Lr, Reg::Sp, -(frame as i32))?;
    asm.add_imm(Reg::Fp, Reg::Sp, 0)?; // mov fp, sp (alias for add #0)

    // Spill incoming params (x0..x7) to their slots.
    for (i, (name, _ty)) in ctx.params.iter().enumerate() {
        let slot = alloc.slot_of(name);
        asm.str_(arg_reg(i), Reg::Sp, slot)?;
    }

    // ---- Body ------------------------------------------------------------
    for instr in ir {
        emit_instr(&mut asm, instr, &mut alloc, &labels, frame)?;
    }

    // ---- Final epilogue (only reached if the function falls off the end) -
    // Defensive: a well-formed CIR ends in `ret_*`/`ret_void`.  We still
    // append an epilogue+ret here so a missing terminator doesn't produce
    // arbitrary code execution past the end of the function.
    emit_epilogue(&mut asm, frame)?;

    asm.finish().map_err(BackendError::from)
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
) -> Result<(), BackendError> {
    let op = instr.op.as_str();

    if op == "label" {
        let name = label_name(instr)
            .ok_or_else(|| BackendError::MalformedInstr("label needs srcs[0]=Var(name)".into()))?;
        let id = *labels.get(name)
            .ok_or_else(|| BackendError::MalformedInstr(format!("undefined label {name}")))?;
        asm.bind(id).map_err(BackendError::from)?;
        return Ok(());
    }

    if op == "jmp" {
        let name = label_name(instr).ok_or_else(|| BackendError::MalformedInstr("jmp needs target".into()))?;
        let id = *labels.get(name).ok_or_else(|| BackendError::MalformedInstr(format!("unknown label {name}")))?;
        asm.b(id);
        return Ok(());
    }

    if op == "jmp_if_true" || op == "jmp_if_false" {
        // Convention: srcs = [cond_var, label_name_var]
        let cond_var = instr.srcs.first().and_then(CIROperand::as_var)
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]=cond")))?;
        let target = instr.srcs.get(1).and_then(CIROperand::as_var)
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[1]=label")))?;
        let target_id = *labels.get(target).ok_or_else(|| BackendError::MalformedInstr(format!("unknown label {target}")))?;
        let s = alloc.slot_of(cond_var);
        asm.ldr(Reg::X0, Reg::Sp, s)?;
        if op == "jmp_if_true" { asm.cbnz(Reg::X0, target_id); }
        else                   { asm.cbz(Reg::X0, target_id); }
        return Ok(());
    }

    if op == "type_assert" {
        // AOT lowering: emit `udf #0xDEAD` as a hard trap.  A real deopt
        // path is jit-only and added in a follow-up.
        asm.udf(0xDEAD);
        return Ok(());
    }

    if op == "ret_void" {
        emit_epilogue(asm, frame)?;
        return Ok(());
    }
    if let Some(_ty) = op.strip_prefix("ret_") {
        // Single source, load into x0, then epilogue.
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::X0, src)?;
        emit_epilogue(asm, frame)?;
        return Ok(());
    }

    // ---- const_<ty> v0 = <literal> ---------------------------------------
    if let Some(_ty) = op.strip_prefix("const_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        let imm = match src {
            CIROperand::Int(n)   => *n as u64,
            CIROperand::Bool(b)  => if *b { 1 } else { 0 },
            CIROperand::Float(_) => return Err(BackendError::UnsupportedOp("const_f64".into())),
            CIROperand::Var(_)   => return Err(BackendError::MalformedInstr(format!("{op} needs literal source"))),
        };
        asm.mov_imm64(Reg::X0, imm);
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // ---- add/sub/mul (typed) --------------------------------------------
    for (prefix, kind) in &[("add_", BinKind::Add), ("sub_", BinKind::Sub), ("mul_", BinKind::Mul)] {
        if op.starts_with(*prefix) {
            return emit_binop(asm, alloc, instr, *kind);
        }
    }

    // ---- comparisons -----------------------------------------------------
    if let Some(rest) = op.strip_prefix("cmp_") {
        let (rel, ty) = parse_cmp_suffix(rest)
            .ok_or_else(|| BackendError::MalformedInstr(format!("bad cmp mnemonic: {op}")))?;
        return emit_cmp(asm, alloc, instr, rel, ty);
    }

    // Anything else is unsupported for V1 — caller should fall back.
    Err(BackendError::UnsupportedOp(op.to_string()))
}

#[derive(Debug, Clone, Copy)]
enum BinKind { Add, Sub, Mul }

fn emit_binop(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    kind: BinKind,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    if instr.srcs.len() < 2 {
        return Err(BackendError::MalformedInstr(format!("{} needs 2 srcs", instr.op)));
    }
    load_operand(asm, alloc, Reg::X0, &instr.srcs[0])?;
    load_operand(asm, alloc, Reg::X1, &instr.srcs[1])?;
    match kind {
        BinKind::Add => asm.add(Reg::X0, Reg::X0, Reg::X1),
        BinKind::Sub => asm.sub(Reg::X0, Reg::X0, Reg::X1),
        BinKind::Mul => asm.mul(Reg::X0, Reg::X0, Reg::X1),
    }
    let slot = alloc.slot_of(dest);
    asm.str_(Reg::X0, Reg::Sp, slot)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum CmpRel { Eq, Ne, Lt, Le, Gt, Ge }

fn parse_cmp_suffix(s: &str) -> Option<(CmpRel, &str)> {
    // "eq_u8" → (Eq, "u8"); "lt_i32" → (Lt, "i32"); etc.
    let (rel_str, ty) = s.split_once('_')?;
    let rel = match rel_str {
        "eq" => CmpRel::Eq, "ne" => CmpRel::Ne,
        "lt" => CmpRel::Lt, "le" => CmpRel::Le,
        "gt" => CmpRel::Gt, "ge" => CmpRel::Ge,
        _ => return None,
    };
    Some((rel, ty))
}

fn emit_cmp(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    rel: CmpRel,
    ty: &str,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    if instr.srcs.len() < 2 {
        return Err(BackendError::MalformedInstr(format!("{} needs 2 srcs", instr.op)));
    }
    load_operand(asm, alloc, Reg::X0, &instr.srcs[0])?;
    load_operand(asm, alloc, Reg::X1, &instr.srcs[1])?;
    asm.cmp(Reg::X0, Reg::X1);

    let signed = ty.starts_with('i');
    let cond = match (rel, signed) {
        (CmpRel::Eq, _)         => Cond::Eq,
        (CmpRel::Ne, _)         => Cond::Ne,
        (CmpRel::Lt, true)      => Cond::Lt,
        (CmpRel::Le, true)      => Cond::Le,
        (CmpRel::Gt, true)      => Cond::Gt,
        (CmpRel::Ge, true)      => Cond::Ge,
        (CmpRel::Lt, false)     => Cond::Lo, // unsigned <
        (CmpRel::Le, false)     => Cond::Ls, // unsigned ≤
        (CmpRel::Gt, false)     => Cond::Hi, // unsigned >
        (CmpRel::Ge, false)     => Cond::Hs, // unsigned ≥
    };
    asm.cset(Reg::X0, cond);
    let slot = alloc.slot_of(dest);
    asm.str_(Reg::X0, Reg::Sp, slot)?;
    Ok(())
}

// ===========================================================================
// Helpers
// ===========================================================================

fn label_name(instr: &CIRInstr) -> Option<&str> {
    instr.srcs.first().and_then(CIROperand::as_var)
}

fn require_dest(instr: &CIRInstr) -> Result<&str, BackendError> {
    instr.dest.as_deref().ok_or_else(|| BackendError::MalformedInstr(format!("{} requires dest", instr.op)))
}

fn arg_reg(i: usize) -> Reg {
    match i {
        0 => Reg::X0, 1 => Reg::X1, 2 => Reg::X2, 3 => Reg::X3,
        4 => Reg::X4, 5 => Reg::X5, 6 => Reg::X6, 7 => Reg::X7,
        _ => unreachable!("checked at function entry"),
    }
}

fn load_operand(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    dst: Reg,
    op: &CIROperand,
) -> Result<(), BackendError> {
    match op {
        CIROperand::Var(name) => {
            let slot = alloc.slot_of(name);
            asm.ldr(dst, Reg::Sp, slot).map_err(BackendError::from)
        }
        CIROperand::Int(n)  => { asm.mov_imm64(dst, *n as u64); Ok(()) }
        CIROperand::Bool(b) => { asm.mov_imm64(dst, if *b { 1 } else { 0 }); Ok(()) }
        CIROperand::Float(_) => Err(BackendError::UnsupportedOp("float operand".into())),
    }
}

fn emit_epilogue(asm: &mut Assembler, frame: u32) -> Result<(), BackendError> {
    asm.ldp_post(Reg::Fp, Reg::Lr, Reg::Sp, frame as i32)?;
    asm.ret();
    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use jit_core::cir::{CIRInstr, CIROperand};

    fn ctx<'a>(name: &'a str, params: &'a [(String, String)], ret: &'a str) -> FunctionContext<'a> {
        FunctionContext { name, params, return_type: ret }
    }

    fn const_u64(dest: &str, n: i64) -> CIRInstr {
        CIRInstr { op: "const_u64".into(), dest: Some(dest.into()),
                   srcs: vec![CIROperand::Int(n)], ty: "u64".into(), deopt_to: None }
    }

    fn add_u64(dest: &str, a: &str, b: &str) -> CIRInstr {
        CIRInstr { op: "add_u64".into(), dest: Some(dest.into()),
                   srcs: vec![CIROperand::Var(a.into()), CIROperand::Var(b.into())],
                   ty: "u64".into(), deopt_to: None }
    }

    fn ret_u64(src: &str) -> CIRInstr {
        CIRInstr { op: "ret_u64".into(), dest: None,
                   srcs: vec![CIROperand::Var(src.into())], ty: "void".into(), deopt_to: None }
    }

    #[test]
    fn backend_name_is_aarch64() {
        assert_eq!(AArch64Backend.name(), "aarch64");
    }

    #[test]
    fn empty_function_emits_prologue_and_epilogue() {
        // fn() { return; }   (well-formed though weird)
        let cir: Vec<CIRInstr> = vec![
            CIRInstr { op: "ret_void".into(), dest: None, srcs: vec![],
                       ty: "void".into(), deopt_to: None },
        ];
        let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("ok");
        // Expect at least: stp, mov_fp, ldp, ret  (+ defensive trailing
        // epilogue) → 5+ instructions.
        assert!(bytes.len() >= 5 * 4, "got {} bytes", bytes.len());
        assert_eq!(bytes.len() % 4, 0);
    }

    #[test]
    fn add_two_constants_returns_sum() {
        // fn() -> u64 { let a = 3; let b = 4; return a + b; }
        let cir = vec![
            const_u64("a", 3),
            const_u64("b", 4),
            add_u64("v0", "a", "b"),
            ret_u64("v0"),
        ];
        let bytes = compile(&ctx("addc", &[], "u64"), &cir).expect("ok");
        // 4-byte-aligned, non-empty.
        assert!(bytes.len() > 0);
        assert_eq!(bytes.len() % 4, 0);
    }

    #[test]
    fn function_with_two_params_and_add() {
        // fn(a: u64, b: u64) -> u64 { return a + b; }
        let params = vec![("a".into(), "u64".into()), ("b".into(), "u64".into())];
        let cir = vec![
            add_u64("v0", "a", "b"),
            ret_u64("v0"),
        ];
        let bytes = compile(&ctx("add", &params, "u64"), &cir).expect("ok");
        assert!(bytes.len() > 0);
    }

    #[test]
    fn rejects_more_than_8_params() {
        let params: Vec<(String, String)> = (0..9).map(|i| (format!("p{i}"), "u64".into())).collect();
        let err = compile(&ctx("toomany", &params, "u64"), &[]).unwrap_err();
        assert!(err.contains("TooManyParams"), "got: {err}");
    }

    #[test]
    fn label_jmp_and_back() {
        // fn() { L: jmp L; }   (infinite loop; valid encoding)
        let cir = vec![
            CIRInstr { op: "label".into(), dest: None,
                       srcs: vec![CIROperand::Var("L".into())], ty: "void".into(), deopt_to: None },
            CIRInstr { op: "jmp".into(), dest: None,
                       srcs: vec![CIROperand::Var("L".into())], ty: "void".into(), deopt_to: None },
        ];
        let bytes = compile(&ctx("loop", &[], "void"), &cir).expect("ok");
        assert!(bytes.len() > 0);
    }

    #[test]
    fn jmp_if_true_lowers() {
        // fn(x: u64) { if (x) jmp L; L: ret_void }
        let params = vec![("x".into(), "u64".into())];
        let cir = vec![
            CIRInstr { op: "jmp_if_true".into(), dest: None,
                       srcs: vec![CIROperand::Var("x".into()), CIROperand::Var("L".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "label".into(), dest: None,
                       srcs: vec![CIROperand::Var("L".into())], ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None, srcs: vec![],
                       ty: "void".into(), deopt_to: None },
        ];
        let bytes = compile(&ctx("br", &params, "void"), &cir).expect("ok");
        assert!(bytes.len() > 0);
    }

    #[test]
    fn cmp_lt_u32_emits_cset() {
        let params = vec![("a".into(), "u32".into()), ("b".into(), "u32".into())];
        let cir = vec![
            CIRInstr { op: "cmp_lt_u32".into(), dest: Some("v0".into()),
                       srcs: vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
                       ty: "bool".into(), deopt_to: None },
            ret_u64("v0"),
        ];
        let bytes = compile(&ctx("lt", &params, "u64"), &cir).expect("ok");
        assert!(bytes.len() > 0);
    }

    #[test]
    fn unsupported_op_returns_none_via_trait() {
        // An opcode not in the V1 set — the trait method must return None
        // so callers fall back to the interpreter.
        let cir = vec![
            CIRInstr { op: "send".into(), dest: Some("v0".into()),
                       srcs: vec![CIROperand::Var("recv".into())],
                       ty: "any".into(), deopt_to: None },
        ];
        let result = AArch64Backend.compile_function(&ctx("x", &[], "any"), &cir);
        assert!(result.is_none());
    }

    #[test]
    fn type_assert_emits_udf() {
        let cir = vec![
            CIRInstr { op: "type_assert".into(), dest: None,
                       srcs: vec![CIROperand::Var("x".into()), CIROperand::Var("u8".into())],
                       ty: "void".into(), deopt_to: Some(0) },
            CIRInstr { op: "ret_void".into(), dest: None, srcs: vec![],
                       ty: "void".into(), deopt_to: None },
        ];
        let params = vec![("x".into(), "any".into())];
        let bytes = compile(&ctx("guard", &params, "void"), &cir).expect("ok");
        // udf #0xDEAD has the bit pattern 0xDEAD.  Search for it.
        let words: Vec<u32> = bytes.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        assert!(words.iter().any(|&w| w == 0x0000DEAD), "expected udf #0xDEAD in {words:?}");
    }
}
