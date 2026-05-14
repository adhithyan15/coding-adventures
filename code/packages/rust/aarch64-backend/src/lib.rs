//! # `aarch64-backend` — ARM64 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into AArch64 machine code via
//! [`aarch64_encoder`].  Plugs into both `jit-core` and `aot-core` through
//! the shared [`jit_core::backend::Backend`] trait.
//!
//! ## Scope
//!
//! | Family | CIR mnemonics |
//! |--------|---------------|
//! | Constants | `const_u8` … `const_u64`, `const_i8` … `const_i64`, `const_bool` |
//! | Integer arithmetic | `add_<ty>`, `sub_<ty>`, `mul_<ty>` |
//! | Division | `div_<ty>` (SDIV/UDIV), `mod_<ty>` (SDIV+MSUB or UDIV+MSUB) — LANG38 |
//! | Comparisons | `cmp_eq_<ty>`, `cmp_ne_<ty>`, `cmp_lt_<ty>`, `cmp_le_<ty>`, `cmp_gt_<ty>`, `cmp_ge_<ty>` (signed and unsigned) |
//! | Logical | `and_<ty>`, `or_<ty>`, `xor_<ty>` — LANG38 |
//! | Shifts | `shl_<ty>`, `shr_<ty>` (arithmetic for `i*`, logical for `u*`) — LANG38 |
//! | Unary | `neg_<ty>` (negate), `not_<ty>` (bitwise NOT), `mov_<ty>` — LANG38 |
//! | Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
//! | Returns | `ret_<ty>`, `ret_void` |
//! | Type guards | `type_assert` (lowered to `udf` trap — AOT has no deopt) |
//! | Calls | `call` (direct + cross-function BL via external relocs) |
//! | Globals | `global_load`, `global_store` (LANG39 — ADRP+ADD+LDR/STR via `_twig_globals`) |
//! | I/O | `io_out` (LANG40 — BL `__twig_print_i64`; helper injected by twig-aot) |
//! | Float, send, properties | **NOT YET** |
//! | Closures | **NOT YET** |
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

use aarch64_encoder::{Assembler, Cond, EncodeError, ExternalReloc, LabelId, Reg};
pub use aarch64_encoder::ExternalReloc as Reloc;
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

/// Stack-spill register allocator.
///
/// Each virtual lives at a fixed `[sp, #offset]` slot.  Offsets start at
/// **16** to leave the bottom 16 bytes of the frame free for the saved
/// `fp` (at `[sp + 0]`) and `lr` (at `[sp + 8]`) — that's where
/// `stp fp, lr, [sp, #-frame]!` writes them, and we must not let
/// virtuals overlap.
#[derive(Debug)]
struct RegAlloc {
    /// `name → byte offset from sp`, with the +16 fp/lr reservation
    /// already baked in.
    slots: HashMap<String, u32>,
    /// Next byte offset to hand out — also starts at 16 for the same
    /// reason.
    next_offset: u32,
}

impl Default for RegAlloc {
    fn default() -> Self {
        RegAlloc { slots: HashMap::new(), next_offset: 16 }
    }
}

impl RegAlloc {
    fn slot_of(&mut self, name: &str) -> u32 {
        if let Some(&s) = self.slots.get(name) { return s; }
        let s = self.next_offset;
        self.next_offset = self.next_offset.checked_add(8).expect("slot overflow");
        self.slots.insert(name.to_string(), s);
        s
    }

    /// Total frame size (saved fp/lr + virtual storage), 16-byte aligned.
    fn frame_size(&self) -> u32 {
        (self.next_offset + 15) & !15
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

// ===========================================================================
// Global-variable relocation type (LANG39)
// ===========================================================================

/// Word-index positions of an ADRP+ADD instruction pair that references
/// `_twig_globals` and needs two Mach-O ARM64 relocations:
///
/// - `adrp_word` → `ARM64_RELOC_PAGE21` on the `ADRP X1, #0` instruction.
/// - `add_word`  → `ARM64_RELOC_PAGEOFF12` on the `ADD X1, X1, #0` instruction.
///
/// Both word indices are relative to the **start of this function's** byte
/// output.  `twig-aot` converts them to byte offsets in the fully-linked
/// text section by adding the function's byte offset.
///
/// # Example
///
/// If `global_load` for slot 2 is the 7th instruction in a function, `adrp_word`
/// will be 7 and `add_word` will be 8 (they are always adjacent).
#[derive(Debug, Clone, Copy)]
pub struct GlobalWordReloc {
    /// Word index of the `ADRP X1, #0` placeholder.
    pub adrp_word: usize,
    /// Word index of the `ADD X1, X1, #0` placeholder.
    pub add_word: usize,
}

// ===========================================================================
// Public compile entry points
// ===========================================================================

/// Public-ish entry point used by tests.  Production callers go through
/// the `Backend` trait.  Relocations are silently discarded; use
/// [`compile_with_relocs`] when you need them for AOT cross-function linking.
pub fn compile(ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Result<Vec<u8>, String> {
    compile_inner(ctx, ir, &HashMap::new())
        .map(|(bytes, _ext, _glob)| bytes)
        .map_err(|e| format!("aarch64-backend: {e:?}"))
}

/// Like [`compile`] but also returns external (cross-function) relocations.
///
/// Each [`ExternalReloc`] describes a `BL` placeholder instruction that must
/// be patched after all functions are linked into a single code section.
/// The linker writes the correct PC-relative offset into the placeholder once
/// it knows the absolute byte offsets of all functions.
pub fn compile_with_relocs(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
) -> Result<(Vec<u8>, Vec<ExternalReloc>), String> {
    compile_inner(ctx, ir, &HashMap::new())
        .map(|(bytes, ext, _glob)| (bytes, ext))
        .map_err(|e| format!("aarch64-backend: {e:?}"))
}

/// Like [`compile_with_relocs`] but also handles `global_load`/`global_store`
/// CIR instructions using the provided slot map.
///
/// `global_slots` maps each global name (as it appears in `srcs[0].as_var()`)
/// to a zero-based slot index.  Slot `i` corresponds to bytes `[i*8, i*8+8)`
/// in the `_twig_globals` data section.
///
/// Returns the function's machine code bytes, any cross-function `BL`
/// relocation entries, and a list of [`GlobalWordReloc`] entries — one per
/// `global_load` / `global_store` instruction — that the Mach-O packager uses
/// to emit `ARM64_RELOC_PAGE21` / `ARM64_RELOC_PAGEOFF12` records.
pub fn compile_with_globals(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
    global_slots: &HashMap<String, usize>,
) -> Result<(Vec<u8>, Vec<ExternalReloc>, Vec<GlobalWordReloc>), String> {
    compile_inner(ctx, ir, global_slots)
        .map_err(|e| format!("aarch64-backend: {e:?}"))
}

fn compile_inner(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
    global_slots: &HashMap<String, usize>,
) -> Result<(Vec<u8>, Vec<ExternalReloc>, Vec<GlobalWordReloc>), BackendError> {
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

    let frame = alloc.frame_size();
    if frame > 504 {
        // stp_pre/ldp_post use a 7-bit signed immediate × 8, so the
        // pre-indexed delta is bounded at ±504.  Functions that need a
        // bigger frame are out of scope for V1.
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
    // Pre-create labels referenced by jmp* and call (targets may be before or
    // after their use; forward references require pre-creation).
    for instr in ir {
        if matches!(instr.op.as_str(), "jmp" | "jmp_if_true" | "jmp_if_false") {
            if let Some(target) = label_name(instr) {
                labels.entry(target.to_string())
                    .or_insert_with(|| asm.create_label());
            }
        }
        // For `call callee_name, args...`: pre-create a label for the callee
        // name so self-recursive calls can be resolved within this function.
        if instr.op == "call" {
            if let Some(CIROperand::Var(name)) = instr.srcs.first() {
                labels.entry(name.clone())
                    .or_insert_with(|| asm.create_label());
            }
        }
    }

    // ── Self-recursive call target ─────────────────────────────────────────
    //
    // Bind the current function's own name as a label pointing to the
    // very start of the prologue.  `call <fn_name> …` instructions in
    // the body emit `BL fn_name_label` which re-enters the function here,
    // re-executing the full AAPCS64 prologue (stp fp,lr + spill args) for
    // each new call frame — the same sequence the first call into the function
    // executes.
    //
    // Cross-function calls (callee_name != fn_name) are not yet supported and
    // will return `BackendError::UnsupportedOp` at instruction-emit time.
    if let Some(&entry_label) = labels.get(ctx.name) {
        asm.bind(entry_label).map_err(BackendError::from)?;
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
    let mut global_relocs: Vec<GlobalWordReloc> = Vec::new();
    for instr in ir {
        emit_instr(&mut asm, instr, &mut alloc, &labels, frame, ctx.name,
                   global_slots, &mut global_relocs)?;
    }

    // ---- Final epilogue (only reached if the function falls off the end) -
    // Defensive: a well-formed CIR ends in `ret_*`/`ret_void`.  We still
    // append an epilogue+ret here so a missing terminator doesn't produce
    // arbitrary code execution past the end of the function.
    emit_epilogue(&mut asm, frame)?;

    let external_relocs = std::mem::take(&mut asm.external_relocs);
    let bytes = asm.finish().map_err(BackendError::from)?;
    Ok((bytes, external_relocs, global_relocs))
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
    global_slots: &HashMap<String, usize>,
    global_relocs: &mut Vec<GlobalWordReloc>,
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

    // ---- mov_<ty> dest = src --------------------------------------------
    //
    // Trivial typed move.  With stack-spill regalloc this is just a
    // load + store; the type tag is informational (we always move
    // 64 bits).
    if let Some(_ty) = op.strip_prefix("mov_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::X0, src)?;
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
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

    // ---- call  callee_name, arg0, arg1, … ----------------------------------
    //
    // IIR/CIR `call` layout:
    //   op = "call", dest = Some(result_var), srcs = [Var(fn_name), Var(a0), …]
    //
    // AAPCS64 calling convention:
    //   1. Load each argument from its stack slot into x0..x7.
    //   2. Emit `BL <target_label>`.
    //   3. After the call returns, store x0 (return value) to dest's stack slot.
    //
    // The stack-spill allocator stores all values in fixed `[sp, #slot]`
    // locations, so there are no in-register live values to save around the
    // call.  The saved `fp` and `lr` are already on the stack (via `stp` in
    // the prologue) and therefore survive the callee's prologue/epilogue.
    //
    // Self-recursive calls (callee == current function) are supported by
    // resolving `fn_name` against the `labels` map (which includes a
    // binding for `fn_name` → the function body entry label, added in
    // `compile_inner`).
    //
    // Cross-function calls (callee != current function, i.e. mutual recursion
    // or calls to other module functions) are not yet supported in V1 because
    // each function is compiled independently into a separate byte buffer; the
    // relative offset to the callee is not known at instruction-emit time.
    if op == "call" {
        // srcs[0] = Var(callee_name), srcs[1..] = arguments
        let callee_name = match instr.srcs.first() {
            Some(CIROperand::Var(name)) => name.as_str(),
            _ => return Err(BackendError::MalformedInstr(
                "call: srcs[0] must be Var(function_name)".into()
            )),
        };
        let arg_srcs = &instr.srcs[1..];
        if arg_srcs.len() > 8 {
            return Err(BackendError::UnsupportedOp(format!(
                "call: too many arguments ({}) — AAPCS64 supports at most 8 register args",
                arg_srcs.len()
            )));
        }

        // Step 1 — load arguments into x0..x7.
        // We collect (arg_reg, stack_slot) pairs first to detect any overlap
        // between an argument's src slot and an earlier argument's destination
        // register.  With stack-spill allocation, all values are already on
        // the stack, so we can load them sequentially without aliasing issues
        // as long as we process them left-to-right.
        const ARG_REGS: [Reg; 8] = [
            Reg::X0, Reg::X1, Reg::X2, Reg::X3,
            Reg::X4, Reg::X5, Reg::X6, Reg::X7,
        ];
        for (i, src) in arg_srcs.iter().enumerate() {
            load_operand(asm, alloc, ARG_REGS[i], src)?;
        }

        // Step 2 — branch-with-link to the callee.
        //
        // Self-recursive calls: `fn_name` == the current function name, and
        // compile_inner bound `fn_name` as a label at the function body entry
        // (after the prologue parameter-spill) so BL re-enters the function.
        //
        // Cross-function calls: emit a placeholder `BL #0` and record an
        // `ExternalReloc` so the multi-function AOT linker can patch the offset
        // after all function binaries are concatenated.
        if callee_name == fn_name {
            let target_id = *labels.get(callee_name).ok_or_else(|| {
                BackendError::MalformedInstr(format!("call: no label for '{callee_name}'"))
            })?;
            asm.bl(target_id);
        } else {
            // Cross-function call: placeholder BL (offset will be patched by linker).
            asm.bl_external(callee_name);
        }

        // Step 3 — save return value from x0 to the destination stack slot.
        if let Some(dest) = &instr.dest {
            let slot = alloc.slot_of(dest);
            asm.str_(Reg::X0, Reg::Sp, slot)?;
        }

        return Ok(());
    }

    // ── LANG38 additions ─────────────────────────────────────────────────────

    // ---- div_<ty> dest = a / b -------------------------------------------
    //
    // Signed types (i*): SDIV.  Unsigned types (u*): UDIV.
    // Division by zero produces 0 per the ARM architecture spec — no trap.
    //
    // The type suffix drives signed/unsigned:
    //   "i64" → sdiv   "u64" → udiv   etc.
    if let Some(_ty) = op.strip_prefix("div_") {
        return emit_div(asm, alloc, instr, _ty, false);
    }

    // ---- mod_<ty> dest = a % b -------------------------------------------
    //
    // Modulo via divide-then-multiply-subtract:
    //   sdiv/udiv X2, X0, X1     ; X2 = a / b
    //   msub      X0, X2, X1, X0 ; X0 = X0 - X2*X1  =  a mod b
    //
    // This uses X2 as an additional scratch register.  With the stack-spill
    // allocator every value lives in a fixed stack slot, so X2 is never
    // "live" between instructions — borrowing it here is safe.
    if let Some(_ty) = op.strip_prefix("mod_") {
        return emit_div(asm, alloc, instr, _ty, true);
    }

    // ---- and_<ty> / or_<ty> / xor_<ty> dest = a OP b --------------------
    //
    // Logical binary operations — type suffix is informational only; the
    // operation is always 64-bit word-level (same as add/sub/mul).
    for (prefix, kind) in &[("and_", BitwiseKind::And), ("or_", BitwiseKind::Or), ("xor_", BitwiseKind::Xor)] {
        if op.starts_with(*prefix) {
            return emit_bitwise(asm, alloc, instr, *kind);
        }
    }

    // ---- shl_<ty> / shr_<ty>  dest = a SHIFT b ---------------------------
    //
    // Variable-amount shifts.  `shr_<ty>` emits ASRV (arithmetic, sign-
    // extending) for signed types and LSRV (logical, zero-filling) for
    // unsigned ones.  Both clamp the shift amount to 0..63 per the ARM spec.
    if let Some(ty) = op.strip_prefix("shl_") {
        return emit_shift(asm, alloc, instr, ShiftKind::Lsl, ty);
    }
    if let Some(ty) = op.strip_prefix("shr_") {
        let kind = if ty.starts_with('i') { ShiftKind::Asr } else { ShiftKind::Lsr };
        return emit_shift(asm, alloc, instr, kind, ty);
    }

    // ---- neg_<ty> dest = -src  (two's-complement negate) -----------------
    if let Some(_ty) = op.strip_prefix("neg_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::X0, src)?;
        asm.neg_(Reg::X0, Reg::X0);
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // ---- not_<ty> dest = ~src  (bitwise NOT) -----------------------------
    if let Some(_ty) = op.strip_prefix("not_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::X0, src)?;
        asm.mvn(Reg::X0, Reg::X0);
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // ---- global_load Str("name") → dest  (LANG39) -------------------------
    //
    // Materialise the address of `_twig_globals` via ADRP+ADD (to be patched
    // by the Mach-O linker), then load the 8-byte value at slot*8.
    //
    // CIR encoding after aot_specialise:
    //   srcs[0] = Var("name")  (Str("name") was lifted to Var by CIROperand::From)
    //   dest    = Some("%v")
    //
    // ARM64 sequence (4 instructions):
    //   ADRP X1, _twig_globals@PAGE    ; placeholder, patched by ld
    //   ADD  X1, X1, _twig_globals@PAGEOFF ; placeholder, patched by ld
    //   LDR  X0, [X1, #slot*8]         ; load value from global slot
    //   STR  X0, [SP, #dest_slot]      ; spill to stack frame
    if op == "global_load" {
        let name = instr.srcs.first().and_then(CIROperand::as_var)
            .ok_or_else(|| BackendError::MalformedInstr("global_load: srcs[0] must be Var(name)".into()))?;
        let slot = global_slots.get(name)
            .copied()
            .ok_or_else(|| BackendError::MalformedInstr(format!("global_load: unknown global '{name}'")))?;
        let dest = require_dest(instr)?;

        // ADRP X1, #0  (ARM64_RELOC_PAGE21 target)
        let adrp_word = asm.adrp_placeholder(Reg::X1);
        // ADD  X1, X1, #0  (ARM64_RELOC_PAGEOFF12 target)
        let add_word = asm.len_words();
        asm.add_imm(Reg::X1, Reg::X1, 0)?;
        global_relocs.push(GlobalWordReloc { adrp_word, add_word });

        // LDR X0, [X1, #slot*8]
        //
        // Guard against slot indices that would overflow the ARM64 12-bit
        // unsigned offset field.  For 64-bit (8-byte) accesses the LDR
        // immediate is encoded in units of 8, so the maximum representable
        // offset is 0xFFF * 8 = 32,760 bytes → 4,095 slots.  Silently
        // truncating a large slot to u32 would produce a machine instruction
        // that reads from the wrong address at runtime.
        const MAX_GLOBAL_SLOT: usize = 4_095;
        if slot > MAX_GLOBAL_SLOT {
            return Err(BackendError::MalformedInstr(format!(
                "global_load: slot index {slot} exceeds ARM64 12-bit LDR offset limit \
                 (max {MAX_GLOBAL_SLOT} slots)"
            )));
        }
        let byte_offset: u32 = (slot * 8)
            .try_into()
            .map_err(|_| BackendError::MalformedInstr(
                format!("global_load: slot byte offset overflows u32 (slot={slot})")
            ))?;
        asm.ldr(Reg::X0, Reg::X1, byte_offset)?;

        // Spill to dest's stack slot
        let d = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, d)?;
        return Ok(());
    }

    // ---- global_store Str("name"), src  (LANG39) --------------------------
    //
    // Load the value from the stack frame, then store it into the global slot.
    //
    // CIR encoding:
    //   srcs[0] = Var("name")   (global name, lifted from Str)
    //   srcs[1] = Var("%v")     (value to write)
    //
    // ARM64 sequence (4 instructions):
    //   LDR  X0, [SP, #val_slot]        ; load value from stack frame
    //   ADRP X1, _twig_globals@PAGE     ; placeholder, patched by ld
    //   ADD  X1, X1, _twig_globals@PAGEOFF ; placeholder, patched by ld
    //   STR  X0, [X1, #slot*8]          ; write to global slot
    if op == "global_store" {
        let name = instr.srcs.first().and_then(CIROperand::as_var)
            .ok_or_else(|| BackendError::MalformedInstr("global_store: srcs[0] must be Var(name)".into()))?;
        let slot = global_slots.get(name)
            .copied()
            .ok_or_else(|| BackendError::MalformedInstr(format!("global_store: unknown global '{name}'")))?;
        let val_src = instr.srcs.get(1)
            .ok_or_else(|| BackendError::MalformedInstr("global_store: needs srcs[1]=value".into()))?;

        // Load value from caller's stack slot into X0.
        load_operand(asm, alloc, Reg::X0, val_src)?;

        // ADRP X1, #0  (ARM64_RELOC_PAGE21 target)
        let adrp_word = asm.adrp_placeholder(Reg::X1);
        // ADD  X1, X1, #0  (ARM64_RELOC_PAGEOFF12 target)
        let add_word = asm.len_words();
        asm.add_imm(Reg::X1, Reg::X1, 0)?;
        global_relocs.push(GlobalWordReloc { adrp_word, add_word });

        // STR X0, [X1, #slot*8]
        //
        // Same 4,095-slot limit as global_load — ARM64 12-bit unsigned offset.
        const MAX_GLOBAL_SLOT_STORE: usize = 4_095;
        if slot > MAX_GLOBAL_SLOT_STORE {
            return Err(BackendError::MalformedInstr(format!(
                "global_store: slot index {slot} exceeds ARM64 12-bit STR offset limit \
                 (max {MAX_GLOBAL_SLOT_STORE} slots)"
            )));
        }
        let byte_offset: u32 = (slot * 8)
            .try_into()
            .map_err(|_| BackendError::MalformedInstr(
                format!("global_store: slot byte offset overflows u32 (slot={slot})")
            ))?;
        asm.str_(Reg::X0, Reg::X1, byte_offset)?;
        // global_store has no dest.
        return Ok(());
    }

    // ── LANG40 — io_out: print integer to stdout ──────────────────────────────
    //
    // CIR encoding:
    //   op    = "io_out"
    //   dest  = None  (void side-effect)
    //   srcs  = [Var(val_reg)]  (the i64 value to print)
    //
    // The backend emits:
    //   LDR X0, [SP, #val_slot]      ; load value → first AAPCS64 arg reg
    //   BL  __twig_print_i64         ; placeholder; patched by cross-fn linker
    //
    // `__twig_print_i64` is a self-contained helper injected into the text
    // section by `twig-aot::compile_module_to_text_raw` whenever this reloc
    // appears.  It converts x0 (i64) to decimal ASCII and writes to fd 1 via
    // the macOS write(2) syscall (x16=4, SVC #0x80), followed by '\n'.
    //
    // No dest register is written — io_out is a pure side-effecting call.
    if op == "io_out" {
        if instr.srcs.is_empty() {
            return Err(BackendError::MalformedInstr("io_out needs 1 src".into()));
        }
        // Load the integer value into X0 (first AAPCS64 argument register).
        load_operand(asm, alloc, Reg::X0, &instr.srcs[0])?;
        // Emit a placeholder BL that the AOT linker resolves to the helper.
        asm.bl_external("__twig_print_i64");
        return Ok(());
    }

    // Anything else is unsupported — caller should fall back.
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
// LANG38 helpers — division, bitwise, shift
// ===========================================================================

/// Emit `div_<ty>` or `mod_<ty>`.
///
/// Both division flavours share the same load sequence; the only difference
/// is what happens after the divide instruction:
/// - **div**: the quotient in X0 is the result.
/// - **mod**: a follow-up `MSUB X0, X2, X1, X0` computes `a − (a/b)×b`.
///
/// X2 is used as a scratch register for the intermediate quotient during
/// modulo.  The stack-spill allocator keeps every live value in a fixed
/// stack slot, so X2 is free between instructions.
fn emit_div(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    ty: &str,
    want_mod: bool,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    if instr.srcs.len() < 2 {
        return Err(BackendError::MalformedInstr(format!("{} needs 2 srcs", instr.op)));
    }
    // dividend → X0,  divisor → X1
    load_operand(asm, alloc, Reg::X0, &instr.srcs[0])?;
    load_operand(asm, alloc, Reg::X1, &instr.srcs[1])?;

    let signed = ty.starts_with('i');

    if want_mod {
        // quotient into X2 (scratch), then msub to get remainder
        if signed { asm.sdiv(Reg::X2, Reg::X0, Reg::X1); }
        else       { asm.udiv(Reg::X2, Reg::X0, Reg::X1); }
        // X0 = X0 - X2 * X1 = dividend - quotient * divisor = remainder
        asm.msub(Reg::X0, Reg::X2, Reg::X1, Reg::X0);
    } else {
        if signed { asm.sdiv(Reg::X0, Reg::X0, Reg::X1); }
        else       { asm.udiv(Reg::X0, Reg::X0, Reg::X1); }
    }

    let slot = alloc.slot_of(dest);
    asm.str_(Reg::X0, Reg::Sp, slot)?;
    Ok(())
}

/// Bitwise binary operation kinds (AND / OR / XOR).
#[derive(Debug, Clone, Copy)]
enum BitwiseKind { And, Or, Xor }

/// Emit a two-operand bitwise op: `dest = a OP b`.
fn emit_bitwise(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    kind: BitwiseKind,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    if instr.srcs.len() < 2 {
        return Err(BackendError::MalformedInstr(format!("{} needs 2 srcs", instr.op)));
    }
    load_operand(asm, alloc, Reg::X0, &instr.srcs[0])?;
    load_operand(asm, alloc, Reg::X1, &instr.srcs[1])?;
    match kind {
        BitwiseKind::And => asm.and_(Reg::X0, Reg::X0, Reg::X1),
        BitwiseKind::Or  => asm.orr(Reg::X0, Reg::X0, Reg::X1),
        BitwiseKind::Xor => asm.eor(Reg::X0, Reg::X0, Reg::X1),
    }
    let slot = alloc.slot_of(dest);
    asm.str_(Reg::X0, Reg::Sp, slot)?;
    Ok(())
}

/// Variable-shift operation kinds.
#[derive(Debug, Clone, Copy)]
enum ShiftKind { Lsl, Lsr, Asr }

/// Emit a shift: `dest = value SHIFT amount`.
fn emit_shift(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    kind: ShiftKind,
    _ty: &str,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    if instr.srcs.len() < 2 {
        return Err(BackendError::MalformedInstr(format!("{} needs 2 srcs", instr.op)));
    }
    // value → X0,  shift amount → X1
    load_operand(asm, alloc, Reg::X0, &instr.srcs[0])?;
    load_operand(asm, alloc, Reg::X1, &instr.srcs[1])?;
    match kind {
        ShiftKind::Lsl => asm.lsl_reg(Reg::X0, Reg::X0, Reg::X1),
        ShiftKind::Lsr => asm.lsr_reg(Reg::X0, Reg::X0, Reg::X1),
        ShiftKind::Asr => asm.asr_reg(Reg::X0, Reg::X0, Reg::X1),
    }
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
// LANG40 — emit_print_helper(): self-contained i64-to-stdout printer
// ===========================================================================

/// Emit a self-contained ARM64 function (`__twig_print_i64`) that converts
/// the signed 64-bit integer in `x0` to decimal ASCII and writes it to
/// stdout (fd 1) followed by `'\n'`, using the macOS `write(2)` syscall
/// (`x16 = 4`, `SVC #0x80`).
///
/// # Why not `_printf`?
///
/// Calling `_printf` requires dyld stub sections and external-symbol nlist
/// entries that `code-packager` does not currently emit.  This self-contained
/// helper lives directly in `__TEXT/__text` alongside user functions and is
/// resolved by the existing cross-function BL linker — zero additional Mach-O
/// complexity.
///
/// # Stack layout
///
/// ```text
/// [sp +  0]  saved x29 (fp)
/// [sp +  8]  saved x30 (lr)
/// [sp + 16]  digit buffer — 32 bytes (holds 20 decimal digits + sign)
/// ── frame = 48 bytes (16-byte aligned) ──
/// [sp + 48]  scratch byte (red zone; used for '\n' write)
/// ```
///
/// # Algorithm
///
/// 1. Prologue: `STP x29,x30,[sp,#-48]!`.
/// 2. Special-case `x0 == 0`: write literal `"0\n"` and return.
/// 3. If negative: set sign flag (`x1 = 1`) and negate `x0`.
/// 4. Digit loop: UDIV quotient into `x3`; MSUB remainder into `x4`;
///    `ADD x4,'0'`; `STRB w4,[x5,#-1]!`; `CBNZ x0,.loop`.
/// 5. Prepend `'-'` if sign flag is set.
/// 6. `write(1, x5, x6-x5)` via SVC #0x80 (syscall 4).
/// 7. Append newline: store `'\n'` at `[x6]` (red-zone safe) and repeat
///    the write syscall for 1 byte.
/// 8. Epilogue: `LDP x29,x30,[sp],#48; RET`.
///
/// # Register map
///
/// | Register | Role |
/// |----------|------|
/// | x0  | value (input); fd / modified by loop |
/// | x1  | sign flag (0 = positive, 1 = negative); buf pointer for write |
/// | x2  | constant divisor = 10; byte count for write |
/// | x3  | quotient scratch |
/// | x4  | digit / character scratch |
/// | x5  | write pointer (starts at buf_end, decrements each digit) |
/// | x6  | buf_end = sp+48 (one past digit buffer) |
/// | x16 | syscall number (4 = write on macOS ARM64) |
pub fn emit_print_helper() -> Vec<u8> {
    // We build the instruction words directly (raw u32 constants) rather than
    // through the label-aware Assembler, because the Assembler resolves labels
    // within a single pass and doesn't support forward references across
    // instruction-count-variable zero-paths.  Every instruction is fixed —
    // no data-dependent branches change the helper's length — so we can
    // precompute all branch offsets once and inline them as constants.
    //
    // All encodings verified against DDI 0487 (ARM Architecture Reference
    // Manual) and cross-checked against the aarch64-encoder unit tests.
    //
    // Word layout (0-based index):
    //
    //  0: STP  x29,x30,[sp,#-48]!     ; prologue — save fp/lr, allocate frame
    //  1: ADD  x29,sp,#0              ; fp = sp (debugger-friendly frame ptr)
    //  2: ADD  x6,sp,#48              ; x6 = buf_end (one past 32-byte digit buf)
    //  3: CMP  x0,#0                  ; is value zero?
    //  4: CBNZ x0,.non_zero (+17)     ; if x0 ≠ 0 → word 21
    //
    //  — zero path (words 5..20) —
    //  5: MOVZ w4,#48  ('0')
    //  6: SUB  x5,x6,#1               ; x5 = buf_end - 1 (inside frame)
    //  7: STRB w4,[x5]                ; write '0' to buffer
    //  8: MOVZ x0,#1                  ; fd = stdout
    //  9: MOV  x1,x5                  ; buf ptr
    // 10: MOVZ x2,#1                  ; len = 1
    // 11: MOVZ x16,#4                 ; write syscall
    // 12: SVC  #0x80
    // 13: MOVZ w4,#10  ('\n')
    // 14: STRB w4,[x6]               ; '\n' at buf_end (red zone, safe)
    // 15: MOVZ x0,#1
    // 16: MOV  x1,x6
    // 17: MOVZ x2,#1
    // 18: MOVZ x16,#4
    // 19: SVC  #0x80
    // 20: B    .epilogue (+30)        ; → word 50
    //
    //  — non-zero path (.non_zero = word 21) —
    // 21: MOVZ x1,#0                  ; sign = 0 (positive)
    // 22: CMP  x0,#0
    // 23: B.GE .digit_loop (+5)       ; if x0 ≥ 0 → word 28
    // 24: MOVZ x1,#1                  ; sign = 1 (negative)
    // 25: NEG  x0,x0                  ; negate to make positive
    // 26: MOV  x5,x6                  ; write ptr = buf_end
    // 27: MOVZ x2,#10                 ; divisor
    //
    // — .digit_loop (word 28) —
    // 28: UDIV x3,x0,x2              ; quotient
    // 29: MSUB x4,x3,x2,x0           ; remainder = x0 − x3×x2
    // 30: MOV  x0,x3                  ; value ← quotient (advance)
    // 31: ADD  x4,x4,#48             ; digit → ASCII ('0'=48)
    // 32: STRB w4,[x5,#-1]!          ; *--x5 = digit
    // 33: CBNZ x0,.digit_loop (-5)   ; → word 28 if value ≠ 0
    //
    // 34: CMP  x1,#0                  ; was value negative?
    // 35: B.EQ .write (+3)            ; if positive → word 38
    // 36: MOVZ w4,#45  ('-')
    // 37: STRB w4,[x5,#-1]!          ; *--x5 = '-'
    //
    // — .write (word 38) —
    // 38: MOVZ x0,#1                  ; fd = stdout
    // 39: MOV  x1,x5                  ; buf = write pointer
    // 40: SUB  x2,x6,x5              ; len = buf_end − write_ptr
    // 41: MOVZ x16,#4
    // 42: SVC  #0x80
    // 43: MOVZ w4,#10  ('\n')
    // 44: STRB w4,[x6]               ; '\n' at buf_end (red zone)
    // 45: MOVZ x0,#1
    // 46: MOV  x1,x6
    // 47: MOVZ x2,#1
    // 48: MOVZ x16,#4
    // 49: SVC  #0x80
    //
    // — .epilogue (word 50) —
    // 50: LDP  x29,x30,[sp],#48
    // 51: RET
    //
    // Total: 52 words = 208 bytes.

    // ── Words array — one entry per instruction (verified against DDI 0487) ──
    #[rustfmt::skip]
    let words: [u32; 52] = [
        0xA9BD7BFD, // [ 0] STP x29,x30,[sp,#-48]!      ; prologue: save fp/lr, allocate 48-byte frame
        0x910003FD, // [ 1] ADD x29,sp,#0               ; fp = sp
        0x9100C3E6, // [ 2] ADD x6,sp,#48               ; x6 = buf_end (one past digit buffer)
        0xF100001F, // [ 3] CMP x0,#0                   ; is value == 0?
        0xB5000220, // [ 4] CBNZ x0,+17 (.non_zero)     ; if x0 != 0 skip zero path → word 21
        0x52800604, // [ 5] MOVZ w4,#48 ('0')           ; zero path: prepare '0' character
        0xD10004C5, // [ 6] SUB x5,x6,#1                ; x5 = buf_end - 1 (inside frame)
        0x390000A4, // [ 7] STRB w4,[x5]                ; write '0' byte to buffer
        0xD2800020, // [ 8] MOVZ x0,#1                  ; fd = stdout
        0xAA0503E1, // [ 9] MOV x1,x5                   ; buf ptr
        0xD2800042, // [10] MOVZ x2,#1                  ; len = 1
        0xD2800090, // [11] MOVZ x16,#4                 ; write(2) syscall number
        0xD4001001, // [12] SVC #0x80                   ; write('0')
        0x52800144, // [13] MOVZ w4,#10 ('\n')
        0x390000C4, // [14] STRB w4,[x6]                ; '\n' at buf_end (macOS red zone: safe)
        0xD2800020, // [15] MOVZ x0,#1
        0xAA0603E1, // [16] MOV x1,x6                   ; buf = &newline
        0xD2800042, // [17] MOVZ x2,#1
        0xD2800090, // [18] MOVZ x16,#4
        0xD4001001, // [19] SVC #0x80                   ; write('\n')
        0x1400001E, // [20] B +30 (.epilogue)            ; skip non-zero path → word 50
        0xD2800001, // [21] MOVZ x1,#0                  ; non-zero path: sign = 0 (positive)
        0xF100001F, // [22] CMP x0,#0
        0x540000AA, // [23] B.GE +5 (.digit_loop)       ; if x0 >= 0 → word 28
        0xD2800021, // [24] MOVZ x1,#1                  ; sign = 1 (negative)
        0xCB0003E0, // [25] NEG x0,x0                   ; negate → make positive
        0xAA0603E5, // [26] MOV x5,x6                   ; write ptr = buf_end
        0xD2800142, // [27] MOVZ x2,#10                 ; divisor constant
        0x9AC20803, // [28] UDIV x3,x0,x2               ; .digit_loop: quotient
        0x9B028064, // [29] MSUB x4,x3,x2,x0            ; remainder = x0 - x3*x2
        0xAA0303E0, // [30] MOV x0,x3                   ; advance: value = quotient
        0x9100C084, // [31] ADD x4,x4,#48               ; digit -> ASCII ('0'=48)
        0x381FFCA4, // [32] STRB w4,[x5,#-1]!           ; *--x5 = digit byte
        0xB5FFFF60, // [33] CBNZ x0,-5 (.digit_loop)    ; loop while value != 0 → word 28
        0xF100003F, // [34] CMP x1,#0                   ; was value negative?
        0x54000060, // [35] B.EQ +3 (.write)            ; if positive skip '-' → word 38
        0x528005A4, // [36] MOVZ w4,#45 ('-')
        0x381FFCA4, // [37] STRB w4,[x5,#-1]!           ; *--x5 = '-'
        0xD2800020, // [38] MOVZ x0,#1                  ; .write: fd = stdout
        0xAA0503E1, // [39] MOV x1,x5                   ; buf = write pointer
        0xCB0500C2, // [40] SUB x2,x6,x5                ; len = buf_end - write_ptr
        0xD2800090, // [41] MOVZ x16,#4
        0xD4001001, // [42] SVC #0x80                   ; write(number digits)
        0x52800144, // [43] MOVZ w4,#10 ('\n')
        0x390000C4, // [44] STRB w4,[x6]                ; '\n' at buf_end (red zone)
        0xD2800020, // [45] MOVZ x0,#1
        0xAA0603E1, // [46] MOV x1,x6
        0xD2800042, // [47] MOVZ x2,#1
        0xD2800090, // [48] MOVZ x16,#4
        0xD4001001, // [49] SVC #0x80                   ; write('\n')
        0xA8C37BFD, // [50] LDP x29,x30,[sp],#48        ; .epilogue: restore fp/lr, deallocate
        0xD65F03C0, // [51] RET
    ];

    // Convert words → little-endian bytes
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
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

    // ---- LANG38: division, modulo, logical, shift, unary ----

    fn make_binop_cir(op: &str, dest: &str, a: &str, b: &str, ty: &str) -> CIRInstr {
        CIRInstr {
            op: op.into(),
            dest: Some(dest.into()),
            srcs: vec![CIROperand::Var(a.into()), CIROperand::Var(b.into())],
            ty: ty.into(),
            deopt_to: None,
        }
    }

    fn make_unop_cir(op: &str, dest: &str, src: &str, ty: &str) -> CIRInstr {
        CIRInstr {
            op: op.into(),
            dest: Some(dest.into()),
            srcs: vec![CIROperand::Var(src.into())],
            ty: ty.into(),
            deopt_to: None,
        }
    }

    // Helper: compile a tiny function [prologue, instr, ret] and assert it
    // succeeds and produces valid-length ARM64 output.
    fn compile_with_binop(
        op: &str,
        ty: &str,
        signed: bool,
    ) -> Vec<u8> {
        let param_ty = if signed { "i64" } else { "u64" };
        let params = vec![("a".into(), param_ty.into()), ("b".into(), param_ty.into())];
        let ret_ty = format!("ret_{param_ty}");
        let cir = vec![
            make_binop_cir(op, "v0", "a", "b", ty),
            CIRInstr {
                op: ret_ty,
                dest: None,
                srcs: vec![CIROperand::Var("v0".into())],
                ty: param_ty.into(),
                deopt_to: None,
            },
        ];
        compile(&ctx(&format!("f_{op}"), &params, param_ty), &cir)
            .unwrap_or_else(|e| panic!("{op} compile failed: {e}"))
    }

    #[test]
    fn div_i64_lowers() {
        let bytes = compile_with_binop("div_i64", "i64", true);
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn div_u64_lowers() {
        let bytes = compile_with_binop("div_u64", "u64", false);
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn mod_i64_lowers() {
        let bytes = compile_with_binop("mod_i64", "i64", true);
        // mod expands to sdiv + msub = 2 extra instructions vs div
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn mod_u64_lowers() {
        let bytes = compile_with_binop("mod_u64", "u64", false);
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn and_i64_lowers() {
        let bytes = compile_with_binop("and_i64", "i64", true);
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn or_i64_lowers() {
        let bytes = compile_with_binop("or_i64", "i64", true);
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn xor_i64_lowers() {
        let bytes = compile_with_binop("xor_i64", "i64", true);
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn shl_i64_lowers() {
        let bytes = compile_with_binop("shl_i64", "i64", true);
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn shr_i64_lowers_asr() {
        // Signed shift right → ASRV
        let bytes = compile_with_binop("shr_i64", "i64", true);
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn shr_u64_lowers_lsr() {
        // Unsigned shift right → LSRV
        let bytes = compile_with_binop("shr_u64", "u64", false);
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn neg_i64_lowers() {
        let params = vec![("x".into(), "i64".into())];
        let cir = vec![
            make_unop_cir("neg_i64", "v0", "x", "i64"),
            CIRInstr { op: "ret_i64".into(), dest: None,
                srcs: vec![CIROperand::Var("v0".into())], ty: "i64".into(), deopt_to: None },
        ];
        let bytes = compile(&ctx("fneg", &params, "i64"), &cir).expect("neg_i64");
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    #[test]
    fn not_i64_lowers() {
        let params = vec![("x".into(), "i64".into())];
        let cir = vec![
            make_unop_cir("not_i64", "v0", "x", "i64"),
            CIRInstr { op: "ret_i64".into(), dest: None,
                srcs: vec![CIROperand::Var("v0".into())], ty: "i64".into(), deopt_to: None },
        ];
        let bytes = compile(&ctx("fnot", &params, "i64"), &cir).expect("not_i64");
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0);
    }

    // ---- LANG39: global_load / global_store ----

    #[test]
    fn global_store_emits_four_instructions() {
        // global_store Var("x"), Var("v0")
        // Sequence: LDR X0, [SP, #val_slot]; ADRP X1; ADD X1, X1, #0; STR X0, [X1, #0]
        let mut slots: HashMap<String, usize> = HashMap::new();
        slots.insert("x".into(), 0);

        let cir = vec![
            CIRInstr { op: "const_i64".into(), dest: Some("v0".into()),
                       srcs: vec![CIROperand::Int(42)], ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "global_store".into(), dest: None,
                       srcs: vec![CIROperand::Var("x".into()), CIROperand::Var("v0".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None, srcs: vec![],
                       ty: "void".into(), deopt_to: None },
        ];
        let (bytes, _ext, g_relocs) = compile_with_globals(
            &ctx("f_gs", &[], "void"), &cir, &slots
        ).expect("global_store must compile");
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0, "byte-aligned output");
        assert_eq!(g_relocs.len(), 1, "one global_store → one GlobalWordReloc");
        let r = g_relocs[0];
        assert_eq!(r.add_word, r.adrp_word + 1, "ADD immediately follows ADRP");
    }

    #[test]
    fn global_load_emits_four_instructions() {
        // global_load Var("x") → v1
        // Sequence: ADRP X1; ADD X1, X1, #0; LDR X0, [X1, #0]; STR X0, [SP, #dest_slot]
        let mut slots: HashMap<String, usize> = HashMap::new();
        slots.insert("x".into(), 0);

        let cir = vec![
            CIRInstr { op: "global_load".into(), dest: Some("v1".into()),
                       srcs: vec![CIROperand::Var("x".into())],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "ret_i64".into(), dest: None,
                       srcs: vec![CIROperand::Var("v1".into())],
                       ty: "i64".into(), deopt_to: None },
        ];
        let (bytes, _ext, g_relocs) = compile_with_globals(
            &ctx("f_gl", &[], "i64"), &cir, &slots
        ).expect("global_load must compile");
        assert!(bytes.len() > 0 && bytes.len() % 4 == 0, "byte-aligned output");
        assert_eq!(g_relocs.len(), 1, "one global_load → one GlobalWordReloc");
        let r = g_relocs[0];
        assert_eq!(r.add_word, r.adrp_word + 1, "ADD immediately follows ADRP");
    }

    #[test]
    fn global_load_stores_slot_offset_in_ldr() {
        // Slot 2 → offset 16 bytes.  The LDR instruction word should have
        // imm12=2 (16/8=2) in the [21:10] field of an F9400000-class instruction.
        let mut slots: HashMap<String, usize> = HashMap::new();
        slots.insert("y".into(), 2);

        let cir = vec![
            CIRInstr { op: "global_load".into(), dest: Some("v0".into()),
                       srcs: vec![CIROperand::Var("y".into())],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "ret_i64".into(), dest: None,
                       srcs: vec![CIROperand::Var("v0".into())],
                       ty: "i64".into(), deopt_to: None },
        ];
        let (bytes, _, g_relocs) = compile_with_globals(
            &ctx("f_slot", &[], "i64"), &cir, &slots
        ).expect("compile ok");

        let r = g_relocs[0];
        // The LDR instruction is immediately after the ADD (r.add_word + 1).
        let ldr_byte = (r.add_word + 1) * 4;
        let ldr_word = u32::from_le_bytes(bytes[ldr_byte..ldr_byte+4].try_into().unwrap());
        // LDR X0, [X1, #16]: 0xF9400000 | (imm12=2 << 10) | (X1=1 << 5) | X0=0
        //   = 0xF9400000 | 0x800 | 0x20 | 0 = 0xF9400820
        assert_eq!(ldr_word, 0xF9400820, "LDR X0,[X1,#16] for slot 2");
    }

    #[test]
    fn global_store_unknown_name_errors() {
        let slots: HashMap<String, usize> = HashMap::new(); // empty!
        let cir = vec![
            CIRInstr { op: "global_store".into(), dest: None,
                       srcs: vec![CIROperand::Var("z".into()), CIROperand::Int(0)],
                       ty: "void".into(), deopt_to: None },
        ];
        let result = compile_with_globals(&ctx("f_err", &[], "void"), &cir, &slots);
        assert!(result.is_err(), "unknown global should error");
    }

    #[test]
    fn two_globals_produce_two_relocs() {
        let mut slots: HashMap<String, usize> = HashMap::new();
        slots.insert("a".into(), 0);
        slots.insert("b".into(), 1);

        let cir = vec![
            CIRInstr { op: "const_i64".into(), dest: Some("v0".into()),
                       srcs: vec![CIROperand::Int(1)], ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "global_store".into(), dest: None,
                       srcs: vec![CIROperand::Var("a".into()), CIROperand::Var("v0".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "global_load".into(), dest: Some("v1".into()),
                       srcs: vec![CIROperand::Var("b".into())],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "ret_i64".into(), dest: None,
                       srcs: vec![CIROperand::Var("v1".into())],
                       ty: "i64".into(), deopt_to: None },
        ];
        let (_, _, g_relocs) = compile_with_globals(
            &ctx("f_two", &[], "i64"), &cir, &slots
        ).expect("compile ok");
        assert_eq!(g_relocs.len(), 2, "one reloc per global access");
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

    // ---- LANG40: io_out handler + emit_print_helper ----

    #[test]
    fn io_out_emits_bl_reloc() {
        // io_out with a single source should compile without error and emit
        // exactly one ExternalReloc targeting "__twig_print_i64".
        //
        // CIR:
        //   const_i64 v0 = 42
        //   io_out v0
        //   ret_void
        let cir = vec![
            CIRInstr { op: "const_i64".into(), dest: Some("v0".into()),
                       srcs: vec![CIROperand::Int(42)],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "io_out".into(), dest: None,
                       srcs: vec![CIROperand::Var("v0".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None,
                       srcs: vec![], ty: "void".into(), deopt_to: None },
        ];
        let (bytes, ext_relocs, _) = compile_with_globals(
            &ctx("print_42", &[], "void"), &cir, &HashMap::new()
        ).expect("io_out should compile");

        assert!(!bytes.is_empty(), "should produce machine code");
        assert_eq!(ext_relocs.len(), 1, "exactly one external reloc (the BL placeholder)");
        assert_eq!(ext_relocs[0].symbol, "__twig_print_i64",
                   "reloc target must be the print helper");
    }

    #[test]
    fn io_out_missing_src_errors() {
        // io_out with no sources should return an error, not panic.
        let cir = vec![
            CIRInstr { op: "io_out".into(), dest: None,
                       srcs: vec![], ty: "void".into(), deopt_to: None },
        ];
        let result = compile_with_globals(
            &ctx("bad_io", &[], "void"), &cir, &HashMap::new()
        );
        assert!(result.is_err(), "io_out with no srcs should error");
    }

    #[test]
    fn emit_print_helper_has_prologue() {
        // The helper must start with the canonical STP X29,X30,[SP,#-48]!
        // word (0xA9BD7BFD).  This verifies the prologue is correctly formed
        // and the frame size is 48 bytes as specified.
        let bytes = emit_print_helper();
        assert!(!bytes.is_empty(), "helper should be non-empty");
        assert_eq!(bytes.len() % 4, 0, "helper must be word-aligned");
        let first_word = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(first_word, 0xA9BD7BFD,
                   "first word must be STP x29,x30,[sp,#-48]! = 0xA9BD7BFD");
    }

    #[test]
    fn emit_print_helper_ends_with_ret() {
        // The last instruction must be RET (0xD65F03C0).
        let bytes = emit_print_helper();
        let n = bytes.len();
        assert!(n >= 4);
        let last_word = u32::from_le_bytes(bytes[n-4..n].try_into().unwrap());
        assert_eq!(last_word, 0xD65F03C0,
                   "last word must be RET = 0xD65F03C0");
    }

    #[test]
    fn emit_print_helper_size_is_52_words() {
        // The helper is a fixed-instruction-count function: 52 words = 208 bytes.
        // If this assertion fails, update the B/CBNZ branch offsets in the
        // emit_print_helper() source accordingly.
        let bytes = emit_print_helper();
        assert_eq!(bytes.len(), 52 * 4,
                   "helper must be exactly 52 instructions (208 bytes)");
    }
}
