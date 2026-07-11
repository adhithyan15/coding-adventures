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
//! | I/O | `io_out` (LANG40/LANG41 — BL `__twig_print_i64`; resolved by system linker from runtime archive) |
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
//! Every typed integer mnemonic computes on 64-bit ARM registers, then — for a
//! narrow **unsigned** type (`u4`/`u8`/`u16`/`u32`) — masks the result back to
//! its declared width with a follow-up `AND` (LANG-FULL E2, the native-AOT leg).
//! So `add_u8 200, 100` yields `44` (300 mod 256), `not_u8 0` yields `255`, and
//! `shl_u8 1, 8` yields `0`, matching the wrap semantics the other backends
//! (vm-core, jit-core, wasm, jvm, cil) already provide.  See [`mask_narrow_x0`].
//! Signed narrow types (`i8`/`i16`/`i32`) would need sign-extension rather than
//! a plain mask and are not emitted by any current frontend — left unmasked.

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
// LANG75 — runtime-helper signature table
// ===========================================================================
//
// `call_builtin "<name>", <args>` looks `name` up in this table.  Every
// V1 helper has a fixed signature (arg count + returns-a-value bit);
// the backend rejects mismatched call sites with `MalformedInstr`.
//
// Linker symbols are always prefixed `__twig_` — see runtime/twig_runtime.c.
//
// | Mnemonic       | C signature                                   | Returns |
// |---------------|-----------------------------------------------|---------|
// | `print_i64`   | `void __twig_print_i64(int64_t)`              | no      |
// | `putchar`     | `void __twig_putchar(int32_t c)`              | no      |
// | `getchar`     | `int32_t __twig_getchar(void)`                | yes     |
// | `print_string`| `void __twig_print_string(const char*, int64_t)` | no      |
// | `input_i64`   | `int64_t __twig_input_i64(void)`              | yes     |
// | `input_str`   | `int64_t __twig_input_str(void)` (E4-dyn str handle) | yes |
// | `exit`        | `void __twig_exit(int32_t)` (noreturn)        | no      |
// | `alloc_bytes` | `int64_t __twig_alloc_bytes(int64_t n)`       | yes     |
// | `lispy_cons`  | `uint64_t __dyn_cons(uint64_t, uint64_t)` | yes  |
// | `lispy_car`   | `uint64_t __dyn_car(uint64_t)`         | yes     |
// | `lispy_cdr`   | `uint64_t __dyn_cdr(uint64_t)`         | yes     |
// | `str_eq`      | `int64_t __twig_str_eq(int64_t, int64_t)`    | yes     |

#[derive(Debug, Clone, Copy)]
struct BuiltinSig {
    name: &'static str,
    n_args: usize,
    returns: bool,
}

const V1_BUILTINS: &[BuiltinSig] = &[
    BuiltinSig { name: "print_i64",    n_args: 1, returns: false },
    BuiltinSig { name: "putchar",      n_args: 1, returns: false },
    BuiltinSig { name: "getchar",      n_args: 0, returns: true  },
    BuiltinSig { name: "print_string", n_args: 2, returns: false },
    BuiltinSig { name: "input_i64",    n_args: 0, returns: true  },
    // E4-dyn: BASIC string `INPUT A$` — reads a whole line as a runtime string.
    // Same 0-arg / returns-i64 shape as `input_i64`; the returned i64 is the
    // handle (base address) of a `[i64 len][bytes]` heap block, carried in x0
    // like any other pointer-as-i64 (`alloc_bytes`/`str_eq`), so no new lowering.
    BuiltinSig { name: "input_str",    n_args: 0, returns: true  },
    BuiltinSig { name: "exit",         n_args: 1, returns: false },
    // LANG76 — heap allocator.  Returns a pointer (treated as i64).
    BuiltinSig { name: "alloc_bytes",  n_args: 1, returns: true  },
    // LANG77 — the shared lisp value runtime (McCarthy Lisp L3b-2b).  These
    // dispatch to `__dyn_*` in `twig-aot/runtime/lispy_runtime.c`,
    // which implements `lispy-runtime`'s NaN-box tagged-value model.  Each
    // takes/returns an opaque 64-bit `LispyValue`.  No backend-specific
    // logic — the generic `call_builtin` path marshals args + emits the BL.
    BuiltinSig { name: "lispy_cons",   n_args: 2, returns: true  },
    BuiltinSig { name: "lispy_car",    n_args: 1, returns: true  },
    BuiltinSig { name: "lispy_cdr",    n_args: 1, returns: true  },
    // LANG77 L3b-2c — unbox a tagged integer to a raw machine word at the
    // program-exit boundary.  `int64_t __dyn_unbox_int(uint64_t)`.
    BuiltinSig { name: "lispy_unbox_int", n_args: 1, returns: true },
    // LANG77 L3b-2c-2 — the ATOM/EQ predicates (return tagged #t/#f) and the
    // COND truthiness normaliser (returns a raw 0/1 for jmp_if_false).
    BuiltinSig { name: "lispy_pair_p",    n_args: 1, returns: true },
    BuiltinSig { name: "lispy_not",       n_args: 1, returns: true },
    BuiltinSig { name: "lispy_equal",     n_args: 2, returns: true },
    BuiltinSig { name: "lispy_truthy",    n_args: 1, returns: true },
    // LANG77 W13b — the universal program-exit coercion for a polymorphic
    // (lambda / `any`) result: dispatch on the runtime tag.
    // `int64_t __dyn_to_exit_code(uint64_t)`.
    BuiltinSig { name: "lispy_to_exit_code", n_args: 1, returns: true },
    // LANG-STR-RT — runtime string ops on LANG-STR-RT length-prefixed buffers.
    // Both operands are i64 pointers to `[int64_t len][char bytes...]` buffers.
    BuiltinSig { name: "str_eq", n_args: 2, returns: true },
    // E4-dyn runtime string concatenation.  `int64_t __twig_str_concat(int64_t a,
    // int64_t b)` reads both `[i64 len][bytes]` headers and returns a handle to a
    // fresh joined block.  Same 2-arg / returns-i64 shape as `str_eq` (both operand
    // handles ride x0/x1, the result handle rides x0), so the generic `call_builtin`
    // marshaller needs no new codegen — only this table entry.
    BuiltinSig { name: "str_concat", n_args: 2, returns: true },
    // TWIG-GC (native-aot-substrate PR-1) — GC-managed allocation and safepoint.
    // `gc_alloc(n)` returns a GC-tracked pointer (0 on OOM).
    // `gc_safepoint()` triggers a collection when the live set exceeds the
    // adaptive threshold.  Used by IIR `safepoint` lowering.
    BuiltinSig { name: "gc_alloc",     n_args: 1, returns: true  },
    BuiltinSig { name: "gc_safepoint", n_args: 0, returns: false },
];

fn lookup_builtin(name: &str) -> Option<BuiltinSig> {
    V1_BUILTINS.iter().copied().find(|s| s.name == name)
}

fn v1_builtin_names() -> Vec<&'static str> {
    V1_BUILTINS.iter().map(|s| s.name).collect()
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
    // `sub_imm` takes a 12-bit unsigned immediate (0..4095), so the split
    // prologue (see below) supports frames up to 4080 bytes (4080 = 4096
    // rounded down to 16-byte alignment, covering 508 variable slots).
    // Functions beyond that are out of scope for V1.
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
    // Small frames (≤ 504 bytes): `STP X29,X30,[SP,#-frame]!` — pre-indexed
    // combined allocate+save in one instruction.
    // Large frames (> 504 bytes): split into `SUB SP,SP,#frame` (12-bit imm,
    // up to 4080 bytes) followed by two `STR`s.  The variable slots ([SP,#16]
    // onward) are the same in both cases; only the fp/lr save differs.
    if frame <= 504 {
        asm.stp_pre(Reg::Fp, Reg::Lr, Reg::Sp, -(frame as i32))?;
    } else {
        asm.sub_imm(Reg::Sp, Reg::Sp, frame)?;
        asm.str_(Reg::Fp, Reg::Sp, 0)?; // [sp+0] = saved fp
        asm.str_(Reg::Lr, Reg::Sp, 8)?; // [sp+8] = saved lr
    }
    asm.add_imm(Reg::Fp, Reg::Sp, 0)?; // fp = sp (alias for mov fp,sp)

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
            // An `f64` (ALGOL `real`, LANG-FULL E3) lives in its 8-byte stack
            // slot as raw IEEE-754 bits — identical to an integer slot — so we
            // materialise the bit pattern in X0 and `str` it. No FP register is
            // needed to *load a constant*; only arithmetic/compare use the D
            // registers (see `emit_fp_binop`/`emit_fp_cmp`).
            CIROperand::Float(f) => f.to_bits(),
            CIROperand::Var(_)   => return Err(BackendError::MalformedInstr(format!("{op} needs literal source"))),
        };
        asm.mov_imm64(Reg::X0, imm);
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // ---- LANG-FULL E8: int ⇄ real conversions ---------------------------
    //
    // These reach the backend with their bare IIR names (the `specialise`
    // pass passes unrecognised ops through unchanged), so they are matched
    // here rather than via a typed `_<ty>` suffix.
    //
    //   int_to_real        ldr x0,[src]; scvtf d0,x0;            str_d d0,[dest]
    //   real_to_int_trunc  ldr_d d0,[src]; fcvtzs x0,d0;         str x0,[dest]
    //   real_to_int_floor  ldr_d d0,[src]; frintm d0,d0; fcvtzs x0,d0; str x0,[dest]
    //
    // `fcvtzs` rounds toward zero and *saturates* on NaN/±∞/out-of-range (ARM
    // does not trap) — a documented divergence from the VM trap shared with the
    // JVM backend; every finite, in-range value (all `entier`/coercion produces)
    // converts identically.
    if op == "int_to_real" {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr("int_to_real needs srcs[0]".into()))?;
        load_operand(asm, alloc, Reg::X0, src)?; // i64 → X0
        asm.scvtf(Reg::X0, Reg::X0);             // D0 = (double) X0
        let slot = alloc.slot_of(dest);
        asm.str_d(Reg::X0, Reg::Sp, slot)?;      // store the double
        return Ok(());
    }
    if op == "real_to_int_trunc" || op == "real_to_int_floor" {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_fp_operand(asm, alloc, Reg::X0, src)?; // f64 → D0
        if op == "real_to_int_floor" {
            asm.frintm(Reg::X0, Reg::X0);           // D0 = floor(D0)  (round to −∞)
        }
        asm.fcvtzs(Reg::X0, Reg::X0);               // X0 = (i64) D0   (trunc toward zero)
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;          // store the integer
        return Ok(());
    }
    // AL8 sqrt — `FSQRT Dd, Dn` (single hardware FP instruction, no libm).
    //   f64_sqrt dest <- src  →  ldr_d D0,[src]; fsqrt D0,D0; str_d D0,[dest]
    // NaN propagates; negative input → NaN (IEEE-754 / matches VM `f64::sqrt`).
    if op == "f64_sqrt" {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr("f64_sqrt needs srcs[0]".into()))?;
        load_fp_operand(asm, alloc, Reg::X0, src)?; // f64 → D0
        asm.fsqrt(Reg::X0, Reg::X0);               // D0 = sqrt(D0)
        let slot = alloc.slot_of(dest);
        asm.str_d(Reg::X0, Reg::Sp, slot)?;        // store the result as f64
        return Ok(());
    }
    // BA-pow — `BL pow` (libm two-argument power, no hardware opcode).
    //   f64_pow dest <- base, exp  →  ldr_d D0,[base]; ldr_d D1,[exp]; BL pow; str_d D0,[dest]
    // AAPCS64: first FP arg in D0, second in D1; result in D0.
    if op == "f64_pow" {
        let dest = require_dest(instr)?;
        let base = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr("f64_pow needs srcs[0]".into()))?;
        let exp_ = instr.srcs.get(1)
            .ok_or_else(|| BackendError::MalformedInstr("f64_pow needs srcs[1]".into()))?;
        load_fp_operand(asm, alloc, Reg::X0, base)?; // D0 = base
        load_fp_operand(asm, alloc, Reg::X1, exp_)?; // D1 = exp
        asm.bl_external("pow");                       // D0 = pow(D0, D1)
        let slot = alloc.slot_of(dest);
        asm.str_d(Reg::X0, Reg::Sp, slot)?;          // store the result as f64
        return Ok(());
    }

    // AL8 transcendentals — sin/cos/ln/exp via libm (AAPCS64: D0 → BL → D0).
    //
    // ALGOL 60 calls these `sin`, `cos`, `ln`, `exp` but libm uses `log` for
    // natural log.  The AAPCS64 FP calling convention passes the first f64
    // argument in D0 and returns the f64 result in D0, matching the ldr_d /
    // str_d pattern already used by f64_sqrt (but with a BL instead of fsqrt).
    //
    // libm is pre-linked on both macOS (`-lSystem` includes libm) and Linux
    // (`-lm`) — see twig-aot/src/lib.rs.  The BL placeholder is resolved by
    // the AOT cross-function linker at link time.
    if matches!(op, "f64_sin" | "f64_cos" | "f64_ln" | "f64_exp" | "f64_atan" | "f64_tan") {
        let libm_sym = match op {
            "f64_sin"  => "sin",
            "f64_cos"  => "cos",
            "f64_ln"   => "log",  // libm natural log is `log`, not `ln`
            "f64_exp"  => "exp",
            "f64_atan" => "atan",
            "f64_tan"  => "tan",
            _ => unreachable!(),
        };
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(
                format!("{op} needs srcs[0]")))?;
        load_fp_operand(asm, alloc, Reg::X0, src)?; // f64 argument → D0
        asm.bl_external(libm_sym);                   // BL sin/cos/log/exp
        let slot = alloc.slot_of(dest);
        asm.str_d(Reg::X0, Reg::Sp, slot)?;         // store f64 result
        return Ok(());
    }

    // ---- add/sub/mul (typed) --------------------------------------------
    for (prefix, kind) in &[("add_", BinKind::Add), ("sub_", BinKind::Sub), ("mul_", BinKind::Mul)] {
        if let Some(ty) = op.strip_prefix(*prefix) {
            return emit_binop(asm, alloc, instr, *kind, ty);
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
        if let Some(ty) = op.strip_prefix(*prefix) {
            return emit_bitwise(asm, alloc, instr, *kind, ty);
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
    if let Some(ty) = op.strip_prefix("neg_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::X0, src)?;
        asm.neg_(Reg::X0, Reg::X0);
        mask_narrow_x0(asm, ty); // E2: -x mod 2ⁿ for narrow widths
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // ---- not_<ty> dest = ~src  (bitwise NOT) -----------------------------
    if let Some(ty) = op.strip_prefix("not_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::X0, src)?;
        asm.mvn(Reg::X0, Reg::X0);
        mask_narrow_x0(asm, ty); // E2: ~x flips only the low n bits for a uⁿ
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

    // ── LANG75 — call_builtin "<name>", <args> ─────────────────────────────────
    //
    // Generic dispatch to runtime helpers.  Look `name` up in the V1
    // helper table, validate arg count, marshal args into x0..x7 per
    // AAPCS64, emit `BL __twig_<name>` (placeholder; linker patches).
    // If the helper returns, store x0 into the dest slot.
    //
    // `io_out v` is sugar for `call_builtin "print_i64", v` and stays
    // in the dispatch above for backwards compatibility with frontends
    // and existing tests that emit `io_out` directly.
    //
    // Unknown helper names → `BackendError::MalformedInstr` — the spec's
    // "BackendRefused" (soft refusal, not a panic).
    if op == "call_builtin" {
        let name = match instr.srcs.first() {
            Some(CIROperand::Var(s)) => s.as_str(),
            _ => return Err(BackendError::MalformedInstr(
                "call_builtin: srcs[0] must be Var(helper_name)".into(),
            )),
        };
        let sig = lookup_builtin(name).ok_or_else(|| {
            BackendError::MalformedInstr(format!(
                "call_builtin: unknown helper '{name}' (V1 table: {})",
                v1_builtin_names().join(", "),
            ))
        })?;
        let arg_srcs = &instr.srcs[1..];
        if arg_srcs.len() != sig.n_args {
            return Err(BackendError::MalformedInstr(format!(
                "call_builtin '{name}': expected {} arg(s), got {}",
                sig.n_args, arg_srcs.len(),
            )));
        }
        if sig.returns && instr.dest.is_none() {
            return Err(BackendError::MalformedInstr(format!(
                "call_builtin '{name}': returns a value but dest is None",
            )));
        }
        if !sig.returns && instr.dest.is_some() {
            return Err(BackendError::MalformedInstr(format!(
                "call_builtin '{name}': returns void but dest is Some",
            )));
        }
        // AAPCS64 supplies 8 GPR arg slots — all V1 helpers fit in ≤ 2.
        const ARG_REGS: [Reg; 8] = [
            Reg::X0, Reg::X1, Reg::X2, Reg::X3,
            Reg::X4, Reg::X5, Reg::X6, Reg::X7,
        ];
        for (i, src) in arg_srcs.iter().enumerate() {
            load_operand(asm, alloc, ARG_REGS[i], src)?;
        }
        let symbol = format!("__twig_{name}");
        asm.bl_external(&symbol);
        if sig.returns {
            if let Some(dest) = &instr.dest {
                let slot = alloc.slot_of(dest);
                asm.str_(Reg::X0, Reg::Sp, slot)?;
            }
        }
        return Ok(());
    }

    // ── LANG76 — byte memory ops + heap allocation ─────────────────────────────

    // `alloc_bytes <n> -> <dest>` — sugar for `call_builtin "alloc_bytes", n`.
    if op == "alloc_bytes" {
        let dest = require_dest(instr)?;
        let n_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("alloc_bytes: needs srcs[0]=byte_count".into())
        })?;
        load_operand(asm, alloc, Reg::X0, n_src)?;
        asm.bl_external("__twig_alloc_bytes");
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // `load_byte <ptr>, <offset> -> <dest>` — read one byte from
    // `[ptr + offset]`, zero-extend, store into `dest`.
    //
    // ARM64 sequence (X0/X1 are the standard scratch pair):
    //   ldr  x0, [sp, ptr_slot]
    //   ldr  x1, [sp, offset_slot]
    //   add  x0, x0, x1                  ; x0 = ptr + offset
    //   ldrb w0, [x0]                    ; load byte, zero-extend to w0
    //   str  x0, [sp, dest_slot]         ; LDRB zeros upper 32 bits of x0
    if op == "load_byte" {
        let dest = require_dest(instr)?;
        let ptr_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("load_byte: needs srcs[0]=ptr".into())
        })?;
        let off_src = instr.srcs.get(1).ok_or_else(|| {
            BackendError::MalformedInstr("load_byte: needs srcs[1]=offset".into())
        })?;
        load_operand(asm, alloc, Reg::X0, ptr_src)?;
        load_operand(asm, alloc, Reg::X1, off_src)?;
        asm.add(Reg::X0, Reg::X0, Reg::X1);
        asm.ldrb(Reg::X0, Reg::X0, 0)?;
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // `store_byte <ptr>, <offset>, <value>` — write the low 8 bits of
    // `value` to `[ptr + offset]`.  No dest.
    //
    // Sequence:
    //   ldr  x0, [sp, ptr_slot]
    //   ldr  x1, [sp, offset_slot]
    //   add  x0, x0, x1
    //   ldr  x2, [sp, value_slot]
    //   strb w2, [x0]                    ; strb writes only the low byte
    if op == "store_byte" {
        if instr.dest.is_some() {
            return Err(BackendError::MalformedInstr(
                "store_byte: must not have a dest".into(),
            ));
        }
        let ptr_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("store_byte: needs srcs[0]=ptr".into())
        })?;
        let off_src = instr.srcs.get(1).ok_or_else(|| {
            BackendError::MalformedInstr("store_byte: needs srcs[1]=offset".into())
        })?;
        let val_src = instr.srcs.get(2).ok_or_else(|| {
            BackendError::MalformedInstr("store_byte: needs srcs[2]=value".into())
        })?;
        load_operand(asm, alloc, Reg::X0, ptr_src)?;
        load_operand(asm, alloc, Reg::X1, off_src)?;
        asm.add(Reg::X0, Reg::X0, Reg::X1);
        load_operand(asm, alloc, Reg::X2, val_src)?;
        asm.strb(Reg::X2, Reg::X0, 0)?;
        return Ok(());
    }

    // ── LANG-FULL E5 — bounds-checked arrays (static, length-prefixed) ─────────
    //
    // An array is a single `__twig_alloc_bytes` block laid out as `[i64 length]
    // [elem 0][elem 1]…`; the IIR *handle* is the block base. The native target
    // has no managed runtime, so `array_get`/`array_set` emit an EXPLICIT
    // unsigned bounds compare and trap with `udf` — the aarch64 twin of the LLVM
    // `icmp uge`+`llvm.trap` and WASM `i64.ge_u`+`unreachable` lowerings.

    // `alloc_array <count> -> <dest>` (ty = `array<i64>`). Allocate `8 + count*8`
    // bytes via the shared runtime helper, store the length header, return base.
    //   x0 = count ; x0 = count<<3 ; x0 += 8 ; bl __twig_alloc_bytes ; [x0]=count
    if op == "alloc_array" {
        let dest = require_dest(instr)?;
        let count_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("alloc_array: needs srcs[0]=count".into())
        })?;
        // The AOT specialiser collapses the `array<T>` result type to `any` (it
        // is not in the scalar allowlist), so `instr.ty` no longer carries the
        // element type here. The native backend supports 8-byte elements
        // (`i64`/`u64`/`f64`), so the stride is a fixed 8 — the per-access ops
        // validate the element width.
        let elem_size: i32 = 8;
        load_operand(asm, alloc, Reg::X0, count_src)?;
        // total = count*8 + 8 → count<<3, then +8.
        asm.mov_imm64(Reg::X1, elem_size.trailing_zeros() as u64); // log2(8)=3
        asm.lsl_reg(Reg::X0, Reg::X0, Reg::X1);
        asm.add_imm(Reg::X0, Reg::X0, 8)?;
        asm.bl_external("__twig_alloc_bytes");
        // dest = base (x0); then write the length header [base+0] = count.
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        load_operand(asm, alloc, Reg::X1, count_src)?; // reload count (call clobbered regs)
        asm.str_(Reg::X1, Reg::X0, 0)?;
        return Ok(());
    }

    // `array_len <handle> -> <dest>` — load the i64 length header at `[base+0]`.
    if op == "array_len" {
        let dest = require_dest(instr)?;
        let h_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("array_len: needs srcs[0]=handle".into())
        })?;
        load_operand(asm, alloc, Reg::X0, h_src)?;
        asm.ldr(Reg::X0, Reg::X0, 0)?;
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // `array_get <handle>, <idx> -> <dest>` (ty = element). Bounds-check then load.
    //   x0=base ; x1=idx ; x2=[base] (len) ; cmp idx,len ; b.lo ok ; udf ; ok:
    //   x1 = idx<<3 ; x0 = base+x1 ; ldr x0,[x0,#8] ; str x0,[sp,dest]
    if op == "array_get" {
        let dest = require_dest(instr)?;
        let h_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("array_get: needs srcs[0]=handle".into())
        })?;
        let i_src = instr.srcs.get(1).ok_or_else(|| {
            BackendError::MalformedInstr("array_get: needs srcs[1]=idx".into())
        })?;
        let elem_size = native_array_elem_size(&instr.ty)?;
        load_operand(asm, alloc, Reg::X0, h_src)?;
        load_operand(asm, alloc, Reg::X1, i_src)?;
        asm.ldr(Reg::X2, Reg::X0, 0)?; // length
        asm.cmp(Reg::X1, Reg::X2); // idx - len
        let ok = asm.create_label();
        asm.b_cond(Cond::Lo, ok); // idx <u len → in bounds, skip trap
        asm.udf(0xDEAD);
        asm.bind(ok).map_err(BackendError::from)?;
        asm.mov_imm64(Reg::X3, elem_size.trailing_zeros() as u64);
        asm.lsl_reg(Reg::X1, Reg::X1, Reg::X3); // idx*elem_size
        asm.add(Reg::X0, Reg::X0, Reg::X1); // base + idx*size
        asm.ldr(Reg::X0, Reg::X0, 8)?; // element (past the 8-byte header)
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // `array_set <handle>, <idx>, <val>` (no dest, ty = element). Bounds-check, store.
    if op == "array_set" {
        if instr.dest.is_some() {
            return Err(BackendError::MalformedInstr("array_set: must not have a dest".into()));
        }
        let h_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("array_set: needs srcs[0]=handle".into())
        })?;
        let i_src = instr.srcs.get(1).ok_or_else(|| {
            BackendError::MalformedInstr("array_set: needs srcs[1]=idx".into())
        })?;
        let v_src = instr.srcs.get(2).ok_or_else(|| {
            BackendError::MalformedInstr("array_set: needs srcs[2]=val".into())
        })?;
        let elem_size = native_array_elem_size(&instr.ty)?;
        load_operand(asm, alloc, Reg::X0, h_src)?;
        load_operand(asm, alloc, Reg::X1, i_src)?;
        asm.ldr(Reg::X2, Reg::X0, 0)?; // length
        asm.cmp(Reg::X1, Reg::X2);
        let ok = asm.create_label();
        asm.b_cond(Cond::Lo, ok);
        asm.udf(0xDEAD);
        asm.bind(ok).map_err(BackendError::from)?;
        asm.mov_imm64(Reg::X3, elem_size.trailing_zeros() as u64);
        asm.lsl_reg(Reg::X1, Reg::X1, Reg::X3); // idx*elem_size
        asm.add(Reg::X0, Reg::X0, Reg::X1); // base + idx*size
        load_operand(asm, alloc, Reg::X2, v_src)?;
        asm.str_(Reg::X2, Reg::X0, 8)?; // store past the header
        return Ok(());
    }

    // ---- Heap cons cells (lispy `ref<LispyPair>`) — L3b -------------------
    //
    // `iir_builtin_lowering::lower_heap_builtins` rewrites a Lisp frontend's
    // `call_builtin "cons"/"car"/"cdr"/"null?"` into these word-granular heap
    // ops.  A pair is a **2-word (16-byte) cell**: field 0 = car/head,
    // field 1 = cdr/tail.  We allocate it with the same `__twig_alloc_bytes`
    // runtime helper `alloc_bytes` uses, and read/write fields with plain
    // 64-bit loads/stores at byte offset `idx*8`.  Values are **raw 64-bit
    // words** — no NaN-boxing — so a cons-of-integers program round-trips
    // exactly: `(CAR (CONS 7 9))` allocates a cell, stores 7/9, loads field
    // 0, and returns the raw `7`.  (V1 leaks like `alloc_bytes`; no GC.)

    // `alloc [<size>] -> <dest>` — allocate a GC-managed heap object.
    //
    // `srcs[0]` is an optional compile-time integer specifying the payload size
    // in bytes.  If absent (legacy IIR that omits the size), we default to 16
    // (two 8-byte words — the original LispyPair size).
    //
    // Calls `__twig_gc_alloc` (TWIG-GC, twig_gc.c) instead of the old
    // `__twig_alloc_bytes` so that the allocation is tracked by the GC and
    // freed when it becomes unreachable.  `__twig_gc_alloc` has the same
    // signature: one i64 argument (byte count), returns i64 pointer.
    if op == "alloc" {
        let dest = require_dest(instr)?;
        let size_bytes: u64 = match instr.srcs.first() {
            Some(CIROperand::Int(n)) if *n > 0 => *n as u64,
            _ => 16, // default: 2-word LispyPair
        };
        asm.mov_imm64(Reg::X0, size_bytes);
        asm.bl_external("__twig_gc_alloc");
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // `field_store <ptr>, <idx>, <value>` — `[ptr + idx*8] = value`.  No dest.
    if op == "field_store" {
        if instr.dest.is_some() {
            return Err(BackendError::MalformedInstr(
                "field_store: must not have a dest".into(),
            ));
        }
        let ptr_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("field_store: needs srcs[0]=ptr".into())
        })?;
        let off = field_offset(instr, 1)?;
        let val_src = instr.srcs.get(2).ok_or_else(|| {
            BackendError::MalformedInstr("field_store: needs srcs[2]=value".into())
        })?;
        load_operand(asm, alloc, Reg::X0, ptr_src)?;
        load_operand(asm, alloc, Reg::X1, val_src)?;
        asm.str_(Reg::X1, Reg::X0, off)?;
        return Ok(());
    }

    // `field_load <ptr>, <idx> -> <dest>` — `dest = [ptr + idx*8]`.
    if op == "field_load" {
        let dest = require_dest(instr)?;
        let ptr_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("field_load: needs srcs[0]=ptr".into())
        })?;
        let off = field_offset(instr, 1)?;
        load_operand(asm, alloc, Reg::X0, ptr_src)?;
        asm.ldr(Reg::X0, Reg::X0, off)?;
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // `is_null <x> -> <dest>` — `dest = (x == 0)` (nil is the 0 word).
    if op == "is_null" {
        let dest = require_dest(instr)?;
        let x_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("is_null: needs srcs[0]".into())
        })?;
        load_operand(asm, alloc, Reg::X0, x_src)?;
        asm.mov_imm64(Reg::X1, 0);
        asm.cmp(Reg::X0, Reg::X1);
        asm.cset(Reg::X0, Cond::Eq);
        let slot = alloc.slot_of(dest);
        asm.str_(Reg::X0, Reg::Sp, slot)?;
        return Ok(());
    }

    // `safepoint` — IIR back-edge / function-entry GC check.
    //
    // Lowers to a call to `__twig_gc_safepoint()` which triggers a GC
    // cycle when gc_live_bytes >= gc_threshold.  No arguments, no return
    // value, no dest register — the call clobbers X0-X7 + X30 per AAPCS64,
    // but the callee-save convention means live slots on the stack are
    // unaffected.  We do not need to save/restore anything because the frame
    // allocator already spilled all live values before the safepoint.
    if op == "safepoint" {
        asm.bl_external("__twig_gc_safepoint");
        return Ok(());
    }

    // Anything else is unsupported — caller should fall back.
    Err(BackendError::UnsupportedOp(op.to_string()))
}

/// Largest field byte-offset the heap ops accept.  Matches the LDR/STR
/// unsigned-immediate ceiling (`0x7FF8`, a multiple of 8), so the bound is
/// enforced **here** rather than relying on the lowering pass only ever
/// emitting field index 0/1 — a future producer with a larger index gets a
/// clean `MalformedInstr`, never a wrapped offset or out-of-bounds access.
const MAX_FIELD_OFFSET: u64 = 0x7FF8;

/// Read a `field_load`/`field_store` field-index operand — a compile-time
/// `Int` — and convert it to a byte offset (`idx * 8`).  Pair fields are
/// word-sized, so index 0 → offset 0 (car), index 1 → offset 8 (cdr).  A
/// negative, non-literal, or out-of-range index is a `MalformedInstr`.
fn field_offset(instr: &CIRInstr, i: usize) -> Result<u32, BackendError> {
    match instr.srcs.get(i) {
        Some(CIROperand::Int(n)) if *n >= 0 => (*n as u64)
            .checked_mul(8)
            .filter(|off| *off <= MAX_FIELD_OFFSET)
            .map(|off| off as u32)
            .ok_or_else(|| {
                BackendError::MalformedInstr(format!(
                    "{}: field index {n} is out of range",
                    instr.op
                ))
            }),
        _ => Err(BackendError::MalformedInstr(format!(
            "{}: field index at srcs[{i}] must be a non-negative integer literal",
            instr.op
        ))),
    }
}

#[derive(Debug, Clone, Copy)]
enum BinKind { Add, Sub, Mul }

/// LANG-FULL E2 (native-AOT leg): the bit-width of a narrow **unsigned** type,
/// or `None` for full-width / signed / non-integer types.
///
/// Native registers are 64-bit, so a `u8` add of `200 + 100` computes `300` in
/// the register — the high bits are *not* dropped the way a real 8-bit machine
/// would.  To make narrow-width arithmetic *wrap* (mod 2ⁿ) like the other
/// backends already do (vm-core, jit-core, wasm, jvm, cil), we mask the result
/// down to its declared width with a follow-up `AND`.  Only unsigned widths are
/// masked here; signed narrow types (`i8`/`i16`/`i32`) need sign-extension, not
/// a plain mask, and no current frontend emits them — left out of scope.
fn narrow_unsigned_bits(ty: &str) -> Option<u32> {
    match ty {
        "u4"  => Some(4),
        "u8"  => Some(8),
        "u16" => Some(16),
        "u32" => Some(32),
        _ => None, // u64 / i* / f* / bool / void — no masking
    }
}

/// Mask the value in `X0` down to `ty`'s width, in place, when `ty` is a narrow
/// unsigned type.  Uses `X2` as a scratch register for the mask constant; with
/// the stack-spill allocator every live value lives in a fixed stack slot, so
/// `X2` is never live between instructions and is free to borrow.  A no-op for
/// full-width / signed / non-integer types.
fn mask_narrow_x0(asm: &mut Assembler, ty: &str) {
    if let Some(bits) = narrow_unsigned_bits(ty) {
        // bits is 4/8/16/32, so `1 << bits` never overflows u64 and the mask is
        // a valid positive constant (0xF / 0xFF / 0xFFFF / 0xFFFF_FFFF).
        let mask = (1u64 << bits) - 1;
        asm.mov_imm64(Reg::X2, mask);
        asm.and_(Reg::X0, Reg::X0, Reg::X2);
    }
}

fn emit_binop(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    kind: BinKind,
    ty: &str,
) -> Result<(), BackendError> {
    if ty == "f64" {
        // ALGOL `real` arithmetic (LANG-FULL E3): operands ride D0/D1, the op
        // is `fadd`/`fsub`/`fmul`, and the double result is `str`'d back. No
        // E2 width mask (that's integer-only).
        let fk = match kind {
            BinKind::Add => FpBin::Add,
            BinKind::Sub => FpBin::Sub,
            BinKind::Mul => FpBin::Mul,
        };
        return emit_fp_binop(asm, alloc, instr, fk);
    }
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
    mask_narrow_x0(asm, ty); // E2: wrap u8/u16/u32 results mod 2ⁿ
    let slot = alloc.slot_of(dest);
    asm.str_(Reg::X0, Reg::Sp, slot)?;
    Ok(())
}

/// Which floating-point binary op (LANG-FULL E3).
#[derive(Debug, Clone, Copy)]
enum FpBin { Add, Sub, Mul, Div }

/// Load an `f64` operand into a D register (`Reg`'s `idx` names the D slot).
/// Frontends materialise constants into stack slots first, so an arithmetic/
/// compare operand is a `Var` — a `Float` *immediate* operand would need an
/// `fmov`/scratch-slot dance and isn't emitted on this slice.
fn load_fp_operand(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    dreg: Reg,
    op: &CIROperand,
) -> Result<(), BackendError> {
    match op {
        CIROperand::Var(name) => {
            let s = alloc.slot_of(name);
            asm.ldr_d(dreg, Reg::Sp, s)?;
            Ok(())
        }
        other => Err(BackendError::UnsupportedOp(format!(
            "f64 operand must be a Var (materialise the constant first), got {other:?}"
        ))),
    }
}

/// Emit an `f64` `add`/`sub`/`mul`/`div`: `ldr d0,[a]; ldr d1,[b]; f<op> d0,d0,d1;
/// str d0,[dest]` (LANG-FULL E3). IEEE division by zero is `±inf`/`NaN` (no trap).
fn emit_fp_binop(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    kind: FpBin,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    if instr.srcs.len() < 2 {
        return Err(BackendError::MalformedInstr(format!("{} needs 2 srcs", instr.op)));
    }
    load_fp_operand(asm, alloc, Reg::X0, &instr.srcs[0])?; // D0
    load_fp_operand(asm, alloc, Reg::X1, &instr.srcs[1])?; // D1
    match kind {
        FpBin::Add => asm.fadd(Reg::X0, Reg::X0, Reg::X1),
        FpBin::Sub => asm.fsub(Reg::X0, Reg::X0, Reg::X1),
        FpBin::Mul => asm.fmul(Reg::X0, Reg::X0, Reg::X1),
        FpBin::Div => asm.fdiv(Reg::X0, Reg::X0, Reg::X1),
    }
    let slot = alloc.slot_of(dest);
    asm.str_d(Reg::X0, Reg::Sp, slot)?;
    Ok(())
}

/// Emit an `f64` comparison: `ldr d0,[a]; ldr d1,[b]; fcmp d0,d1; cset x0,<cond>;
/// str x0,[dest]` (LANG-FULL E3). The boolean result is an `int` 0/1. FP
/// condition codes give IEEE *ordered* semantics — a NaN operand makes every
/// `<`/`<=`/`>`/`>=`/`==` false (and `!=` true).
fn emit_fp_cmp(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    rel: CmpRel,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    if instr.srcs.len() < 2 {
        return Err(BackendError::MalformedInstr(format!("{} needs 2 srcs", instr.op)));
    }
    load_fp_operand(asm, alloc, Reg::X0, &instr.srcs[0])?; // D0
    load_fp_operand(asm, alloc, Reg::X1, &instr.srcs[1])?; // D1
    asm.fcmp(Reg::X0, Reg::X1);
    // After FCMP: less→N=1; equal→Z=1,C=1; greater→C=1; unordered→C=1,V=1.
    let cond = match rel {
        CmpRel::Eq => Cond::Eq, // Z=1
        CmpRel::Ne => Cond::Ne, // Z=0 (includes unordered)
        CmpRel::Lt => Cond::Mi, // N=1 — only ordered-less-than
        CmpRel::Le => Cond::Ls, // C=0 or Z=1 — ordered ≤
        CmpRel::Gt => Cond::Gt, // Z=0 && N==V — ordered >
        CmpRel::Ge => Cond::Ge, // N==V — ordered ≥
    };
    asm.cset(Reg::X0, cond);
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
    if ty == "f64" {
        return emit_fp_cmp(asm, alloc, instr, rel);
    }
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
    if ty == "f64" {
        // ALGOL `real` division (LANG-FULL E3). `mod` is integer-only, so a
        // `mod_f64` would be a frontend bug.
        if want_mod {
            return Err(BackendError::UnsupportedOp("mod_f64 (real modulo is undefined)".into()));
        }
        return emit_fp_binop(asm, alloc, instr, FpBin::Div);
    }
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

    // E2: div/mod of in-range uⁿ operands already fits, so this mask is a no-op;
    // kept uniform with the other narrow ops.
    mask_narrow_x0(asm, ty);
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
    ty: &str,
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
    // E2: AND/OR/XOR of two already-masked uⁿ operands stays in range, so this
    // mask is provably redundant — but keeping it uniform with the other narrow
    // ops costs two instructions and guards against an unmasked operand.
    mask_narrow_x0(asm, ty);
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
    ty: &str,
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
    // E2: a left shift can push bits above the declared width (`1u8 << 8` must
    // be 0, not 256), so mask the result.  Right shifts only ever shrink the
    // value, so the mask is a no-op there — applied uniformly for simplicity.
    mask_narrow_x0(asm, ty);
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

/// The byte size of an E5 array element type on the native backend. 64-bit
/// integer and `f64` elements share the same 8-byte memory representation here:
/// the backend copies raw bits between stack slots and array storage, while f64
/// arithmetic/comparisons load those bits through FP registers when needed.
/// Smaller element widths still produce a clear error rather than a silently
/// wrong stride.
fn native_array_elem_size(elem: &str) -> Result<i32, BackendError> {
    match elem {
        // E4d-BA-arr: a `str` element (BASIC `DIM A$(n)`) is an 8-byte runtime
        // string handle — the address of a `[i64 len][bytes]` block — stored and
        // loaded as a plain word exactly like an i64, so no separate str load/store
        // path is needed (twig-aot already materialises the handle into the slot).
        "i64" | "u64" | "f64" | "str" => Ok(8),
        other => Err(BackendError::MalformedInstr(format!(
            "array element {other:?} not supported on the native backend (8-byte elements only so far)"
        ))),
    }
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
    // Mirror the prologue: small frames use the combined `LDP … [SP],#frame`
    // (post-indexed); large frames use two separate `LDR`s + `ADD SP,SP,#frame`.
    if frame <= 504 {
        asm.ldp_post(Reg::Fp, Reg::Lr, Reg::Sp, frame as i32)?;
    } else {
        asm.ldr(Reg::Fp, Reg::Sp, 0)?; // restore fp from [sp+0]
        asm.ldr(Reg::Lr, Reg::Sp, 8)?; // restore lr from [sp+8]
        asm.add_imm(Reg::Sp, Reg::Sp, frame)?; // sp += frame
    }
    asm.ret();
    Ok(())
}

// ===========================================================================
// (emit_print_helper removed in LANG41)
//
// The LANG40 `emit_print_helper()` function emitted a self-contained ARM64
// helper with macOS-specific `write(2)` syscall numbers (`x16=4`, `SVC #0x80`)
// hard-coded as ARM64 instruction words.  This was non-portable: the syscall
// ABI differs between macOS and Linux.
//
// LANG41 replaces it with a proper runtime library (`runtime/twig_runtime.c`,
// compiled by `twig-aot/build.rs`).  The `io_out` CIR opcode handler still
// emits `BL __twig_print_i64` via `bl_external` (unchanged), but:
//
//  - `twig-aot`'s two-pass linker now *collects* unresolved BL targets as
//    `ExternBranchReloc` entries instead of failing.
//  - `pack_object_with_globals_and_externals` emits `N_UNDF | N_EXT` symbol-
//    table entries and `ARM64_RELOC_BRANCH26` records for them.
//  - `invoke_ld` writes the embedded runtime archive to a temp file and
//    passes it to `ld`, which patches the BL offsets from `printf`-based code.
// ===========================================================================

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

    fn heap(op: &str, dest: Option<&str>, srcs: Vec<CIROperand>, ty: &str) -> CIRInstr {
        CIRInstr { op: op.into(), dest: dest.map(Into::into), srcs, ty: ty.into(), deopt_to: None }
    }

    // L3b: the cons heap ops (`alloc`/`field_store`/`field_load`/`is_null`)
    // that `lower_heap_builtins` produces from `cons`/`car`/`cdr`/`null?`.

    // LANG-FULL E5 — bounds-checked arrays lower to a `__twig_alloc_bytes` call,
    // an explicit `cmp`+`b.lo`+`udf` bounds trap, and base+idx*8 loads/stores.
    #[test]
    fn array_ops_lower_with_bounds_trap() {
        let cir = vec![
            const_u64("n", 3),
            heap("alloc_array", Some("a"), vec![CIROperand::Var("n".into())], "any"),
            const_u64("i", 0),
            const_u64("v", 42),
            heap("array_set", None,
                 vec![CIROperand::Var("a".into()), CIROperand::Var("i".into()), CIROperand::Var("v".into())], "i64"),
            heap("array_get", Some("r"),
                 vec![CIROperand::Var("a".into()), CIROperand::Var("i".into())], "i64"),
            heap("array_len", Some("m"), vec![CIROperand::Var("a".into())], "i64"),
            ret_u64("r"),
        ];
        let bytes = compile(&ctx("arr", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("array ops must lower: {e}"));
        let words: Vec<u32> = bytes.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        // Two bounds checks (array_get + array_set) ⇒ at least two `udf #0xDEAD` traps.
        let traps = words.iter().filter(|&&w| w == 0x0000DEAD).count();
        assert!(traps >= 2, "expected ≥2 udf bounds traps, got {traps} in {words:?}");
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    /// `f64` array elements lower as raw 8-byte loads/stores; f64 math reads
    /// the same bits from the destination slot through FP registers later.
    #[test]
    fn array_get_accepts_f64_element() {
        let cir = vec![
            const_u64("n", 1),
            heap("alloc_array", Some("a"), vec![CIROperand::Var("n".into())], "any"),
            const_u64("i", 0),
            heap("array_get", Some("r"),
                 vec![CIROperand::Var("a".into()), CIROperand::Var("i".into())], "f64"),
            ret_u64("r"),
        ];
        assert!(compile(&ctx("arr", &[], "u64"), &cir).is_ok(),
            "f64 array element should lower as an 8-byte native load");
    }

    #[test]
    fn cons_car_heap_ops_lower() {
        // The CIR for `(CAR (CONS 7 9))`: allocate a 2-word cell, store 7
        // and 9 into fields 0 and 1, load field 0 back.
        let cir = vec![
            const_u64("h", 7),
            const_u64("t", 9),
            heap("alloc", Some("cell"), vec![], "any"),
            heap("field_store", None,
                 vec![CIROperand::Var("cell".into()), CIROperand::Int(0), CIROperand::Var("h".into())], "void"),
            heap("field_store", None,
                 vec![CIROperand::Var("cell".into()), CIROperand::Int(1), CIROperand::Var("t".into())], "void"),
            heap("field_load", Some("r"),
                 vec![CIROperand::Var("cell".into()), CIROperand::Int(0)], "any"),
            ret_u64("r"),
        ];
        let bytes = compile(&ctx("cons_car", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("cons/car heap ops must lower: {e}"));
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    // L3b-2b (LANG77): the *runtime-call* form of `(CAR (CONS 7 9))` —
    // cons/car are `call_builtin "lispy_*"` dispatching to `__dyn_*`,
    // the alternative to the structural ops above (see `RUNTIME_RENAMES`).

    fn call_builtin(dest: Option<&str>, name: &str, args: &[&str]) -> CIRInstr {
        let mut srcs = vec![CIROperand::Var(name.into())];
        srcs.extend(args.iter().map(|a| CIROperand::Var((*a).into())));
        CIRInstr { op: "call_builtin".into(), dest: dest.map(Into::into),
                   srcs, ty: "any".into(), deopt_to: None }
    }

    #[test]
    fn lispy_runtime_cons_car_emit_external_calls() {
        // `(CAR (CONS 7 9))` via the runtime path: two BLs to the C lisp
        // runtime, recorded as external relocations the linker resolves from
        // the runtime archive.
        let cir = vec![
            const_u64("h", 7),
            const_u64("t", 9),
            call_builtin(Some("cell"), "lispy_cons", &["h", "t"]),
            call_builtin(Some("r"), "lispy_car", &["cell"]),
            ret_u64("r"),
        ];
        let (bytes, ext) = compile_with_relocs(&ctx("lispy", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("lispy runtime calls must lower: {e}"));
        assert!(!bytes.is_empty() && bytes.len() % 4 == 0);
        let symbols: Vec<&str> = ext.iter().map(|r| r.symbol.as_str()).collect();
        assert!(symbols.contains(&"__dyn_cons"), "missing cons call: {symbols:?}");
        assert!(symbols.contains(&"__dyn_car"), "missing car call: {symbols:?}");
    }

    #[test]
    fn lispy_cons_wrong_arity_is_rejected() {
        // lispy_cons takes exactly 2 args — one arg must be a soft refusal.
        let cir = vec![
            const_u64("h", 7),
            call_builtin(Some("cell"), "lispy_cons", &["h"]),
            ret_u64("cell"),
        ];
        assert!(compile(&ctx("bad_cons", &[], "u64"), &cir).is_err());
    }

    #[test]
    fn lispy_full_boxed_cons_car_unbox_lowers() {
        // The complete L3b-2c-1 CIR for `(CAR (CONS 7 9))`: boxed atoms
        // (7<<3, 9<<3), cons, car, then unbox the result for the exit code.
        let cir = vec![
            const_u64("h", 7 << 3),
            const_u64("t", 9 << 3),
            call_builtin(Some("cell"), "lispy_cons", &["h", "t"]),
            call_builtin(Some("boxed"), "lispy_car", &["cell"]),
            call_builtin(Some("r"), "lispy_unbox_int", &["boxed"]),
            ret_u64("r"),
        ];
        let (bytes, ext) = compile_with_relocs(&ctx("full", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("boxed cons/car/unbox must lower: {e}"));
        assert!(!bytes.is_empty() && bytes.len() % 4 == 0);
        let symbols: Vec<&str> = ext.iter().map(|r| r.symbol.as_str()).collect();
        for want in ["__dyn_cons", "__dyn_car", "__dyn_unbox_int"] {
            assert!(symbols.contains(&want), "missing {want}: {symbols:?}");
        }
    }

    #[test]
    fn lispy_atom_eq_predicates_and_truthy_lower() {
        // L3b-2c-2: `(ATOM 5)` = not(pair?(5)), normalised for a branch via
        // lispy_truthy; plus equal? (EQ). All four predicates must lower.
        let cir = vec![
            const_u64("x", 5 << 3),
            call_builtin(Some("p"), "lispy_pair_p", &["x"]),
            call_builtin(Some("a"), "lispy_not", &["p"]),
            call_builtin(Some("t"), "lispy_truthy", &["a"]),
            call_builtin(Some("e"), "lispy_equal", &["x", "x"]),
            ret_u64("e"),
        ];
        let (bytes, ext) = compile_with_relocs(&ctx("preds", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("predicates must lower: {e}"));
        assert!(!bytes.is_empty() && bytes.len() % 4 == 0);
        let symbols: Vec<&str> = ext.iter().map(|r| r.symbol.as_str()).collect();
        for want in [
            "__dyn_pair_p", "__dyn_not",
            "__dyn_truthy", "__dyn_equal",
        ] {
            assert!(symbols.contains(&want), "missing {want}: {symbols:?}");
        }
    }

    /// W14b (F7): the universal exit coercion `lispy_to_exit_code` — the program
    /// boundary for a polymorphic lambda result — lowers to a BL into the runtime.
    #[test]
    fn lispy_to_exit_code_lowers() {
        assert!(lookup_builtin("lispy_to_exit_code").is_some(), "builtin must be registered");
        let cir = vec![
            const_u64("x", 5 << 3),
            call_builtin(Some("r"), "lispy_to_exit_code", &["x"]),
            ret_u64("r"),
        ];
        let (bytes, ext) = compile_with_relocs(&ctx("exit_coerce", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("to_exit_code must lower: {e}"));
        assert!(!bytes.is_empty() && bytes.len() % 4 == 0);
        let symbols: Vec<&str> = ext.iter().map(|r| r.symbol.as_str()).collect();
        assert!(
            symbols.contains(&"__dyn_to_exit_code"),
            "missing __dyn_to_exit_code: {symbols:?}",
        );
    }

    #[test]
    fn is_null_lowers() {
        let cir = vec![
            const_u64("x", 0),
            heap("is_null", Some("r"), vec![CIROperand::Var("x".into())], "bool"),
            ret_u64("r"),
        ];
        let bytes = compile(&ctx("isnull", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("is_null must lower: {e}"));
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn field_store_rejects_dest_and_non_literal_index() {
        // field_store must not carry a dest.
        let bad_dest = vec![heap("field_store", Some("oops"),
            vec![CIROperand::Var("cell".into()), CIROperand::Int(0), CIROperand::Var("h".into())], "void")];
        assert!(compile(&ctx("bad", &[], "u64"), &bad_dest).is_err());
        // a non-literal field index is rejected.
        let bad_idx = vec![heap("field_load", Some("r"),
            vec![CIROperand::Var("cell".into()), CIROperand::Var("i".into())], "any")];
        assert!(compile(&ctx("bad2", &[], "u64"), &bad_idx).is_err());
        // an out-of-range field index is rejected (no wrap, no OOB).
        let huge = vec![heap("field_load", Some("r"),
            vec![CIROperand::Var("cell".into()), CIROperand::Int(1 << 40)], "any")];
        assert!(compile(&ctx("bad3", &[], "u64"), &huge).is_err());
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
        assert!(!bytes.is_empty());
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
        assert!(!bytes.is_empty());
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
        assert!(!bytes.is_empty());
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
        assert!(!bytes.is_empty());
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
        assert!(!bytes.is_empty());
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
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn div_u64_lowers() {
        let bytes = compile_with_binop("div_u64", "u64", false);
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn mod_i64_lowers() {
        let bytes = compile_with_binop("mod_i64", "i64", true);
        // mod expands to sdiv + msub = 2 extra instructions vs div
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn mod_u64_lowers() {
        let bytes = compile_with_binop("mod_u64", "u64", false);
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn and_i64_lowers() {
        let bytes = compile_with_binop("and_i64", "i64", true);
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn or_i64_lowers() {
        let bytes = compile_with_binop("or_i64", "i64", true);
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn xor_i64_lowers() {
        let bytes = compile_with_binop("xor_i64", "i64", true);
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn shl_i64_lowers() {
        let bytes = compile_with_binop("shl_i64", "i64", true);
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn shr_i64_lowers_asr() {
        // Signed shift right → ASRV
        let bytes = compile_with_binop("shr_i64", "i64", true);
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
    }

    #[test]
    fn shr_u64_lowers_lsr() {
        // Unsigned shift right → LSRV
        let bytes = compile_with_binop("shr_u64", "u64", false);
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
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
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
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
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(4));
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
        assert!(!bytes.is_empty() && bytes.len() % 4 == 0, "byte-aligned output");
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
        assert!(!bytes.is_empty() && bytes.len() % 4 == 0, "byte-aligned output");
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
        assert!(words.contains(&0x0000DEAD), "expected udf #0xDEAD in {words:?}");
    }

    // ---- LANG40/LANG41: io_out handler (BL __twig_print_i64 external reloc) ----

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

    // ── LANG75 — call_builtin lowering ────────────────────────────────────────

    #[test]
    fn call_builtin_putchar_emits_bl_reloc() {
        // CIR:
        //   const_i32 v0 = 65
        //   call_builtin "putchar", v0
        //   ret_void
        let cir = vec![
            CIRInstr { op: "const_i32".into(), dest: Some("v0".into()),
                       srcs: vec![CIROperand::Int(65)],
                       ty: "i32".into(), deopt_to: None },
            CIRInstr { op: "call_builtin".into(), dest: None,
                       srcs: vec![CIROperand::Var("putchar".into()),
                                  CIROperand::Var("v0".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None,
                       srcs: vec![], ty: "void".into(), deopt_to: None },
        ];
        let (_bytes, ext_relocs, _) = compile_with_globals(
            &ctx("emit_A", &[], "void"), &cir, &HashMap::new()
        ).expect("call_builtin should compile");
        // Exactly one BL placeholder to __twig_putchar.
        let putc_relocs: Vec<_> = ext_relocs.iter()
            .filter(|r| r.symbol == "__twig_putchar").collect();
        assert_eq!(putc_relocs.len(), 1,
                   "expected exactly one __twig_putchar BL placeholder");
    }

    #[test]
    fn call_builtin_getchar_stores_x0_to_dest() {
        // CIR: call_builtin "getchar" → r; ret_i32 r
        let cir = vec![
            CIRInstr { op: "call_builtin".into(), dest: Some("r".into()),
                       srcs: vec![CIROperand::Var("getchar".into())],
                       ty: "i32".into(), deopt_to: None },
            CIRInstr { op: "ret_i32".into(), dest: None,
                       srcs: vec![CIROperand::Var("r".into())],
                       ty: "i32".into(), deopt_to: None },
        ];
        let (_bytes, ext_relocs, _) = compile_with_globals(
            &ctx("read_one", &[], "i32"), &cir, &HashMap::new()
        ).expect("call_builtin should compile");
        // One BL placeholder to __twig_getchar.
        assert_eq!(ext_relocs.iter().filter(|r| r.symbol == "__twig_getchar").count(), 1);
    }

    #[test]
    fn call_builtin_print_string_records_two_arg_loads() {
        // CIR: const_i64 p=0; const_i64 n=0; call_builtin "print_string", p, n
        let cir = vec![
            CIRInstr { op: "const_i64".into(), dest: Some("p".into()),
                       srcs: vec![CIROperand::Int(0)],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "const_i64".into(), dest: Some("n".into()),
                       srcs: vec![CIROperand::Int(0)],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "call_builtin".into(), dest: None,
                       srcs: vec![CIROperand::Var("print_string".into()),
                                  CIROperand::Var("p".into()),
                                  CIROperand::Var("n".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None,
                       srcs: vec![], ty: "void".into(), deopt_to: None },
        ];
        let (_bytes, ext_relocs, _) = compile_with_globals(
            &ctx("emit_str", &[], "void"), &cir, &HashMap::new()
        ).expect("call_builtin should compile");
        assert_eq!(ext_relocs.iter().filter(|r| r.symbol == "__twig_print_string").count(), 1);
    }

    #[test]
    fn call_builtin_unknown_name_refuses() {
        let cir = vec![
            CIRInstr { op: "call_builtin".into(), dest: None,
                       srcs: vec![CIROperand::Var("frobnicate".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None, srcs: vec![],
                       ty: "void".into(), deopt_to: None },
        ];
        let result = compile_with_globals(
            &ctx("bad_name", &[], "void"), &cir, &HashMap::new()
        );
        assert!(result.is_err(), "unknown builtin name should error");
    }

    #[test]
    fn call_builtin_wrong_arity_refuses() {
        // putchar expects 1 arg; supplying 0 should be rejected.
        let cir = vec![
            CIRInstr { op: "call_builtin".into(), dest: None,
                       srcs: vec![CIROperand::Var("putchar".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None, srcs: vec![],
                       ty: "void".into(), deopt_to: None },
        ];
        let result = compile_with_globals(
            &ctx("bad_arity", &[], "void"), &cir, &HashMap::new()
        );
        assert!(result.is_err(), "wrong arity should error");
    }

    // ── LANG76 — byte memory ops + heap allocation ────────────────────────────

    #[test]
    fn alloc_bytes_emits_bl_to_runtime() {
        let cir = vec![
            CIRInstr { op: "const_i64".into(), dest: Some("n".into()),
                       srcs: vec![CIROperand::Int(16)],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "alloc_bytes".into(), dest: Some("buf".into()),
                       srcs: vec![CIROperand::Var("n".into())],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "ret_i64".into(), dest: None,
                       srcs: vec![CIROperand::Var("buf".into())],
                       ty: "i64".into(), deopt_to: None },
        ];
        let (_bytes, ext_relocs, _) = compile_with_globals(
            &ctx("a", &[], "i64"), &cir, &HashMap::new()
        ).expect("alloc_bytes should compile");
        assert_eq!(ext_relocs.iter().filter(|r| r.symbol == "__twig_alloc_bytes").count(), 1);
    }

    #[test]
    fn load_byte_compiles() {
        // CIR: load_byte ptr, off -> v; ret_i64 v
        let params = vec![("ptr".into(), "i64".into()),
                          ("off".into(), "i64".into())];
        let cir = vec![
            CIRInstr { op: "load_byte".into(), dest: Some("v".into()),
                       srcs: vec![CIROperand::Var("ptr".into()),
                                  CIROperand::Var("off".into())],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "ret_i64".into(), dest: None,
                       srcs: vec![CIROperand::Var("v".into())],
                       ty: "i64".into(), deopt_to: None },
        ];
        let bytes = compile(&ctx("lb", &params, "i64"), &cir).expect("compile ok");
        // LDRB W0, [X0]: 0x39400000 (Rt=0, Rn=0, imm12=0).
        let words: Vec<u32> = bytes.chunks(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert!(words.contains(&0x39400000),
                "expected LDRB W0, [X0] (0x39400000) in {words:08X?}");
    }

    #[test]
    fn store_byte_compiles() {
        // CIR: store_byte ptr, off, val
        let params = vec![("ptr".into(), "i64".into()),
                          ("off".into(), "i64".into()),
                          ("val".into(), "i64".into())];
        let cir = vec![
            CIRInstr { op: "store_byte".into(), dest: None,
                       srcs: vec![CIROperand::Var("ptr".into()),
                                  CIROperand::Var("off".into()),
                                  CIROperand::Var("val".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None,
                       srcs: vec![], ty: "void".into(), deopt_to: None },
        ];
        let bytes = compile(&ctx("sb", &params, "void"), &cir).expect("compile ok");
        // STRB W2, [X0]: 0x39000002 (Rt=2, Rn=0, imm12=0).
        let words: Vec<u32> = bytes.chunks(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert!(words.contains(&0x39000002),
                "expected STRB W2, [X0] (0x39000002) in {words:08X?}");
    }

    #[test]
    fn load_byte_missing_offset_refuses() {
        let params = vec![("ptr".into(), "i64".into())];
        let cir = vec![
            CIRInstr { op: "load_byte".into(), dest: Some("v".into()),
                       srcs: vec![CIROperand::Var("ptr".into())],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "ret_i64".into(), dest: None,
                       srcs: vec![CIROperand::Var("v".into())],
                       ty: "i64".into(), deopt_to: None },
        ];
        assert!(compile(&ctx("bad", &params, "i64"), &cir).is_err());
    }

    #[test]
    fn store_byte_with_dest_refuses() {
        let cir = vec![
            CIRInstr { op: "const_i64".into(), dest: Some("p".into()),
                       srcs: vec![CIROperand::Int(0)],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "const_i64".into(), dest: Some("o".into()),
                       srcs: vec![CIROperand::Int(0)],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "const_i64".into(), dest: Some("v".into()),
                       srcs: vec![CIROperand::Int(0)],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "store_byte".into(), dest: Some("r".into()),  // illegal!
                       srcs: vec![CIROperand::Var("p".into()),
                                  CIROperand::Var("o".into()),
                                  CIROperand::Var("v".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None, srcs: vec![],
                       ty: "void".into(), deopt_to: None },
        ];
        assert!(compile(&ctx("bad", &[], "void"), &cir).is_err());
    }

    #[test]
    fn call_builtin_print_i64_matches_io_out() {
        // call_builtin "print_i64", v should produce the same BL target.
        let cir = vec![
            CIRInstr { op: "const_i64".into(), dest: Some("v0".into()),
                       srcs: vec![CIROperand::Int(42)],
                       ty: "i64".into(), deopt_to: None },
            CIRInstr { op: "call_builtin".into(), dest: None,
                       srcs: vec![CIROperand::Var("print_i64".into()),
                                  CIROperand::Var("v0".into())],
                       ty: "void".into(), deopt_to: None },
            CIRInstr { op: "ret_void".into(), dest: None, srcs: vec![],
                       ty: "void".into(), deopt_to: None },
        ];
        let (_bytes, ext_relocs, _) = compile_with_globals(
            &ctx("p", &[], "void"), &cir, &HashMap::new()
        ).expect("call_builtin should compile");
        assert_eq!(ext_relocs.iter().filter(|r| r.symbol == "__twig_print_i64").count(), 1);
    }

    // =======================================================================
    // LANG-FULL E2 (native-AOT leg): narrow-width unsigned masking
    // =======================================================================
    //
    // A native 64-bit register holds the full result of `add_u8 200, 100`
    // (= 300); to make a `uⁿ` type *wrap* mod-2ⁿ like the other backends, the
    // codegen appends a `mov X2, #mask; and X0, X0, X2` after each narrow op.
    // The structural tests below prove the mask bytes are emitted (every host);
    // the executed tests prove the *value* wraps (Apple-Silicon macOS only,
    // where we can install and call the generated code via `jit-loader-macos`).

    /// Build `const_<ty> a; const_<ty> b; <op>_<ty> v0 = a,b; ret_u64 v0`.
    fn narrow_binop_module(op: &str, ty: &str, a: i64, b: i64) -> Vec<u8> {
        let mk_const = |dest: &str, n: i64| CIRInstr {
            op: format!("const_{ty}"), dest: Some(dest.into()),
            srcs: vec![CIROperand::Int(n)], ty: ty.into(), deopt_to: None,
        };
        let cir = vec![
            mk_const("a", a),
            mk_const("b", b),
            make_binop_cir(&format!("{op}_{ty}"), "v0", "a", "b", ty),
            ret_u64("v0"),
        ];
        compile(&ctx("f", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("{op}_{ty} compile failed: {e}"))
    }

    /// Build `const_<ty> a; <op>_<ty> v0 = a; ret_u64 v0`.
    fn narrow_unop_module(op: &str, ty: &str, a: i64) -> Vec<u8> {
        let cir = vec![
            CIRInstr { op: format!("const_{ty}"), dest: Some("a".into()),
                       srcs: vec![CIROperand::Int(a)], ty: ty.into(), deopt_to: None },
            make_unop_cir(&format!("{op}_{ty}"), "v0", "a", ty),
            ret_u64("v0"),
        ];
        compile(&ctx("f", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("{op}_{ty} compile failed: {e}"))
    }

    #[test]
    fn narrow_add_emits_two_extra_mask_instructions() {
        // add_u64 (no mask) vs add_u8 (mask) differ ONLY by `mov X2,#mask`
        // + `and X0,X0,X2` = two 4-byte ARM64 instructions = 8 bytes.
        let wide = narrow_binop_module("add", "u64", 200, 100);
        let narrow = narrow_binop_module("add", "u8", 200, 100);
        assert_eq!(narrow.len(), wide.len() + 8,
            "u8 add must emit a 2-instruction width mask the u64 add does not");
    }

    #[test]
    fn narrow_not_emits_mask() {
        let wide = narrow_unop_module("not", "u64", 0);
        let narrow = narrow_unop_module("not", "u8", 0);
        assert_eq!(narrow.len(), wide.len() + 8);
    }

    #[test]
    fn i64_ops_are_never_masked() {
        // i64 is full-width — no mask, identical length to u64.
        assert_eq!(
            narrow_binop_module("add", "i64", 1, 2).len(),
            narrow_binop_module("add", "u64", 1, 2).len(),
        );
    }

    // ---- Executed proofs: install the bytes and call them. ----
    // Gated to Apple-Silicon macOS, the only host where `jit-loader-macos`
    // installs MAP_JIT pages.  Linux-aarch64 / x86 hosts run the structural
    // tests above; the lang-aot matrix provides the end-to-end executed proof
    // once a frontend emits narrow `type_hint`s (LANG-FULL N6).
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    fn run_narrow_binop(op: &str, ty: &str, a: i64, b: i64) -> u64 {
        let bytes = narrow_binop_module(op, ty, a, b);
        let page = jit_loader_macos::CodePage::new(&bytes).expect("install code page");
        let f: extern "C" fn() -> u64 = unsafe { page.as_function() };
        f()
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    fn run_narrow_unop(op: &str, ty: &str, a: i64) -> u64 {
        let bytes = narrow_unop_module(op, ty, a);
        let page = jit_loader_macos::CodePage::new(&bytes).expect("install code page");
        let f: extern "C" fn() -> u64 = unsafe { page.as_function() };
        f()
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    #[test]
    fn u8_arithmetic_wraps_when_executed() {
        assert_eq!(run_narrow_binop("add", "u8", 200, 100), 44);  // 300 & 0xFF
        assert_eq!(run_narrow_binop("mul", "u8", 16, 16), 0);     // 256 & 0xFF
        assert_eq!(run_narrow_binop("sub", "u8", 0, 1), 255);     // -1 & 0xFF
        assert_eq!(run_narrow_binop("add", "u8", 255, 1), 0);     // cell wrap
        // i64 at full width does NOT wrap.
        assert_eq!(run_narrow_binop("add", "i64", 200, 100), 300);
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    #[test]
    fn u8_not_and_shift_wrap_when_executed() {
        assert_eq!(run_narrow_unop("not", "u8", 0), 255);   // ~0 over a byte
        assert_eq!(run_narrow_binop("shl", "u8", 1, 7), 128);
        assert_eq!(run_narrow_binop("shl", "u8", 1, 8), 0); // shifted past the byte
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    #[test]
    fn u16_u32_widths_wrap_when_executed() {
        assert_eq!(run_narrow_binop("add", "u16", 60000, 10000), 70000 & 0xFFFF);
        // u32: a 64-bit register does NOT wrap a u32 add for free, so the mask
        // is what makes this correct (unlike wasm, where the i32 op wraps).
        assert_eq!(run_narrow_binop("mul", "u32", 0x1_0000, 0x1_0000), 0);
    }

    // ---- f64 (ALGOL `real`) — LANG-FULL E3 ----

    fn const_f64(dest: &str, v: f64) -> CIRInstr {
        CIRInstr { op: "const_f64".into(), dest: Some(dest.into()),
                   srcs: vec![CIROperand::Float(v)], ty: "f64".into(), deopt_to: None }
    }

    /// `const a; const b; <op>_f64 v0 = a,b; ret_u64 v0` → returns the f64 result's
    /// raw bits (the value rides its 8-byte stack slot).
    fn f64_binop_module(op: &str, a: f64, b: f64) -> Vec<u8> {
        let cir = vec![
            const_f64("a", a),
            const_f64("b", b),
            make_binop_cir(&format!("{op}_f64"), "v0", "a", "b", "f64"),
            ret_u64("v0"),
        ];
        compile(&ctx("f", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("{op}_f64 compile failed: {e}"))
    }

    /// `const a; const b; cmp_<rel>_f64 v0 = a,b; ret_u64 v0` → returns 0/1.
    fn f64_cmp_module(rel: &str, a: f64, b: f64) -> Vec<u8> {
        let cir = vec![
            const_f64("a", a),
            const_f64("b", b),
            make_binop_cir(&format!("cmp_{rel}_f64"), "v0", "a", "b", "f64"),
            ret_u64("v0"),
        ];
        compile(&ctx("f", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("cmp_{rel}_f64 compile failed: {e}"))
    }

    /// f64 arithmetic and comparisons compile on every host (structural — runs on
    /// the x86 CI box too, where the executed proofs below are `cfg`'d out).
    #[test]
    fn f64_ops_compile() {
        for op in ["add", "sub", "mul", "div"] {
            assert!(!f64_binop_module(op, 2.5, 2.0).is_empty(), "{op}_f64 should compile");
        }
        for rel in ["eq", "ne", "lt", "le", "gt", "ge"] {
            assert!(!f64_cmp_module(rel, 2.5, 2.0).is_empty(), "cmp_{rel}_f64 should compile");
        }
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    fn run_u64(bytes: &[u8]) -> u64 {
        let page = jit_loader_macos::CodePage::new(bytes).expect("install code page");
        let f: extern "C" fn() -> u64 = unsafe { page.as_function() };
        f()
    }

    /// **Executed** on real Apple-Silicon hardware: f64 arithmetic returns the
    /// exact IEEE-754 result bits (LANG-FULL E3).
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    #[test]
    fn f64_arithmetic_executes() {
        assert_eq!(run_u64(&f64_binop_module("mul", 2.5, 2.0)), 5.0_f64.to_bits());
        assert_eq!(run_u64(&f64_binop_module("add", 2.5, 0.25)), 2.75_f64.to_bits());
        assert_eq!(run_u64(&f64_binop_module("sub", 2.5, 0.25)), 2.25_f64.to_bits());
        assert_eq!(run_u64(&f64_binop_module("div", 7.0, 2.0)), 3.5_f64.to_bits());
    }

    /// **Executed**: f64 comparisons return the right 0/1 boolean.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    #[test]
    fn f64_comparisons_execute() {
        assert_eq!(run_u64(&f64_cmp_module("eq", 5.0, 5.0)), 1);
        assert_eq!(run_u64(&f64_cmp_module("eq", 5.0, 4.0)), 0);
        assert_eq!(run_u64(&f64_cmp_module("lt", 3.5, 4.0)), 1);
        assert_eq!(run_u64(&f64_cmp_module("lt", 4.0, 3.5)), 0);
        assert_eq!(run_u64(&f64_cmp_module("gt", 4.0, 3.5)), 1);
        assert_eq!(run_u64(&f64_cmp_module("le", 5.0, 5.0)), 1);
        assert_eq!(run_u64(&f64_cmp_module("ge", 5.0, 5.0)), 1);
        assert_eq!(run_u64(&f64_cmp_module("ne", 5.0, 4.0)), 1);
    }

    // ---- LANG-FULL E8: int ⇄ real conversions ----

    /// A single-source conversion CIR instruction (`int_to_real`,
    /// `real_to_int_trunc`, `real_to_int_floor`).
    fn make_unary_cir(op: &str, dest: &str, src: &str, ty: &str) -> CIRInstr {
        CIRInstr { op: op.into(), dest: Some(dest.into()),
                   srcs: vec![CIROperand::Var(src.into())], ty: ty.into(), deopt_to: None }
    }

    /// `const i = a; r = int_to_real i; const c = b; d = r − c; o = <round> d;
    /// ret_u64 o` — the round op is `real_to_int_trunc`/`real_to_int_floor`.
    /// Returns the i64 result as a u64 bit pattern.
    fn e8_round_module(round_op: &str, a: i64, b: f64) -> Vec<u8> {
        let cir = vec![
            const_u64("i", a),
            make_unary_cir("int_to_real", "r", "i", "f64"),
            const_f64("c", b),
            make_binop_cir("sub_f64", "d", "r", "c", "f64"),
            make_unary_cir(round_op, "o", "d", "i64"),
            ret_u64("o"),
        ];
        compile(&ctx("f", &[], "u64"), &cir)
            .unwrap_or_else(|e| panic!("{round_op} compile failed: {e}"))
    }

    /// The conversion ops compile on every host (structural — the executed
    /// proof below is `cfg`'d to Apple Silicon only).
    #[test]
    fn e8_conversions_compile() {
        assert!(!e8_round_module("real_to_int_trunc", 45, 0.0).is_empty(), "trunc should compile");
        assert!(!e8_round_module("real_to_int_floor", 45, 0.0).is_empty(), "floor should compile");
    }

    /// **Executed** on real Apple-Silicon hardware: the full conversion chain
    /// (`scvtf` → `fsub` → optional `frintm` → `fcvtzs`) produces the right i64.
    /// `floor(int_to_real(45) − 2.7) = floor(42.3) = 42` matches the
    /// LLVM/WASM/VM/JVM/CLR matrix-cell value.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    #[test]
    fn e8_conversions_execute() {
        // floor(45.0 − 2.7) = floor(42.3) = 42
        assert_eq!(run_u64(&e8_round_module("real_to_int_floor", 45, 2.7)), 42);
        // trunc(45.0 − 2.7) = trunc(42.3) = 42 (toward zero drops the .3)
        assert_eq!(run_u64(&e8_round_module("real_to_int_trunc", 45, 2.7)), 42);
        // Sign sensitivity: floor(2.0 − 4.7) = floor(−2.7) = −3, but
        // trunc(−2.7) = −2. Verifies frintm rounds toward −∞, not toward zero.
        assert_eq!(run_u64(&e8_round_module("real_to_int_floor", 2, 4.7)) as i64, -3);
        assert_eq!(run_u64(&e8_round_module("real_to_int_trunc", 2, 4.7)) as i64, -2);
        // Round-trip identity: int_to_real(45) − 0.0 → 45.0 → 45.
        assert_eq!(run_u64(&e8_round_module("real_to_int_trunc", 45, 0.0)), 45);
    }
}
