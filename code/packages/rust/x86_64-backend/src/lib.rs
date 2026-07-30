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

use gc_core::{StackMapBuilder, StackMapRecord};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use vm_core::value::Value;
use x86_64_encoder::{
    Assembler, Cond, EncodeError, ExternalReloc, ExternalRelocKind, LabelId, Reg,
};

pub use x86_64_encoder::ExternalReloc as Reloc;

// ===========================================================================
// GC precise-roots: reference-typed slots (AOT00-T1)
// ===========================================================================

/// The machine scalar types whose slots can **never** hold a GC reference. A slot of
/// any other type — notably `any`, `str`, `ref<…>`, `array<…>` — is treated as a
/// potential root (see [`is_gc_root_ty`]).
///
/// This is a **deny-list**, not an allow-list, and deliberately so: `aot_core`'s
/// specialiser erases every reference type to `"any"` before it reaches a backend, so
/// a rule keyed on `ref<…>` would never fire and would emit records naming *nothing* —
/// and an empty record authoritatively suppresses a frame's conservative scan, a
/// use-after-free. The deny-list is immune to that erasure. Mirrors the aarch64 backend
/// exactly so both native targets agree on which slots are roots.
fn is_definitely_not_ref(ty: &str) -> bool {
    matches!(
        ty,
        "u4" | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "f32"
            | "f64"
            | "bool"
            | "void"
    )
}

/// Could a value of this CIR type be a GC reference (and so need naming as a root)?
/// True for everything except the machine scalars in [`is_definitely_not_ref`] —
/// including, critically, `any`, the normal type of every dynamic heap value.
fn is_gc_root_ty(ty: &str) -> bool {
    !is_definitely_not_ref(ty)
}

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
    compile_inner(ctx, ir, abi, &HashMap::new())
        .map(|(bytes, _relocs, _stack_map)| bytes)
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
    compile_inner(ctx, ir, abi, &HashMap::new())
        .map(|(bytes, relocs, _stack_map)| (bytes, relocs))
        .map_err(|e| format!("x86_64-backend: {e:?}"))
}

/// Like [`compile_function_with_relocs`] but also handles
/// `global_load` / `global_store` CIR instructions using the supplied
/// slot map.
///
/// `global_slots` maps each global name (as it appears in
/// `srcs[0].as_var()`) to a zero-based slot index.  Slot `i`
/// corresponds to bytes `[i*8, i*8 + 8)` in the `_twig_globals` data
/// section that the packager (LANG45) emits.
///
/// Returns the function's machine code bytes plus any external
/// relocations.  Cross-function `call`s, `io_out` calls into the
/// runtime, and globals access all surface as entries in the
/// returned relocation list (with `PltRel32` for calls and
/// `PcRel32` for global addresses).
pub fn compile_function_with_globals(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
    abi: X86_64Abi,
    global_slots: &HashMap<String, usize>,
) -> Result<(Vec<u8>, Vec<Reloc>), String> {
    compile_inner(ctx, ir, abi, global_slots)
        .map(|(bytes, relocs, _stack_map)| (bytes, relocs))
        .map_err(|e| format!("x86_64-backend: {e:?}"))
}

/// Like [`compile_function_with_globals`] but also returns the function's **GC
/// precise-roots stack map** — one [`StackMapRecord`] per call-return safepoint, naming
/// the reference-typed frame slots live there — for `__gc_collect_precise` to resolve a
/// return address to its exact roots. The x86-64 analogue of the aarch64 backend's
/// `compile_with_globals_and_stackmap`; the emitted machine code is byte-for-byte
/// identical to [`compile_function_with_globals`] (the map is derived, not injected).
#[allow(clippy::type_complexity)]
pub fn compile_function_with_globals_and_stackmap(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
    abi: X86_64Abi,
    global_slots: &HashMap<String, usize>,
) -> Result<(Vec<u8>, Vec<Reloc>, Vec<StackMapRecord>), String> {
    compile_inner(ctx, ir, abi, global_slots).map_err(|e| format!("x86_64-backend: {e:?}"))
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
// | `str_eq`      | `int64_t __twig_str_eq(int64_t, int64_t)`    | yes     |

#[derive(Debug, Clone, Copy)]
struct BuiltinSig {
    /// The bare helper name (e.g. `"putchar"`).  The backend prepends
    /// `__twig_` when emitting the linker symbol.
    name: &'static str,
    /// Number of CIR arguments the helper expects (the `name` Var in
    /// srcs[0] is not counted here).
    n_args: usize,
    /// `true` if the helper writes a return value into RAX/X0 that the
    /// backend should store into the dest slot.
    returns: bool,
}

/// V1 helper table shared by every backend.  Order is the documentation
/// order from the spec; lookup is `O(n)` against six entries which is
/// faster than a `HashMap` at this scale.
const V1_BUILTINS: &[BuiltinSig] = &[
    BuiltinSig { name: "print_i64",    n_args: 1, returns: false },
    BuiltinSig { name: "putchar",      n_args: 1, returns: false },
    BuiltinSig { name: "getchar",      n_args: 0, returns: true  },
    BuiltinSig { name: "print_string", n_args: 2, returns: false },
    BuiltinSig { name: "input_i64",    n_args: 0, returns: true  },
    // E4-dyn: BASIC string `INPUT A$` — reads a whole line as a runtime string.
    // Same 0-arg / returns-i64 shape as `input_i64`; the returned i64 is the
    // handle (base address) of a `[i64 len][bytes]` heap block, carried in RAX
    // like any other pointer-as-i64 (`alloc_bytes`/`str_eq`), so no new lowering.
    BuiltinSig { name: "input_str",    n_args: 0, returns: true  },
    BuiltinSig { name: "exit",         n_args: 1, returns: false },
    // LANG76 — heap allocator.  Returns a pointer (treated as i64).
    BuiltinSig { name: "alloc_bytes",  n_args: 1, returns: true  },
    // LANG77 — the shared lisp value runtime (McCarthy Lisp L3b-2b).  These
    // dispatch to `__dyn_*` in `twig-aot/runtime/dynval_runtime.c`,
    // which implements `lispy-runtime`'s NaN-box tagged-value model.  Each
    // takes/returns an opaque 64-bit `LispyValue`.  No backend-specific
    // logic — the generic `call_builtin` path marshals args + emits the CALL.
    BuiltinSig { name: "dyn_cons",   n_args: 2, returns: true  },
    BuiltinSig { name: "dyn_car",    n_args: 1, returns: true  },
    BuiltinSig { name: "dyn_cdr",    n_args: 1, returns: true  },
    // LANG77 L3b-2c — unbox a tagged integer to a raw machine word at the
    // program-exit boundary.  `int64_t __dyn_unbox_int(uint64_t)`.
    BuiltinSig { name: "dyn_unbox_int", n_args: 1, returns: true },
    // E6d-2b — box a raw machine word back into a tagged `DynValue` at runtime
    // (`n << 3`), for a *dynamic* value that is not a compile-time constant —
    // e.g. the result of dynamic arithmetic re-entering the lisp value world.
    // `uint64_t __dyn_box_int(int64_t)`.
    BuiltinSig { name: "dyn_box_int", n_args: 1, returns: true },
    // LANG77 L3b-2c-2 — the ATOM/EQ predicates (return tagged #t/#f) and the
    // COND truthiness normaliser (returns a raw 0/1 for jmp_if_false).
    BuiltinSig { name: "dyn_pair_p",    n_args: 1, returns: true },
    BuiltinSig { name: "dyn_null_p",    n_args: 1, returns: true },
    BuiltinSig { name: "dyn_not",       n_args: 1, returns: true },
    BuiltinSig { name: "dyn_equal",     n_args: 2, returns: true },
    BuiltinSig { name: "dyn_truthy",    n_args: 1, returns: true },
    // LANG77 W13b — the universal program-exit coercion for a polymorphic
    // (lambda / `any`) result: dispatch on the runtime tag.
    // `int64_t __dyn_to_exit_code(uint64_t)`.
    BuiltinSig { name: "dyn_to_exit_code", n_args: 1, returns: true },
    // LANG-STR-RT — runtime string ops on LANG-STR-RT length-prefixed buffers.
    // Both operands are i64 pointers to `[int64_t len][char bytes...]` buffers.
    BuiltinSig { name: "str_eq", n_args: 2, returns: true },
    // E4-dyn runtime string concatenation.  `int64_t __twig_str_concat(int64_t a,
    // int64_t b)` reads both `[i64 len][bytes]` headers and returns a handle to a
    // fresh joined block.  Same 2-arg / returns-i64 shape as `str_eq` (operand handles
    // ride RDI/RSI, the result handle rides RAX), so the generic `call_builtin`
    // marshaller needs no new codegen — only this table entry.
    BuiltinSig { name: "str_concat", n_args: 2, returns: true },
    // TWIG-GC (native-aot-substrate PR-1) — GC-managed allocation and safepoint.
    BuiltinSig { name: "gc_alloc",     n_args: 1, returns: true  },
    BuiltinSig { name: "gc_safepoint", n_args: 0, returns: false },
    // AOT00-T1 increment C / x86_64 PR-x3 — GC collection + observability entry points
    // a native program uses to drive and measure a collection (→ `__twig_gc_*` aliases
    // in gc-core-capi). `gc_collect` = forced conservative full collect; `gc_collect_precise`
    // = precise-roots stack walk (returns objects freed); `gc_collect_compacting` =
    // precise-roots *moving* collect (relocates movable survivors, rewriting the caller's
    // root slots — spec AOT00-T3 §5; degrades to `gc_collect_precise` when nothing is
    // movable); `gc_live_bytes` = live payload bytes; `gc_stackmap_count` = registered-
    // function count. Mirrors the aarch64 backend.
    BuiltinSig { name: "gc_collect",            n_args: 0, returns: false },
    BuiltinSig { name: "gc_collect_precise",    n_args: 0, returns: true  },
    BuiltinSig { name: "gc_collect_compacting", n_args: 0, returns: true  },
    // Incremental (bounded-pause) cycle (spec AOT00-T4 §6): start → step(budget)→done? →
    // finish. Auto-emits `__twig_gc_collect_incremental_*` via the generic `__twig_<name>`
    // dispatch. Mirrors the aarch64 backend.
    BuiltinSig { name: "gc_collect_incremental_start",  n_args: 0, returns: false },
    BuiltinSig { name: "gc_collect_incremental_step",   n_args: 1, returns: true  },
    BuiltinSig { name: "gc_collect_incremental_finish", n_args: 0, returns: true  },
    BuiltinSig { name: "gc_live_bytes",         n_args: 0, returns: true  },
    BuiltinSig { name: "gc_stackmap_count",     n_args: 0, returns: true  },
    // AOT00-T5 — declare a variable-length reference-array layout so the collector traces (and
    // under compaction relocates) the array + its elements precisely. `(fixed, fixed_count,
    // tail_from) -> kind_id`: the seam a language frontend's array type calls; auto-emits
    // `__twig_gc_register_ref_array_kind` via the generic `__twig_<name>` dispatch. Pass
    // `fixed = 0, fixed_count = 0, tail_from = 0` for a pure reference array (every word a ref).
    BuiltinSig { name: "gc_register_ref_array_kind", n_args: 3, returns: true },
];

fn lookup_builtin(name: &str) -> Option<BuiltinSig> {
    V1_BUILTINS.iter().copied().find(|s| s.name == name)
}

fn v1_builtin_names() -> Vec<&'static str> {
    V1_BUILTINS.iter().map(|s| s.name).collect()
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

#[allow(clippy::type_complexity)]
fn compile_inner(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
    abi: X86_64Abi,
    global_slots: &HashMap<String, usize>,
) -> Result<(Vec<u8>, Vec<ExternalReloc>, Vec<StackMapRecord>), BackendError> {
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
    // Return addresses of self-recursive `call <fn_name>` sites. These use an internal
    // label fixup (no `PltRel32` reloc), so — unlike cross-function/builtin calls —
    // `build_stack_map` can't recover them from the relocation list; `emit_instr`
    // appends each one here as it emits the call (see AOT00-T1 §5, PR-x4).
    let mut recursive_safepoints: Vec<u32> = Vec::new();
    for instr in ir {
        emit_instr(&mut asm, instr, &mut alloc, &labels, frame, ctx.name, abi,
                   global_slots, &mut recursive_safepoints)?;
    }

    // ---- Defensive epilogue (in case CIR falls off the end) ----------------
    emit_epilogue(&mut asm, frame);

    let external_relocs = std::mem::take(&mut asm.external_relocs);
    // Build this function's GC stack map before finishing (it needs the final slot
    // assignment + the call relocations, both available here).
    let stack_map = build_stack_map(ctx, ir, &alloc, frame, &external_relocs,
                                    &recursive_safepoints)?;
    let bytes = asm.finish().map_err(BackendError::from)?;
    Ok((bytes, external_relocs, stack_map))
}

/// Build this function's GC stack map: every reference-typed slot, and a safepoint
/// record at every **call return address**.
///
/// **Slot offsets are RBP-relative and negative.** Locals live at `[rbp − 8 − 8·slot]`
/// (see [`RegAlloc::rbp_offset`]), so the value stored in [`StackMapRecord::slots`] is
/// that signed offset directly — the precise walker reads `rbp + offset` to recover the
/// slot, exactly as [`gc_core`] expects (offsets "may be negative for slots below FP").
///
/// **Safepoints come from two sources.** x86-64 is variable-width, so — unlike the
/// fixed-width aarch64 backend, which post-scans finished code for every `BL` — return
/// addresses can't be recovered by a byte scan. Instead:
///
/// 1. **Cross-function / builtin / libm calls** patch a 4-byte displacement through a
///    `PltRel32` relocation, so their return address is `patch_offset + 4` (the byte just
///    after the call). This captures every call that leaves the module — the usual GC
///    trigger.
/// 2. **Self-recursive calls** (`call <fn_name>`) resolve through an internal label fixup
///    and emit *no* reloc, so `compile_one_with_globals` records each one's return address
///    (`asm.len()` right after the 5-byte `call rel32`) in `recursive_safepoints` and
///    passes it here (AOT00-T1 §5, PR-x4). A recursive frame that allocates can trigger a
///    collection just like any other call, so its live references must be mapped too;
///    before this, recursive frames fell back to a conservative scan (safe but imprecise —
///    they could pin integer look-alikes). Duplicate PCs across the two sources collapse in
///    [`StackMapBuilder::safepoint`], so order and overlap are harmless.
fn build_stack_map(
    ctx: &FunctionContext<'_>,
    ir: &[CIRInstr],
    alloc: &RegAlloc,
    frame: u32,
    relocs: &[ExternalReloc],
    recursive_safepoints: &[u32],
) -> Result<Vec<StackMapRecord>, BackendError> {
    let mut b = StackMapBuilder::new(frame);

    // Declare every reference-typed slot. A ref-typed name with no assigned slot is a
    // compiler bug — silently dropping a root is a use-after-free, so error rather than
    // skip. (Unreachable today: the pre-pass mints a slot for every param/dest/src.)
    let mut declare = |name: &str| -> Result<(), BackendError> {
        let slot = *alloc.slots.get(name).ok_or_else(|| {
            BackendError::MalformedInstr(format!("stack map: no frame slot for '{name}'"))
        })?;
        b.define_ref_slot(RegAlloc::rbp_offset(slot));
        Ok(())
    };
    for (name, ty) in ctx.params {
        if is_gc_root_ty(ty) {
            declare(name)?;
        }
    }
    for instr in ir {
        if let Some(dest) = &instr.dest {
            if is_gc_root_ty(&instr.ty) {
                declare(dest)?;
            }
        }
    }

    // A safepoint at every external call's return address (`patch_offset + 4`)...
    for r in relocs {
        if r.kind == ExternalRelocKind::PltRel32 {
            b.safepoint((r.patch_offset + 4) as u32);
        }
    }
    // ...and at every self-recursive call's return address (no reloc; recovered at emit
    // time). `safepoint` dedups + keeps ascending, so overlaps/ordering are harmless.
    for &pc in recursive_safepoints {
        b.safepoint(pc);
    }
    Ok(b.into_records())
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

#[allow(clippy::too_many_arguments)]
fn emit_instr(
    asm: &mut Assembler,
    instr: &CIRInstr,
    alloc: &mut RegAlloc,
    labels: &HashMap<String, LabelId>,
    frame: u32,
    fn_name: &str,
    abi: X86_64Abi,
    global_slots: &HashMap<String, usize>,
    // Self-recursive `call <fn_name>` return addresses, appended as emitted, so
    // `build_stack_map` can add them as safepoints (they carry no reloc). AOT00-T1 §5.
    recursive_safepoints: &mut Vec<u32>,
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
            // An `f64` (ALGOL `real`, LANG-FULL E3) rides its 8-byte stack slot
            // as raw IEEE-754 bits — identical to an integer slot — so we
            // materialise the bit pattern in a GPR and store it. No XMM register
            // is needed to load a *constant*; only arithmetic/compare use SSE.
            CIROperand::Float(f) => f.to_bits(),
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

    // --- LANG-FULL E8: int ⇄ real conversions ---
    //
    // These arrive with their bare IIR names (the `specialise` pass passes
    // unrecognised ops through unchanged), so they are matched here, not via a
    // typed `_<ty>` suffix. `Reg::Rax` names both `rax` and `xmm0` — the opcode
    // selects the register file, exactly as the f64 arithmetic path relies on.
    //
    //   int_to_real        mov rax,[src]; cvtsi2sd xmm0,rax;       movsd [dest],xmm0
    //   real_to_int_trunc  movsd xmm0,[src]; cvttsd2si rax,xmm0;   mov [dest],rax
    //   real_to_int_floor  movsd xmm0,[src]; roundsd xmm0,xmm0,1; cvttsd2si rax,xmm0; mov [dest],rax
    //
    // `cvttsd2si` truncates toward zero and yields the integer-indefinite
    // `0x8000…0` on NaN/±∞/out-of-range (no trap) — a documented divergence from
    // the VM trap, shared with the JVM/aarch64 backends; every finite, in-range
    // value converts identically. `roundsd … ,1` rounds toward −∞ (floor).
    if op == "int_to_real" {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr("int_to_real needs srcs[0]".into()))?;
        load_operand(asm, alloc, Reg::Rax, src);       // i64 → rax
        asm.cvtsi2sd(Reg::Rax, Reg::Rax);              // xmm0 = (double) rax
        let slot = alloc.slot_of(dest);
        asm.movsd_store(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax); // store xmm0
        return Ok(());
    }
    if op == "real_to_int_trunc" || op == "real_to_int_floor" {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_fp_operand(asm, alloc, Reg::Rax, src)?;   // f64 → xmm0
        if op == "real_to_int_floor" {
            asm.roundsd(Reg::Rax, Reg::Rax, 1);        // xmm0 = floor(xmm0)  (mode 1 = −∞)
        }
        asm.cvttsd2si(Reg::Rax, Reg::Rax);             // rax = (i64) xmm0  (trunc toward zero)
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax); // store rax
        return Ok(());
    }
    // AL8 sqrt — `SQRTSD xmm0,xmm0` (SSE2; single hardware FP instruction, no libm).
    //   f64_sqrt dest <- src  →  movsd xmm0,[src]; sqrtsd xmm0,xmm0; movsd [dest],xmm0
    // NaN propagates; negative input → NaN (IEEE-754 / matches VM `f64::sqrt`).
    if op == "f64_sqrt" {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr("f64_sqrt needs srcs[0]".into()))?;
        load_fp_operand(asm, alloc, Reg::Rax, src)?;            // f64 → xmm0
        asm.sqrtsd(Reg::Rax, Reg::Rax);                         // xmm0 = sqrt(xmm0)
        let slot = alloc.slot_of(dest);
        asm.movsd_store(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax); // store xmm0
        return Ok(());
    }
    // BA-pow — `call pow` (libm two-argument power, no hardware opcode).
    //   f64_pow dest <- base, exp  →  movsd xmm0,[base]; movsd xmm1,[exp]; call pow; movsd [dest],xmm0
    // SysV: first FP arg xmm0, second xmm1; result xmm0.  MS x64: same (xmm0/xmm1).
    if op == "f64_pow" {
        let dest = require_dest(instr)?;
        let base = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr("f64_pow needs srcs[0]".into()))?;
        let exp_ = instr.srcs.get(1)
            .ok_or_else(|| BackendError::MalformedInstr("f64_pow needs srcs[1]".into()))?;
        load_fp_operand(asm, alloc, Reg::Rax, base)?;           // xmm0 = base
        load_fp_operand(asm, alloc, Reg::Rcx, exp_)?;           // xmm1 = exp
        asm.call_rel32("pow", ExternalRelocKind::PltRel32);     // xmm0 = pow(xmm0, xmm1)
        let slot = alloc.slot_of(dest);
        asm.movsd_store(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax); // store xmm0
        return Ok(());
    }

    // AL8 transcendentals — sin/cos/ln/exp via libm (`call rel32` → xmm0).
    //
    // Both System V AMD64 and MS x64 pass the first f64 arg in xmm0 and
    // return the f64 result in xmm0, so there is no ABI difference here.
    // libm is pre-linked on Linux (`-lm`) and macOS (`-lSystem`).
    // ALGOL `ln` maps to libm `log` (natural logarithm).
    if matches!(op, "f64_sin" | "f64_cos" | "f64_ln" | "f64_exp" | "f64_atan" | "f64_tan") {
        let libm_sym = match op {
            "f64_sin"  => "sin",
            "f64_cos"  => "cos",
            "f64_ln"   => "log",   // libm natural log is `log`, not `ln`
            "f64_exp"  => "exp",
            "f64_atan" => "atan",
            "f64_tan"  => "tan",
            _ => unreachable!(),
        };
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(
                format!("{op} needs srcs[0]")))?;
        load_fp_operand(asm, alloc, Reg::Rax, src)?;                     // f64 → xmm0
        asm.call_rel32(libm_sym, ExternalRelocKind::PltRel32);            // call sin/cos/log/exp
        let slot = alloc.slot_of(dest);
        asm.movsd_store(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax); // store xmm0 result
        return Ok(());
    }

    // --- add_<ty> / sub_<ty> / mul_<ty> ---
    if let Some(ty) = op.strip_prefix("add_") { return emit_binop(asm, alloc, instr, BinOp::Add, ty); }
    if let Some(ty) = op.strip_prefix("sub_") { return emit_binop(asm, alloc, instr, BinOp::Sub, ty); }
    if let Some(ty) = op.strip_prefix("mul_") { return emit_binop(asm, alloc, instr, BinOp::Mul, ty); }

    // --- cmp_<rel>_<ty> ---
    if let Some(rest) = op.strip_prefix("cmp_") {
        let (rel, signed) = parse_cmp_suffix(rest)
            .ok_or_else(|| BackendError::MalformedInstr(format!("bad cmp mnemonic: {op}")))?;
        // A `cmp_*_f64` (ALGOL `real`, LANG-FULL E3) uses `ucomisd` + `setcc`,
        // not the integer `cmp` path.
        if rest.ends_with("_f64") {
            return emit_fp_cmp(asm, alloc, instr, rel);
        }
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
            // `call_label` emits a 5-byte `call rel32` (no reloc), so `asm.len()` is now
            // the return address the GC walker will observe at `[rbp + 8]` if a collection
            // fires inside this recursive call. Record it as a safepoint (AOT00-T1 §5) —
            // the recursive frame's live references must be mapped, not conservatively
            // scanned. (Cross-function calls below carry a `PltRel32` reloc instead, which
            // `build_stack_map` recovers separately.)
            recursive_safepoints.push(asm.len() as u32);
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

    // --- global_load name → dest  (LANG39 parity) ---
    //
    // CIR encoding: dest = result_var; srcs[0] = Var(global_name).
    //
    // x86-64 sequence (much simpler than ARM64's ADRP+ADD pair):
    //
    //   lea  rax, [rip + _twig_globals]    ; PcRel32 reloc on _twig_globals
    //   mov  rax, [rax + slot*8]            ; load 64-bit value
    //   mov  [rbp + dest_slot], rax         ; store to dest slot
    //
    // The PcRel32 reloc record carries `addend = -4` (encoder default).
    // The slot byte offset is encoded in the second instruction's disp32.
    if op == "global_load" {
        let dest = require_dest(instr)?;
        let name = instr.srcs.first().and_then(CIROperand::as_var)
            .ok_or_else(|| BackendError::MalformedInstr(
                "global_load: srcs[0] must be Var(name)".into()))?;
        let slot_idx = *global_slots.get(name).ok_or_else(|| {
            BackendError::MalformedInstr(format!("global_load: unknown global '{name}'"))
        })?;
        asm.lea_rip_rel(Reg::Rax, "_twig_globals", ExternalRelocKind::PcRel32);
        let byte_off: i32 = (slot_idx as i64 * 8)
            .try_into()
            .map_err(|_| BackendError::MalformedInstr(
                format!("global_load: slot byte offset overflows i32 (slot={slot_idx})")))?;
        asm.mov_r64_mem(Reg::Rax, Reg::Rax, byte_off);
        let dest_slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(dest_slot), Reg::Rax);
        return Ok(());
    }

    // --- global_store name, val  (LANG39 parity) ---
    //
    // CIR encoding: dest = None; srcs[0] = Var(name); srcs[1] = Var(value).
    //
    // x86-64 sequence:
    //
    //   mov  rcx, [rbp + val_slot]          ; load value
    //   lea  rax, [rip + _twig_globals]    ; PcRel32 reloc
    //   mov  [rax + slot*8], rcx            ; write to global slot
    if op == "global_store" {
        let name = instr.srcs.first().and_then(CIROperand::as_var)
            .ok_or_else(|| BackendError::MalformedInstr(
                "global_store: srcs[0] must be Var(name)".into()))?;
        let slot_idx = *global_slots.get(name).ok_or_else(|| {
            BackendError::MalformedInstr(format!("global_store: unknown global '{name}'"))
        })?;
        let val_src = instr.srcs.get(1).ok_or_else(|| {
            BackendError::MalformedInstr("global_store: needs srcs[1]=value".into())
        })?;
        // Load value into RCX (RAX is needed for the LEA result).
        load_operand(asm, alloc, Reg::Rcx, val_src);
        asm.lea_rip_rel(Reg::Rax, "_twig_globals", ExternalRelocKind::PcRel32);
        let byte_off: i32 = (slot_idx as i64 * 8)
            .try_into()
            .map_err(|_| BackendError::MalformedInstr(
                format!("global_store: slot byte offset overflows i32 (slot={slot_idx})")))?;
        asm.mov_mem_r64(Reg::Rax, byte_off, Reg::Rcx);
        return Ok(());
    }

    // --- io_out val  (LANG40/LANG41 parity) ---
    //
    // CIR encoding: dest = None; srcs[0] = Var(value).
    //
    // Lowers to a CALL into the runtime helper `__twig_print_i64`,
    // passing the value in the ABI's first argument register:
    //
    //   System V: mov rdi, [rbp + val_slot]; call __twig_print_i64
    //   MS x64:   mov rcx, [rbp + val_slot]; call __twig_print_i64
    //
    // The runtime archive (LANG46) provides the helper symbol; the
    // packager (LANG45) emits the PltRel32 reloc record the linker
    // patches.
    //
    // Stack alignment: prologue established RSP ≡ 0 (mod 16); CALL
    // pushes 8 bytes for the return addr, so during the helper's
    // execution RSP ≡ 8 (mod 16) — exactly what the ABI requires on
    // entry to a function.  No per-call adjustment needed.  MS x64
    // shadow space was already reserved in the prologue.
    if op == "io_out" {
        let val_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("io_out: needs srcs[0]=value".into())
        })?;
        let arg0_reg = abi.arg_regs()[0];
        load_operand(asm, alloc, arg0_reg, val_src);
        asm.call_rel32("__twig_print_i64", ExternalRelocKind::PltRel32);
        return Ok(());
    }

    // --- call_builtin "<name>", <args>  (LANG75) ---
    //
    // Generic dispatch to runtime helpers.  Looks `name` up in the V1
    // helper table (see `lookup_builtin`), validates arg count, marshals
    // each arg into the ABI's i-th argument register (RDI/RSI/… on SysV,
    // RCX/RDX/… on MS x64), then emits `call rel32` against the symbol
    // `__twig_<name>` with a `PltRel32` external relocation.  If the
    // helper returns a value, the dest slot receives RAX after the call.
    //
    // `io_out v` is sugar for `call_builtin "print_i64", v` and stays in
    // the dispatch above for backwards compatibility with existing
    // frontends and tests.
    //
    // Unknown helper names → `BackendError::MalformedInstr` (the spec's
    // "BackendRefused" — a soft refusal rather than a hard panic).
    if op == "call_builtin" {
        // srcs[0] must be Var(name) — the helper name without the
        // `__twig_` prefix.
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
        if sig.n_args > abi.max_args() {
            return Err(BackendError::TooManyParams {
                abi: match abi { X86_64Abi::SysV => "SysV", X86_64Abi::MsX64 => "MsX64" },
                got: sig.n_args,
                max: abi.max_args(),
            });
        }

        // Marshal arguments into the ABI's argument registers in order.
        // Stack-spill allocation guarantees no aliasing between sources.
        let arg_regs = abi.arg_regs();
        for (i, src) in arg_srcs.iter().enumerate() {
            load_operand(asm, alloc, arg_regs[i], src);
        }

        // Emit `call rel32` to the runtime symbol.  Both Linux (PLT32 → relaxed
        // to PC32 by the linker for local resolution) and Windows (REL32) treat
        // this the same way.  Two runtime families share this dispatch: the
        // **twig** runtime exports `__twig_<name>` (`print_i64`, `getchar`, …),
        // while the **dyn** value runtime (`dynval_runtime.c`) exports
        // `__dyn_<name>` for the tagged-value builtins, whose IIR names already
        // carry the `dyn_` namespace (`dyn_cons` → `__dyn_cons`).  So a `dyn_*`
        // builtin is `__` + name; everything else is `__twig_` + name.
        let symbol = if name.starts_with("dyn_") {
            format!("__{name}")
        } else {
            format!("__twig_{name}")
        };
        asm.call_rel32(&symbol, ExternalRelocKind::PltRel32);

        // If the helper returns, store RAX into the dest slot.
        if sig.returns {
            if let Some(dest) = &instr.dest {
                let slot = alloc.slot_of(dest);
                asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
            }
        }
        return Ok(());
    }

    // ── LANG76 — byte memory ops + heap allocation ────────────────────────────

    // `alloc_bytes <n> -> <dest>` — sugar for `call_builtin "alloc_bytes", n`.
    //
    // The spec exposes a separate CIR mnemonic so frontends don't have to
    // think about V1_BUILTINS arity validation.  Internally we just emit
    // the same CALL sequence the `call_builtin` arm above would produce,
    // sharing the `__twig_alloc_bytes` runtime symbol and PltRel32 reloc.
    if op == "alloc_bytes" {
        let dest = require_dest(instr)?;
        let n_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("alloc_bytes: needs srcs[0]=byte_count".into())
        })?;
        let arg0_reg = abi.arg_regs()[0];
        load_operand(asm, alloc, arg0_reg, n_src);
        asm.call_rel32("__twig_alloc_bytes", ExternalRelocKind::PltRel32);
        // Return pointer in RAX → dest slot.
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // `load_byte <ptr>, <offset> -> <dest>` — read one byte from
    // `[ptr + offset]`, zero-extend to 64 bits, store into `dest`.
    //
    // Sequence (uses RAX + RCX scratch; both reserved by RegAlloc):
    //   mov  rax, [rbp + ptr_slot]      ; pointer
    //   mov  rcx, [rbp + offset_slot]   ; offset
    //   add  rax, rcx                    ; rax = ptr + offset
    //   movzx rax, byte ptr [rax]        ; zero-extend load
    //   mov  [rbp + dest_slot], rax
    if op == "load_byte" {
        let dest = require_dest(instr)?;
        let ptr_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("load_byte: needs srcs[0]=ptr".into())
        })?;
        let off_src = instr.srcs.get(1).ok_or_else(|| {
            BackendError::MalformedInstr("load_byte: needs srcs[1]=offset".into())
        })?;
        load_operand(asm, alloc, Reg::Rax, ptr_src);
        load_operand(asm, alloc, Reg::Rcx, off_src);
        asm.add(Reg::Rax, Reg::Rcx);
        asm.movzx_r64_byte_at(Reg::Rax, Reg::Rax);
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // `store_byte <ptr>, <offset>, <value>` — write the low 8 bits of
    // `value` to `[ptr + offset]`.  No dest.
    //
    // Sequence:
    //   mov  rax, [rbp + ptr_slot]
    //   mov  rcx, [rbp + offset_slot]
    //   add  rax, rcx
    //   mov  rdx, [rbp + value_slot]
    //   mov  byte ptr [rax], dl          ; store low 8 bits
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
        load_operand(asm, alloc, Reg::Rax, ptr_src);
        load_operand(asm, alloc, Reg::Rcx, off_src);
        asm.add(Reg::Rax, Reg::Rcx);
        load_operand(asm, alloc, Reg::Rdx, val_src);
        asm.mov_byte_at_r8(Reg::Rax, Reg::Rdx);
        return Ok(());
    }

    // ── LANG-FULL E5 — bounds-checked arrays (static, length-prefixed) ─────────
    //
    // An array is a single `__twig_alloc_bytes` block laid out as `[i64 length]
    // [elem 0][elem 1]…`; the IIR *handle* is the block base. The native target
    // has no managed runtime, so `array_get`/`array_set` emit an EXPLICIT
    // unsigned bounds compare and trap with `ud2` — the x86_64 twin of the LLVM
    // `icmp uge`+`llvm.trap` and WASM `i64.ge_u`+`unreachable` lowerings.

    // `alloc_array <count> -> <dest>` — allocate `8 + count*8` bytes, store the
    // length header, return base.  (The AOT specialiser collapses the `array<T>`
    // result type to `any`, so the element width is not on `instr.ty` here; the
    // native backend only supports 8-byte elements, so the stride is a fixed 8 —
    // `array_get`/`array_set` validate the element width per access.)
    //   rdi = count ; shl rdi,3 ; add rdi,8 ; call __twig_alloc_bytes ; [rax]=count
    if op == "alloc_array" {
        let dest = require_dest(instr)?;
        let count_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("alloc_array: needs srcs[0]=count".into())
        })?;
        let arg0_reg = abi.arg_regs()[0];
        load_operand(asm, alloc, arg0_reg, count_src);
        asm.shl_imm8(arg0_reg, 3); // count*8
        asm.add_imm32(arg0_reg, 8); // + 8-byte length header
        asm.call_rel32("__twig_alloc_bytes", ExternalRelocKind::PltRel32);
        // dest = base (rax); then write [base+0] = count.
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        load_operand(asm, alloc, Reg::Rcx, count_src); // reload (call clobbered regs)
        asm.mov_mem_r64(Reg::Rax, 0, Reg::Rcx);
        return Ok(());
    }

    // `array_len <handle> -> <dest>` — load the i64 length header at `[base+0]`.
    if op == "array_len" {
        let dest = require_dest(instr)?;
        let h_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("array_len: needs srcs[0]=handle".into())
        })?;
        load_operand(asm, alloc, Reg::Rax, h_src);
        asm.mov_r64_mem(Reg::Rax, Reg::Rax, 0);
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // `array_get <handle>, <idx> -> <dest>` — bounds-check then load.
    //   rax=base ; rcx=idx ; rdx=[base] (len) ; cmp rcx,rdx ; jb ok ; ud2 ; ok:
    //   shl rcx,3 ; add rax,rcx ; mov rax,[rax+8] ; mov [rbp+dest],rax
    if op == "array_get" {
        let dest = require_dest(instr)?;
        let h_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("array_get: needs srcs[0]=handle".into())
        })?;
        let i_src = instr.srcs.get(1).ok_or_else(|| {
            BackendError::MalformedInstr("array_get: needs srcs[1]=idx".into())
        })?;
        native_array_elem_size(&instr.ty)?; // validate 8-byte element (i64/u64/f64)
        load_operand(asm, alloc, Reg::Rax, h_src);
        load_operand(asm, alloc, Reg::Rcx, i_src);
        asm.mov_r64_mem(Reg::Rdx, Reg::Rax, 0); // length
        asm.cmp(Reg::Rcx, Reg::Rdx); // idx - len
        let ok = asm.create_label();
        asm.jcc(Cond::B, ok); // idx <u len → in bounds, skip trap
        asm.ud2();
        asm.bind(ok).map_err(BackendError::from)?;
        asm.shl_imm8(Reg::Rcx, 3); // idx*8
        asm.add(Reg::Rax, Reg::Rcx); // base + idx*8
        asm.mov_r64_mem(Reg::Rax, Reg::Rax, 8); // element past the 8-byte header
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // `array_set <handle>, <idx>, <val>` (no dest) — bounds-check, store.
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
        native_array_elem_size(&instr.ty)?; // validate 8-byte element (i64/u64/f64)
        load_operand(asm, alloc, Reg::Rax, h_src);
        load_operand(asm, alloc, Reg::Rcx, i_src);
        asm.mov_r64_mem(Reg::Rdx, Reg::Rax, 0); // length
        asm.cmp(Reg::Rcx, Reg::Rdx);
        let ok = asm.create_label();
        asm.jcc(Cond::B, ok);
        asm.ud2();
        asm.bind(ok).map_err(BackendError::from)?;
        asm.shl_imm8(Reg::Rcx, 3); // idx*8
        asm.add(Reg::Rax, Reg::Rcx); // base + idx*8
        load_operand(asm, alloc, Reg::Rdx, v_src);
        asm.mov_mem_r64(Reg::Rax, 8, Reg::Rdx); // store past the header
        return Ok(());
    }

    // ---- Heap cons cells (lispy `ref<LispyPair>`) — L3b -------------------
    //
    // `iir_builtin_lowering::lower_heap_builtins` rewrites a Lisp frontend's
    // `call_builtin "cons"/"car"/"cdr"/"null?"` into these word-granular heap
    // ops.  A pair is a 2-word (16-byte) cell: field 0 = car/head, field 1 =
    // cdr/tail.  We allocate it with `__twig_alloc_bytes` (the helper
    // `alloc_bytes` uses) and read/write fields with plain 64-bit moves at
    // byte displacement `idx*8`.  Values are raw 64-bit words — no NaN-boxing
    // — so `(CAR (CONS 7 9))` round-trips to a raw `7`.  (V1 leaks; no GC.)

    // `alloc -> <dest>` — a fresh 2-word LispyPair cell (the record/union
    // constructor cell), always a pair of **boxed `any` fields** (§ emit_record_def
    // types its params `any`). Allocate it under the MOVABLE `{0,8}` pair kind via
    // `__twig_gc_alloc_pair` so the compacting collector traces both reference
    // fields precisely and relocates the record.
    //
    // This also **fixes a latent x86_64 soundness bug**: the cell used to be
    // allocated via `__twig_alloc_bytes`, which (since twig-aot 0.48.0 routed it
    // through gc-core as a *no-reference* blob kind for strings) scans **none** of
    // its bytes — so a child object referenced only through a record field was
    // untraced and could be reclaimed out from under a live record. A precise
    // `{0,8}` kind traces exactly the two fields, matching aarch64's behaviour.
    // An explicit non-pair size (currently unused on this path) falls back to the
    // conservative kind-0 `__twig_gc_alloc`, which traces its bytes as maybe-refs.
    if op == "alloc" {
        let dest = require_dest(instr)?;
        let explicit_size: Option<i64> = match instr.srcs.first() {
            Some(CIROperand::Int(n)) if *n > 0 => Some(*n),
            _ => None,
        };
        match explicit_size {
            None | Some(16) => {
                asm.call_rel32("__twig_gc_alloc_pair", ExternalRelocKind::PltRel32);
            }
            Some(size_bytes) => {
                asm.mov_r64_imm32(abi.arg_regs()[0], size_bytes as i32);
                asm.call_rel32("__twig_gc_alloc", ExternalRelocKind::PltRel32);
            }
        }
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
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
        let disp = field_disp(instr, 1)?;
        let val_src = instr.srcs.get(2).ok_or_else(|| {
            BackendError::MalformedInstr("field_store: needs srcs[2]=value".into())
        })?;
        load_operand(asm, alloc, Reg::Rax, ptr_src);
        load_operand(asm, alloc, Reg::Rcx, val_src);
        asm.mov_mem_r64(Reg::Rax, disp, Reg::Rcx);
        return Ok(());
    }

    // `field_load <ptr>, <idx> -> <dest>` — `dest = [ptr + idx*8]`.
    if op == "field_load" {
        let dest = require_dest(instr)?;
        let ptr_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("field_load: needs srcs[0]=ptr".into())
        })?;
        let disp = field_disp(instr, 1)?;
        load_operand(asm, alloc, Reg::Rax, ptr_src);
        asm.mov_r64_mem(Reg::Rax, Reg::Rax, disp);
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // `is_null <x> -> <dest>` — `dest = (x == 0)` (nil is the 0 word).
    if op == "is_null" {
        let dest = require_dest(instr)?;
        let x_src = instr.srcs.first().ok_or_else(|| {
            BackendError::MalformedInstr("is_null: needs srcs[0]".into())
        })?;
        load_operand(asm, alloc, Reg::Rax, x_src);
        asm.cmp_imm32(Reg::Rax, 0);
        asm.setcc(Cond::E, Reg::Rax);
        asm.movzx_r64_r8(Reg::Rax, Reg::Rax);
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // --- LANG38-parity additions ---

    // div_<ty> / mod_<ty> — signed types use IDIV, unsigned use DIV.
    if let Some(ty) = op.strip_prefix("div_") { return emit_divmod(asm, alloc, instr, ty, false); }
    if let Some(ty) = op.strip_prefix("mod_") { return emit_divmod(asm, alloc, instr, ty, true);  }

    // and_<ty> / or_<ty> / xor_<ty>
    if let Some(ty) = op.strip_prefix("and_") { return emit_bitwise(asm, alloc, instr, Bitwise::And, ty); }
    if let Some(ty) = op.strip_prefix("or_")  { return emit_bitwise(asm, alloc, instr, Bitwise::Or,  ty); }
    if let Some(ty) = op.strip_prefix("xor_") { return emit_bitwise(asm, alloc, instr, Bitwise::Xor, ty); }

    // shl_<ty>: logical shift left (same for signed/unsigned).
    if let Some(ty) = op.strip_prefix("shl_") { return emit_shift(asm, alloc, instr, ShiftKind::Shl, ty); }
    // shr_<ty>: arithmetic for signed (SAR), logical for unsigned (SHR).
    if let Some(ty) = op.strip_prefix("shr_") {
        let kind = if ty.starts_with('i') { ShiftKind::Sar } else { ShiftKind::Shr };
        return emit_shift(asm, alloc, instr, kind, ty);
    }

    // neg_<ty> dest = -src
    if let Some(ty) = op.strip_prefix("neg_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::Rax, src);
        asm.neg_(Reg::Rax);
        mask_narrow(asm, Reg::Rax, ty); // E2: -x mod 2ⁿ for narrow widths
        let slot = alloc.slot_of(dest);
        asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
        return Ok(());
    }

    // not_<ty> dest = ~src
    if let Some(ty) = op.strip_prefix("not_") {
        let dest = require_dest(instr)?;
        let src = instr.srcs.first()
            .ok_or_else(|| BackendError::MalformedInstr(format!("{op} needs srcs[0]")))?;
        load_operand(asm, alloc, Reg::Rax, src);
        asm.not_(Reg::Rax);
        mask_narrow(asm, Reg::Rax, ty); // E2: ~x flips only the low n bits for a uⁿ
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
    if ty == "f64" {
        // ALGOL `real` division (LANG-FULL E3). `mod` is integer-only.
        if is_mod {
            return Err(BackendError::UnsupportedOp("mod_f64 (real modulo is undefined)".into()));
        }
        return emit_fp_binop(asm, alloc, instr, FpBin::Div);
    }
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
    // E2: quotient/remainder of in-range uⁿ operands already fits, so this mask
    // is a no-op; kept uniform with the other narrow ops.
    mask_narrow(asm, result_reg, ty);
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
    ty: &str,
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
    // E2: AND/OR/XOR of two already-masked uⁿ operands stays in range, so this
    // mask is provably redundant — kept uniform with the other narrow ops.
    mask_narrow(asm, Reg::Rax, ty);
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
    ty: &str,
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
    // E2: a left shift can push bits above the declared width (`1u8 << 8` must
    // be 0, not 256), so mask the result.  Right shifts only shrink the value,
    // so the mask is a no-op there — applied uniformly for simplicity.
    mask_narrow(asm, Reg::Rax, ty);
    let slot = alloc.slot_of(dest);
    asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
    Ok(())
}

// ===========================================================================
// Binary integer ops
// ===========================================================================

#[derive(Debug, Clone, Copy)]
enum BinOp { Add, Sub, Mul }

/// LANG-FULL E2 (native-AOT leg): the bit-width of a narrow **unsigned** type,
/// or `None` for full-width / signed / non-integer types.  See [`mask_narrow`].
fn narrow_unsigned_bits(ty: &str) -> Option<u32> {
    match ty {
        "u4"  => Some(4),
        "u8"  => Some(8),
        "u16" => Some(16),
        "u32" => Some(32),
        _ => None, // u64 / i* / f* / bool / void — no masking
    }
}

/// Mask the value in `reg` down to `ty`'s width when `ty` is a narrow unsigned
/// type, so narrow arithmetic *wraps* mod-2ⁿ instead of keeping the full 64-bit
/// result (`add_u8 200, 100` → 44, not 300).  Mirrors the masks the other
/// backends (vm-core, jit-core, wasm, jvm, cil) already emit.  `RCX` is the
/// scratch register for the mask constant; the stack-spill allocator keeps every
/// live value in a stack slot, so `RCX` is free between instructions.  `reg` is
/// always `RAX` or `RDX` here (never `RCX`).  A no-op for full-width / signed /
/// non-integer types.
fn mask_narrow(asm: &mut Assembler, reg: Reg, ty: &str) {
    if let Some(bits) = narrow_unsigned_bits(ty) {
        let mask = (1u64 << bits) - 1;
        asm.mov_r64_imm64(Reg::Rcx, mask);
        asm.and_(reg, Reg::Rcx);
    }
}

fn emit_binop(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    op: BinOp,
    ty: &str,
) -> Result<(), BackendError> {
    if ty == "f64" {
        let fk = match op {
            BinOp::Add => FpBin::Add,
            BinOp::Sub => FpBin::Sub,
            BinOp::Mul => FpBin::Mul,
        };
        return emit_fp_binop(asm, alloc, instr, fk);
    }
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
    mask_narrow(asm, Reg::Rax, ty); // E2: wrap u8/u16/u32 results mod 2ⁿ
    let slot = alloc.slot_of(dest);
    asm.mov_mem_r64(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
    Ok(())
}

/// Which floating-point binary op (LANG-FULL E3).
#[derive(Debug, Clone, Copy)]
enum FpBin { Add, Sub, Mul, Div }

/// Load an `f64` operand into an XMM register (`Reg`'s number names the XMM
/// slot: `Rax`→`xmm0`, `Rcx`→`xmm1`). Frontends materialise constants into
/// stack slots first, so an arithmetic/compare operand is a `Var`.
fn load_fp_operand(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    xmm: Reg,
    op: &CIROperand,
) -> Result<(), BackendError> {
    match op {
        CIROperand::Var(name) => {
            let slot = alloc.slot_of(name);
            asm.movsd_load(xmm, Reg::Rbp, RegAlloc::rbp_offset(slot));
            Ok(())
        }
        other => Err(BackendError::UnsupportedOp(format!(
            "f64 operand must be a Var (materialise the constant first), got {other:?}"
        ))),
    }
}

/// Emit an `f64` `add`/`sub`/`mul`/`div`: `movsd xmm0,[a]; movsd xmm1,[b];
/// <op>sd xmm0,xmm1; movsd [dest],xmm0` (LANG-FULL E3). IEEE division by zero is
/// `±inf`/`NaN` (no trap), matching every other backend.
fn emit_fp_binop(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    kind: FpBin,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    let lhs = instr.srcs.first()
        .ok_or_else(|| BackendError::MalformedInstr("f64 binop needs srcs[0]".into()))?;
    let rhs = instr.srcs.get(1)
        .ok_or_else(|| BackendError::MalformedInstr("f64 binop needs srcs[1]".into()))?;
    load_fp_operand(asm, alloc, Reg::Rax, lhs)?; // xmm0
    load_fp_operand(asm, alloc, Reg::Rcx, rhs)?; // xmm1
    match kind {
        FpBin::Add => asm.addsd(Reg::Rax, Reg::Rcx),
        FpBin::Sub => asm.subsd(Reg::Rax, Reg::Rcx),
        FpBin::Mul => asm.mulsd(Reg::Rax, Reg::Rcx),
        FpBin::Div => asm.divsd(Reg::Rax, Reg::Rcx),
    }
    let slot = alloc.slot_of(dest);
    asm.movsd_store(Reg::Rbp, RegAlloc::rbp_offset(slot), Reg::Rax);
    Ok(())
}

/// Emit an `f64` comparison via `ucomisd` + `setcc` (LANG-FULL E3). The boolean
/// result is an `int` 0/1. `ucomisd` sets ZF/PF/CF like an *unsigned* compare,
/// and a NaN operand sets PF (and ZF, CF). We pick operand order + condition so
/// every relation has **IEEE-ordered** semantics (a NaN makes `<`/`<=`/`>`/`>=`
/// and `==` false; `!=` true):
///
/// | rel | `ucomisd` | setcc | why                                          |
/// |-----|-----------|-------|----------------------------------------------|
/// | `<` | `b, a`    | `A`   | ordered `b > a` ⇒ `a < b`; NaN → CF/ZF set → false |
/// | `<=`| `b, a`    | `AE`  | ordered `b >= a`; NaN → CF set → false       |
/// | `>` | `a, b`    | `A`   | ordered `a > b`                              |
/// | `>=`| `a, b`    | `AE`  | ordered `a >= b`                             |
/// | `==`| `a, b`    | `E && NP` | ZF=1 (equal **or** NaN) **and** PF=0 (not NaN) |
/// | `!=`| `a, b`    | `NE || P` | ZF=0 **or** PF=1 (NaN)                    |
fn emit_fp_cmp(
    asm: &mut Assembler,
    alloc: &mut RegAlloc,
    instr: &CIRInstr,
    rel: CmpRel,
) -> Result<(), BackendError> {
    let dest = require_dest(instr)?;
    let lhs = instr.srcs.first()
        .ok_or_else(|| BackendError::MalformedInstr("f64 cmp needs srcs[0]".into()))?;
    let rhs = instr.srcs.get(1)
        .ok_or_else(|| BackendError::MalformedInstr("f64 cmp needs srcs[1]".into()))?;
    // Load a→xmm0, b→xmm1.
    load_fp_operand(asm, alloc, Reg::Rax, lhs)?; // xmm0 = a
    load_fp_operand(asm, alloc, Reg::Rcx, rhs)?; // xmm1 = b

    match rel {
        CmpRel::Lt | CmpRel::Le => {
            // Compare b to a (reversed), so a single `seta`/`setae` is ordered.
            asm.ucomisd(Reg::Rcx, Reg::Rax); // ucomisd xmm1(b), xmm0(a)
            asm.setcc(if rel == CmpRel::Lt { Cond::A } else { Cond::Ae }, Reg::Rax);
            asm.movzx_r64_r8(Reg::Rax, Reg::Rax);
        }
        CmpRel::Gt | CmpRel::Ge => {
            asm.ucomisd(Reg::Rax, Reg::Rcx); // ucomisd xmm0(a), xmm1(b)
            asm.setcc(if rel == CmpRel::Gt { Cond::A } else { Cond::Ae }, Reg::Rax);
            asm.movzx_r64_r8(Reg::Rax, Reg::Rax);
        }
        CmpRel::Eq => {
            // ZF=1 AND PF=0 → equal and ordered. `sete`(Rax) & `setnp`(Rcx).
            asm.ucomisd(Reg::Rax, Reg::Rcx);
            asm.setcc(Cond::E, Reg::Rax);
            asm.movzx_r64_r8(Reg::Rax, Reg::Rax);
            asm.setcc(Cond::Np, Reg::Rcx);
            asm.movzx_r64_r8(Reg::Rcx, Reg::Rcx);
            asm.and_(Reg::Rax, Reg::Rcx);
        }
        CmpRel::Ne => {
            // ZF=0 OR PF=1 → not-equal or unordered. `setne` | `setp`.
            asm.ucomisd(Reg::Rax, Reg::Rcx);
            asm.setcc(Cond::Ne, Reg::Rax);
            asm.movzx_r64_r8(Reg::Rax, Reg::Rax);
            asm.setcc(Cond::P, Reg::Rcx);
            asm.movzx_r64_r8(Reg::Rcx, Reg::Rcx);
            asm.or_(Reg::Rax, Reg::Rcx);
        }
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

/// The byte size of an E5 array element type on the native backend. 64-bit
/// integer and `f64` elements share the same 8-byte memory representation here:
/// the backend copies raw bits between stack slots and array storage, while f64
/// arithmetic/comparisons load those bits through SSE when needed. Smaller
/// element widths still produce a clear error rather than a silently wrong
/// stride.
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

/// Largest field byte-displacement the heap ops accept.  Bounds the offset
/// **in the backend** rather than relying on the lowering pass only ever
/// emitting field index 0/1, so a future producer with a larger index gets
/// a clean `MalformedInstr`, never a wrapped/oversized displacement.
const MAX_FIELD_DISP: u64 = 0x7FF8;

/// Read a `field_load`/`field_store` field-index operand (a compile-time
/// `Int`) and convert it to a byte displacement (`idx * 8`).  Pair fields
/// are word-sized: index 0 → disp 0 (car), index 1 → disp 8 (cdr).  A
/// negative, non-literal, or out-of-range index is a `MalformedInstr`.
fn field_disp(instr: &CIRInstr, i: usize) -> Result<i32, BackendError> {
    match instr.srcs.get(i) {
        Some(CIROperand::Int(n)) if *n >= 0 => (*n as u64)
            .checked_mul(8)
            .filter(|off| *off <= MAX_FIELD_DISP)
            .map(|off| off as i32)
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

    // LANG-FULL E5 — bounds-checked arrays lower to a `__twig_alloc_bytes` call,
    // an explicit `cmp`+`jb`+`ud2` bounds trap, and base+idx*8 loads/stores.
    #[test]
    fn array_ops_lower_with_bounds_trap() {
        let ctx = fn_ctx("arr", &[], "u64");
        let ir = vec![
            instr("const_u64", Some("n"), vec![Op::Int(3)]),
            instr("alloc_array", Some("a"), vec![Op::Var("n".into())]),
            instr("const_u64", Some("i"), vec![Op::Int(0)]),
            instr("const_u64", Some("v"), vec![Op::Int(42)]),
            instr("array_set", None, vec![Op::Var("a".into()), Op::Var("i".into()), Op::Var("v".into())]),
            instr("array_get", Some("r"), vec![Op::Var("a".into()), Op::Var("i".into())]),
            instr("array_len", Some("m"), vec![Op::Var("a".into())]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV)
            .expect("array ops must lower");
        // Two bounds checks (array_get + array_set) ⇒ at least two `ud2` (0F 0B) traps.
        let traps = bytes.windows(2).filter(|w| *w == [0x0F, 0x0B]).count();
        assert!(traps >= 2, "expected ≥2 ud2 bounds traps, got {traps} in {bytes:02X?}");
    }

    /// `f64` array elements lower as raw 8-byte loads/stores; f64 math reads
    /// the same bits from the destination slot through SSE later.
    #[test]
    fn array_get_accepts_f64_element() {
        let ctx = fn_ctx("arr", &[], "u64");
        let mut get = instr("array_get", Some("r"),
            vec![Op::Var("a".into()), Op::Var("i".into())]);
        get.ty = "f64".to_string();
        let ir = vec![
            instr("const_u64", Some("n"), vec![Op::Int(1)]),
            instr("alloc_array", Some("a"), vec![Op::Var("n".into())]),
            instr("const_u64", Some("i"), vec![Op::Int(0)]),
            get,
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        assert!(compile_function(&ctx, &ir, X86_64Abi::SysV).is_ok(),
            "f64 array element should lower as an 8-byte native load");
    }

    // ---- L3b: cons heap ops (alloc / field_store / field_load / is_null) ----

    #[test]
    fn cons_car_heap_ops_lower() {
        // CIR for `(CAR (CONS 7 9))`: allocate a 2-word cell, store 7/9 into
        // fields 0/1, load field 0 back.
        let ir = vec![
            instr("const_u64", Some("h"), vec![Op::Int(7)]),
            instr("const_u64", Some("t"), vec![Op::Int(9)]),
            instr("alloc", Some("cell"), vec![]),
            instr("field_store", None, vec![Op::Var("cell".into()), Op::Int(0), Op::Var("h".into())]),
            instr("field_store", None, vec![Op::Var("cell".into()), Op::Int(1), Op::Var("t".into())]),
            instr("field_load", Some("r"), vec![Op::Var("cell".into()), Op::Int(0)]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let bytes = compile_function(&fn_ctx("cons_car", &[], "u64"), &ir, X86_64Abi::SysV)
            .expect("cons/car heap ops must lower");
        assert!(!bytes.is_empty());
    }

    // ---- L3b-2b (LANG77): cons/car via the runtime-call path ----

    fn call_builtin(dest: Option<&str>, name: &str, args: &[&str]) -> CIRInstr {
        let mut srcs = vec![Op::Var(name.into())];
        srcs.extend(args.iter().map(|a| Op::Var((*a).into())));
        instr("call_builtin", dest, srcs)
    }

    // ---- AOT00-T1: GC precise-roots stack-map emission ----

    /// Set an instruction's result type (for marking a dest as a GC reference).
    fn typed(mut i: CIRInstr, ty: &str) -> CIRInstr {
        i.ty = ty.to_string();
        i
    }

    /// A function that produces an `any` value and then makes a call gets a stack map
    /// whose call-return record names the `any` slot as a root — at the RBP-relative
    /// (negative) offset the register allocator assigned it.
    #[test]
    fn stackmap_names_ref_slot_at_call_site() {
        // b = dyn_cons(h, t)  [any]  ; then a second call (dyn_car) as a safepoint.
        let ir = vec![
            instr("const_u64", Some("h"), vec![Op::Int(1)]),
            instr("const_u64", Some("t"), vec![Op::Int(2)]),
            typed(call_builtin(Some("b"), "dyn_cons", &["h", "t"]), "any"),
            call_builtin(Some("r"), "dyn_car", &["b"]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let (_bytes, relocs, sm) = compile_function_with_globals_and_stackmap(
            &fn_ctx("cons", &[], "u64"),
            &ir,
            X86_64Abi::SysV,
            &HashMap::new(),
        )
        .expect("compiles");

        // Two external calls → two safepoint records.
        assert_eq!(sm.len(), 2, "one record per call return address");
        // Each record names exactly the one `any` slot (`b`); the `u64` slots (h/t/r)
        // are excluded. `b` is the third slot minted (h=0, t=1, b=2) → rbp offset −24.
        for rec in &sm {
            assert_eq!(rec.slots, vec![RegAlloc::rbp_offset(2)], "names only the `any` slot");
        }
        // Each record's pc_offset is a real call-return address = a PltRel32 reloc's
        // patch_offset + 4.
        let call_rets: Vec<u32> = relocs
            .iter()
            .filter(|r| r.kind == ExternalRelocKind::PltRel32)
            .map(|r| (r.patch_offset + 4) as u32)
            .collect();
        for rec in &sm {
            assert!(call_rets.contains(&rec.pc_offset), "pc_offset is a call return address");
        }
    }

    /// An `any`-typed parameter is a root at every safepoint (it is spilled to its slot
    /// by the prologue and lives across calls).
    #[test]
    fn stackmap_names_ref_param() {
        let params = [("obj".to_string(), "any".to_string())];
        let ir = vec![
            call_builtin(Some("r"), "dyn_car", &["obj"]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let (_b, _r, sm) = compile_function_with_globals_and_stackmap(
            &fn_ctx("head", &params, "u64"),
            &ir,
            X86_64Abi::SysV,
            &HashMap::new(),
        )
        .expect("compiles");
        assert_eq!(sm.len(), 1);
        // `obj` is param slot 0 → rbp offset −8.
        assert_eq!(sm[0].slots, vec![RegAlloc::rbp_offset(0)]);
    }

    /// A function whose slots are all machine scalars produces records with **empty**
    /// slot lists at its call sites — nothing to root, but the safepoint still exists.
    /// (`cell` is a `dyn_cons` result but typed `u64` here, so it is a non-reference
    /// look-alike — excluded from the map exactly as a real integer slot would be.)
    #[test]
    fn stackmap_empty_for_ref_free_function() {
        let ir = vec![
            instr("const_u64", Some("h"), vec![Op::Int(1)]),
            instr("const_u64", Some("t"), vec![Op::Int(2)]),
            call_builtin(Some("cell"), "dyn_cons", &["h", "t"]), // dest defaults to u64
            instr("ret_u64", None, vec![Op::Var("cell".into())]),
        ];
        let (_b, _r, sm) = compile_function_with_globals_and_stackmap(
            &fn_ctx("nore", &[], "u64"),
            &ir,
            X86_64Abi::SysV,
            &HashMap::new(),
        )
        .expect("compiles");
        assert_eq!(sm.len(), 1, "one call → one record");
        assert!(sm[0].slots.is_empty(), "no reference slots to root");
    }

    /// AOT00-T1 §5 (PR-x4) — a **self-recursive** call is a safepoint too.
    ///
    /// A purely self-recursive function makes *no* external call, so it carries zero
    /// `PltRel32` relocations — before this fix its stack map was therefore empty and a
    /// collection fired inside the recursion fell back to a conservative frame scan. The
    /// recursive call's return address is now recovered at emit time (`call_label` carries
    /// no reloc), so the map gains exactly one record naming the live `any` reference.
    #[test]
    fn stackmap_records_self_recursive_call_safepoint() {
        // rec(acc: any) -> u64  { r = rec(acc); ret r }  — `acc` is live across the call.
        let params = [("acc".to_string(), "any".to_string())];
        let ir = vec![
            instr("call", Some("r"), vec![Op::Var("rec".into()), Op::Var("acc".into())]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let (bytes, relocs, sm) = compile_function_with_globals_and_stackmap(
            &fn_ctx("rec", &params, "u64"),
            &ir,
            X86_64Abi::SysV,
            &HashMap::new(),
        )
        .expect("compiles");

        // No external call ⇒ no PltRel32 reloc: the safepoint cannot have come from a
        // relocation — it can only be the recursive one this PR adds.
        assert!(
            relocs.iter().all(|r| r.kind != ExternalRelocKind::PltRel32),
            "a self-recursive-only function makes no external call",
        );
        assert_eq!(sm.len(), 1, "the recursive call is now the one safepoint");
        // `acc` is param slot 0 → rbp offset −8; live across the recursive call.
        assert_eq!(sm[0].slots, vec![RegAlloc::rbp_offset(0)], "names the live `any` ref");

        // The safepoint PC is the return address: the byte just after the 5-byte
        // `call rel32` (opcode 0xE8). This minimal function emits exactly one 0xE8 (the
        // call disp is negative — 0xFFFF_FFxx — so no stray 0xE8 in the operand bytes).
        let e8_positions: Vec<usize> =
            bytes.iter().enumerate().filter(|(_, &b)| b == 0xE8).map(|(i, _)| i).collect();
        assert_eq!(e8_positions.len(), 1, "one `call rel32` in a purely recursive body");
        assert_eq!(
            sm[0].pc_offset as usize,
            e8_positions[0] + 5,
            "safepoint PC is the return address (byte after the 5-byte call)",
        );
    }

    /// A function with **both** a self-recursive call and an external (builtin) call gets a
    /// safepoint record for *each* — the two safepoint sources (relocations + recorded
    /// recursive return addresses) compose, and both records name the live reference.
    #[test]
    fn stackmap_records_both_recursive_and_external_safepoints() {
        // rec(acc: any) -> u64 { r1 = dyn_car(acc); r2 = rec(acc); ret r2 }
        let params = [("acc".to_string(), "any".to_string())];
        let ir = vec![
            call_builtin(Some("r1"), "dyn_car", &["acc"]), // external → PltRel32 safepoint
            instr("call", Some("r2"), vec![Op::Var("rec".into()), Op::Var("acc".into())]),
            instr("ret_u64", None, vec![Op::Var("r2".into())]),
        ];
        let (_bytes, relocs, sm) = compile_function_with_globals_and_stackmap(
            &fn_ctx("rec", &params, "u64"),
            &ir,
            X86_64Abi::SysV,
            &HashMap::new(),
        )
        .expect("compiles");

        // Exactly one external call → one PltRel32 return address.
        let plt_rets: Vec<u32> = relocs
            .iter()
            .filter(|r| r.kind == ExternalRelocKind::PltRel32)
            .map(|r| (r.patch_offset + 4) as u32)
            .collect();
        assert_eq!(plt_rets.len(), 1, "one builtin call");

        // Two distinct safepoints, ascending, both naming the live `any` slot.
        assert_eq!(sm.len(), 2, "recursive + external = two safepoints");
        assert!(sm[0].pc_offset < sm[1].pc_offset, "records are ascending by PC");
        for rec in &sm {
            assert_eq!(rec.slots, vec![RegAlloc::rbp_offset(0)], "acc rooted at both");
        }
        // One record is the builtin's return address; the other is the recursive one.
        let pcs: Vec<u32> = sm.iter().map(|r| r.pc_offset).collect();
        assert!(pcs.contains(&plt_rets[0]), "one safepoint is the builtin return address");
        assert!(
            pcs.iter().any(|&p| p != plt_rets[0]),
            "the other safepoint is the recursive call (no reloc)",
        );
    }

    /// Deriving the stack map must not change the emitted machine code — the map is
    /// read off the same compilation, not injected into it.
    #[test]
    fn stackmap_does_not_change_emitted_code() {
        let ir = vec![
            instr("const_u64", Some("h"), vec![Op::Int(1)]),
            instr("const_u64", Some("t"), vec![Op::Int(2)]),
            typed(call_builtin(Some("b"), "dyn_cons", &["h", "t"]), "any"),
            instr("ret_u64", None, vec![Op::Var("b".into())]),
        ];
        let plain = compile_function_with_globals(
            &fn_ctx("cons", &[], "u64"), &ir, X86_64Abi::SysV, &HashMap::new(),
        )
        .expect("compiles");
        let (bytes, _r, _sm) = compile_function_with_globals_and_stackmap(
            &fn_ctx("cons", &[], "u64"), &ir, X86_64Abi::SysV, &HashMap::new(),
        )
        .expect("compiles");
        assert_eq!(plain.0, bytes, "stack-map derivation is byte-for-byte transparent");
    }

    #[test]
    fn dynval_runtime_cons_car_emit_external_calls() {
        // `(CAR (CONS 7 9))` through the runtime path: two CALLs to the C
        // lisp runtime, surfaced as external relocations.
        let ir = vec![
            instr("const_u64", Some("h"), vec![Op::Int(7)]),
            instr("const_u64", Some("t"), vec![Op::Int(9)]),
            call_builtin(Some("cell"), "dyn_cons", &["h", "t"]),
            call_builtin(Some("r"), "dyn_car", &["cell"]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let (bytes, relocs) =
            compile_function_with_relocs(&fn_ctx("lispy", &[], "u64"), &ir, X86_64Abi::SysV)
                .expect("lispy runtime calls must lower");
        assert!(!bytes.is_empty());
        let symbols: Vec<&str> = relocs.iter().map(|r| r.symbol.as_str()).collect();
        assert!(symbols.contains(&"__dyn_cons"), "missing cons call: {symbols:?}");
        assert!(symbols.contains(&"__dyn_car"), "missing car call: {symbols:?}");
    }

    /// `call_builtin "gc_collect_compacting" -> freed` lowers to a `call` to the linker
    /// symbol `__twig_gc_collect_compacting` (the moving-collector C-ABI entry, spec
    /// AOT00-T3 §5), via the generic `__twig_<name>` builtin dispatch — the same path
    /// `gc_collect_precise` uses. Proves a native frontend can trigger a compaction.
    #[test]
    fn gc_collect_compacting_emits_external_twig_call() {
        let ir = vec![
            call_builtin(Some("freed"), "gc_collect_compacting", &[]),
            instr("ret_u64", None, vec![Op::Var("freed".into())]),
        ];
        let (bytes, relocs) =
            compile_function_with_relocs(&fn_ctx("gc_compact", &[], "u64"), &ir, X86_64Abi::SysV)
                .expect("gc_collect_compacting must lower");
        assert!(!bytes.is_empty());
        let symbols: Vec<&str> = relocs.iter().map(|r| r.symbol.as_str()).collect();
        assert!(
            symbols.contains(&"__twig_gc_collect_compacting"),
            "missing compacting-collect call: {symbols:?}",
        );
    }

    /// The incremental-collector builtin trio (`gc_collect_incremental_{start,step,finish}`,
    /// spec AOT00-T4 §6) lowers each to a `call` to its `__twig_gc_collect_incremental_*`
    /// linker symbol via the generic `__twig_<name>` dispatch — `step` takes a budget arg.
    #[test]
    fn gc_collect_incremental_emits_external_twig_calls() {
        let ir = vec![
            call_builtin(None, "gc_collect_incremental_start", &[]),
            instr("const_u64", Some("budget"), vec![Op::Int(1_000_000)]),
            call_builtin(Some("done"), "gc_collect_incremental_step", &["budget"]),
            call_builtin(Some("freed"), "gc_collect_incremental_finish", &[]),
            instr("ret_u64", None, vec![Op::Var("freed".into())]),
        ];
        let (bytes, relocs) =
            compile_function_with_relocs(&fn_ctx("gc_incr", &[], "u64"), &ir, X86_64Abi::SysV)
                .expect("gc_collect_incremental_* must lower");
        assert!(!bytes.is_empty());
        let symbols: Vec<&str> = relocs.iter().map(|r| r.symbol.as_str()).collect();
        for want in [
            "__twig_gc_collect_incremental_start",
            "__twig_gc_collect_incremental_step",
            "__twig_gc_collect_incremental_finish",
        ] {
            assert!(symbols.contains(&want), "missing {want}: {symbols:?}");
        }
    }

    /// `call_builtin "gc_register_ref_array_kind" (fixed, fixed_count, tail_from) -> kind`
    /// lowers to a `call` to `__twig_gc_register_ref_array_kind` (the C-ABI seam a language
    /// frontend's array type calls, spec AOT00-T5) via the generic `__twig_<name>` dispatch —
    /// three args in rdi/rsi/rdx. `(0, 0, 0)` declares a pure reference array.
    #[test]
    fn gc_register_ref_array_kind_emits_external_twig_call() {
        let ir = vec![
            instr("const_u64", Some("fixed"), vec![Op::Int(0)]), // null fixed-offsets pointer
            instr("const_u64", Some("fcount"), vec![Op::Int(0)]), // no fixed ref fields
            instr("const_u64", Some("tail"), vec![Op::Int(0)]), // tail from offset 0 (all refs)
            call_builtin(Some("kind"), "gc_register_ref_array_kind", &["fixed", "fcount", "tail"]),
            instr("ret_u64", None, vec![Op::Var("kind".into())]),
        ];
        let (bytes, relocs) =
            compile_function_with_relocs(&fn_ctx("gc_refarray", &[], "u64"), &ir, X86_64Abi::SysV)
                .expect("gc_register_ref_array_kind must lower");
        assert!(!bytes.is_empty());
        let symbols: Vec<&str> = relocs.iter().map(|r| r.symbol.as_str()).collect();
        assert!(
            symbols.contains(&"__twig_gc_register_ref_array_kind"),
            "missing ref-array-kind registration call: {symbols:?}",
        );
    }

    #[test]
    fn dyn_cons_wrong_arity_is_rejected() {
        // dyn_cons takes exactly 2 args; one arg is a soft refusal.
        let ir = vec![
            instr("const_u64", Some("h"), vec![Op::Int(7)]),
            call_builtin(Some("cell"), "dyn_cons", &["h"]),
            instr("ret_u64", None, vec![Op::Var("cell".into())]),
        ];
        assert!(compile_function(&fn_ctx("bad_cons", &[], "u64"), &ir, X86_64Abi::SysV).is_err());
    }

    #[test]
    fn dyn_full_boxed_cons_car_unbox_lowers() {
        // The complete L3b-2c-1 CIR for `(CAR (CONS 7 9))`: boxed atoms,
        // cons, car, then unbox the result for the exit code.
        let ir = vec![
            instr("const_u64", Some("h"), vec![Op::Int(7 << 3)]),
            instr("const_u64", Some("t"), vec![Op::Int(9 << 3)]),
            call_builtin(Some("cell"), "dyn_cons", &["h", "t"]),
            call_builtin(Some("boxed"), "dyn_car", &["cell"]),
            call_builtin(Some("r"), "dyn_unbox_int", &["boxed"]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let (bytes, relocs) =
            compile_function_with_relocs(&fn_ctx("full", &[], "u64"), &ir, X86_64Abi::SysV)
                .expect("boxed cons/car/unbox must lower");
        assert!(!bytes.is_empty());
        let symbols: Vec<&str> = relocs.iter().map(|r| r.symbol.as_str()).collect();
        for want in ["__dyn_cons", "__dyn_car", "__dyn_unbox_int"] {
            assert!(symbols.contains(&want), "missing {want}: {symbols:?}");
        }
    }

    /// A default (2-word pair) `alloc` — the record/union constructor cell — lowers
    /// to a `CALL __twig_gc_alloc_pair` (the MOVABLE `{0,8}` allocator), NOT the old
    /// `__twig_alloc_bytes` (a no-reference blob kind since twig-aot 0.48.0, which
    /// would leave the record's reference fields untraced — a use-after-free for a
    /// child held only via a record field).
    #[test]
    fn pair_alloc_uses_movable_pair_allocator() {
        let ir = vec![
            instr("alloc", Some("cell"), vec![]),
            instr("field_store", None, vec![Op::Var("cell".into()), Op::Int(0), Op::Int(0)]),
            instr("ret_u64", None, vec![Op::Var("cell".into())]),
        ];
        let (_bytes, relocs) =
            compile_function_with_relocs(&fn_ctx("rec", &[], "u64"), &ir, X86_64Abi::SysV)
                .expect("record alloc must lower");
        let symbols: Vec<&str> = relocs.iter().map(|r| r.symbol.as_str()).collect();
        assert!(symbols.contains(&"__twig_gc_alloc_pair"),
                "default-pair alloc must use the movable pair allocator, got {symbols:?}");
        assert!(!symbols.contains(&"__twig_alloc_bytes"),
                "must NOT use the no-ref blob allocator for a record pair: {symbols:?}");
    }

    #[test]
    fn dyn_atom_eq_predicates_and_truthy_lower() {
        // L3b-2c-2: ATOM = not(pair?), normalised via dyn_truthy; plus EQ.
        let ir = vec![
            instr("const_u64", Some("x"), vec![Op::Int(5 << 3)]),
            call_builtin(Some("p"), "dyn_pair_p", &["x"]),
            call_builtin(Some("a"), "dyn_not", &["p"]),
            call_builtin(Some("t"), "dyn_truthy", &["a"]),
            call_builtin(Some("e"), "dyn_equal", &["x", "x"]),
            instr("ret_u64", None, vec![Op::Var("e".into())]),
        ];
        let (bytes, relocs) =
            compile_function_with_relocs(&fn_ctx("preds", &[], "u64"), &ir, X86_64Abi::SysV)
                .expect("predicates must lower");
        assert!(!bytes.is_empty());
        let symbols: Vec<&str> = relocs.iter().map(|r| r.symbol.as_str()).collect();
        for want in [
            "__dyn_pair_p", "__dyn_not",
            "__dyn_truthy", "__dyn_equal",
        ] {
            assert!(symbols.contains(&want), "missing {want}: {symbols:?}");
        }
    }

    /// W14b (F7): the universal exit coercion `dyn_to_exit_code` — the program
    /// boundary for a polymorphic lambda result — lowers to a call into the runtime.
    #[test]
    fn dyn_to_exit_code_lowers() {
        let ir = vec![
            instr("const_u64", Some("x"), vec![Op::Int(5 << 3)]),
            call_builtin(Some("r"), "dyn_to_exit_code", &["x"]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let (bytes, relocs) =
            compile_function_with_relocs(&fn_ctx("exit_coerce", &[], "u64"), &ir, X86_64Abi::SysV)
                .expect("to_exit_code must lower");
        assert!(!bytes.is_empty());
        let symbols: Vec<&str> = relocs.iter().map(|r| r.symbol.as_str()).collect();
        assert!(
            symbols.contains(&"__dyn_to_exit_code"),
            "missing __dyn_to_exit_code: {symbols:?}",
        );
    }

    #[test]
    fn is_null_lowers() {
        let ir = vec![
            instr("const_u64", Some("x"), vec![Op::Int(0)]),
            instr("is_null", Some("r"), vec![Op::Var("x".into())]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let bytes = compile_function(&fn_ctx("isnull", &[], "u64"), &ir, X86_64Abi::SysV)
            .expect("is_null must lower");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn field_store_rejects_dest_and_non_literal_index() {
        let bad_dest = vec![instr("field_store", Some("oops"),
            vec![Op::Var("cell".into()), Op::Int(0), Op::Var("h".into())])];
        assert!(compile_function(&fn_ctx("bad", &[], "u64"), &bad_dest, X86_64Abi::SysV).is_err());
        let bad_idx = vec![instr("field_load", Some("r"),
            vec![Op::Var("cell".into()), Op::Var("i".into())])];
        assert!(compile_function(&fn_ctx("bad2", &[], "u64"), &bad_idx, X86_64Abi::SysV).is_err());
        let huge = vec![instr("field_load", Some("r"),
            vec![Op::Var("cell".into()), Op::Int(1 << 40)])];
        assert!(compile_function(&fn_ctx("bad3", &[], "u64"), &huge, X86_64Abi::SysV).is_err());
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

    // ---- Globals + io_out (phase 6) ----

    #[test]
    fn fn_global_load_emits_lea_and_mov() {
        // CIR: global_load "g0" → v0; ret_u64 v0
        use std::collections::HashMap;
        let mut globals = HashMap::new();
        globals.insert("g0".to_string(), 0usize);
        let ctx = fn_ctx("rd", &[], "u64");
        let ir = vec![
            instr("global_load", Some("v0"), vec![Op::Var("g0".into())]),
            instr("ret_u64", None, vec![Op::Var("v0".into())]),
        ];
        let (bytes, relocs) = compile_function_with_globals(&ctx, &ir, X86_64Abi::SysV, &globals).unwrap();
        // LEA RAX, [RIP+_twig_globals]: 48 8D 05 .. .. .. ..
        assert!(body_contains(&bytes, &[0x48, 0x8D, 0x05]),
                "expected `lea rax, [rip+...]` in {bytes:02X?}");
        // Exactly one PcRel32 reloc on _twig_globals.
        let pcrel: Vec<_> = relocs.iter()
            .filter(|r| r.kind == ExternalRelocKind::PcRel32)
            .collect();
        assert_eq!(pcrel.len(), 1);
        assert_eq!(pcrel[0].symbol, "_twig_globals");
        assert_eq!(pcrel[0].addend, -4);
    }

    #[test]
    fn fn_global_store_emits_lea_and_mov() {
        // CIR: const_u64 v0=42; global_store "g0" = v0; ret_void
        use std::collections::HashMap;
        let mut globals = HashMap::new();
        globals.insert("g0".to_string(), 0usize);
        let ctx = fn_ctx("wr", &[], "void");
        let ir = vec![
            instr("const_u64", Some("v0"), vec![Op::Int(42)]),
            instr("global_store", None,
                  vec![Op::Var("g0".into()), Op::Var("v0".into())]),
            instr("ret_void", None, vec![]),
        ];
        let (bytes, relocs) = compile_function_with_globals(&ctx, &ir, X86_64Abi::SysV, &globals).unwrap();
        assert!(body_contains(&bytes, &[0x48, 0x8D, 0x05]),
                "expected `lea rax, [rip+...]` in {bytes:02X?}");
        let pcrel_count = relocs.iter()
            .filter(|r| r.kind == ExternalRelocKind::PcRel32)
            .count();
        assert_eq!(pcrel_count, 1);
    }

    #[test]
    fn fn_global_load_higher_slot_uses_byte_offset() {
        // global slot 3 should produce a disp32 of 24 in the following MOV.
        use std::collections::HashMap;
        let mut globals = HashMap::new();
        globals.insert("g3".to_string(), 3usize);
        let ctx = fn_ctx("rd", &[], "u64");
        let ir = vec![
            instr("global_load", Some("v0"), vec![Op::Var("g3".into())]),
            instr("ret_u64", None, vec![Op::Var("v0".into())]),
        ];
        let (bytes, _) = compile_function_with_globals(&ctx, &ir, X86_64Abi::SysV, &globals).unwrap();
        // Look for `mov rax, [rax + 24]` — encoded as 48 8B 80 18 00 00 00.
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0x80, 0x18, 0x00, 0x00, 0x00]),
                "expected `mov rax, [rax+24]` in {bytes:02X?}");
    }

    #[test]
    fn fn_io_out_sysv_uses_rdi() {
        // CIR: const_i64 v0 = 99; io_out v0; ret_void
        let ctx = fn_ctx("p", &[], "void");
        let ir = vec![
            instr("const_i64", Some("v0"), vec![Op::Int(99)]),
            instr("io_out", None, vec![Op::Var("v0".into())]),
            instr("ret_void", None, vec![]),
        ];
        let (bytes, relocs) = compile_function_with_relocs(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // System V arg 0 → RDI.  `mov rdi, [rbp+disp32]` = 48 8B BD ..
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0xBD]),
                "expected `mov rdi, [rbp+disp]` for io_out arg on SysV");
        // Then `call __twig_print_i64` (E8) with one PltRel32 reloc.
        let plt: Vec<_> = relocs.iter()
            .filter(|r| r.kind == ExternalRelocKind::PltRel32
                     && r.symbol == "__twig_print_i64")
            .collect();
        assert_eq!(plt.len(), 1);
    }

    #[test]
    fn fn_io_out_msx64_uses_rcx() {
        let ctx = fn_ctx("p", &[], "void");
        let ir = vec![
            instr("const_i64", Some("v0"), vec![Op::Int(99)]),
            instr("io_out", None, vec![Op::Var("v0".into())]),
            instr("ret_void", None, vec![]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::MsX64).unwrap();
        // MS x64 arg 0 → RCX.  `mov rcx, [rbp+disp32]` = 48 8B 8D ..
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0x8D]),
                "expected `mov rcx, [rbp+disp]` for io_out arg on MS x64");
        // And not the System V form (48 8B BD).
        assert!(!body_contains(&bytes, &[0x48, 0x8B, 0xBD]),
                "should NOT emit `mov rdi, ...` on MS x64");
    }

    #[test]
    fn fn_global_load_unknown_errors() {
        use std::collections::HashMap;
        let globals = HashMap::<String, usize>::new();
        let ctx = fn_ctx("rd", &[], "u64");
        let ir = vec![
            instr("global_load", Some("v0"), vec![Op::Var("nope".into())]),
            instr("ret_u64", None, vec![Op::Var("v0".into())]),
        ];
        let result = compile_function_with_globals(&ctx, &ir, X86_64Abi::SysV, &globals);
        assert!(result.is_err());
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

    // ── LANG75 — call_builtin lowering ────────────────────────────────────────

    #[test]
    fn call_builtin_putchar_sysv_uses_rdi_and_records_reloc() {
        // CIR: const_i32 v0 = 65; call_builtin "putchar", v0; ret_void
        let ctx = fn_ctx("emit_A", &[], "void");
        let ir = vec![
            instr("const_i32", Some("v0"), vec![Op::Int(65)]),
            instr("call_builtin", None,
                  vec![Op::Var("putchar".into()), Op::Var("v0".into())]),
            instr("ret_void", None, vec![]),
        ];
        let (bytes, relocs) = compile_function_with_relocs(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // System V arg 0 → RDI.  `mov rdi, [rbp+disp32]` = 48 8B BD …
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0xBD]),
                "expected `mov rdi, [rbp+disp]` for putchar arg on SysV");
        // Then `call __twig_putchar` (E8) with one PltRel32 reloc.
        let plt: Vec<_> = relocs.iter()
            .filter(|r| r.kind == ExternalRelocKind::PltRel32
                     && r.symbol == "__twig_putchar")
            .collect();
        assert_eq!(plt.len(), 1, "expected exactly one __twig_putchar reloc");
    }

    #[test]
    fn call_builtin_putchar_msx64_uses_rcx() {
        let ctx = fn_ctx("emit_A", &[], "void");
        let ir = vec![
            instr("const_i32", Some("v0"), vec![Op::Int(65)]),
            instr("call_builtin", None,
                  vec![Op::Var("putchar".into()), Op::Var("v0".into())]),
            instr("ret_void", None, vec![]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::MsX64).unwrap();
        // MS x64 arg 0 → RCX.  `mov rcx, [rbp+disp32]` = 48 8B 8D ..
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0x8D]),
                "expected `mov rcx, [rbp+disp]` for putchar arg on MS x64");
        assert!(!body_contains(&bytes, &[0x48, 0x8B, 0xBD]),
                "should NOT emit `mov rdi, ...` on MS x64");
    }

    #[test]
    fn call_builtin_getchar_stores_rax_into_dest() {
        // CIR: call_builtin "getchar" → r; ret_i32 r
        let ctx = fn_ctx("read_one", &[], "i32");
        let ir = vec![
            instr("call_builtin", Some("r"),
                  vec![Op::Var("getchar".into())]),
            instr("ret_i32", None, vec![Op::Var("r".into())]),
        ];
        let (bytes, relocs) = compile_function_with_relocs(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // After CALL, store RAX into the dest slot.  `mov [rbp+disp], rax`
        // = 48 89 85 .. .. .. .. (REX.W + 89 /0 ModR/M with RAX as src).
        assert!(body_contains(&bytes, &[0x48, 0x89, 0x85]),
                "expected `mov [rbp+disp], rax` after getchar call");
        let plt: Vec<_> = relocs.iter()
            .filter(|r| r.symbol == "__twig_getchar")
            .collect();
        assert_eq!(plt.len(), 1, "expected exactly one __twig_getchar reloc");
    }

    #[test]
    fn call_builtin_print_string_marshals_two_args_sysv() {
        // CIR: const_i64 p=0; const_i64 n=0; call_builtin "print_string", p, n
        let ctx = fn_ctx("emit_str", &[], "void");
        let ir = vec![
            instr("const_i64", Some("p"), vec![Op::Int(0)]),
            instr("const_i64", Some("n"), vec![Op::Int(0)]),
            instr("call_builtin", None,
                  vec![Op::Var("print_string".into()),
                       Op::Var("p".into()),
                       Op::Var("n".into())]),
            instr("ret_void", None, vec![]),
        ];
        let (bytes, relocs) = compile_function_with_relocs(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // arg 0 → RDI (48 8B BD), arg 1 → RSI (48 8B B5).
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0xBD]),
                "expected `mov rdi, [rbp+disp]` for print_string arg 0");
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0xB5]),
                "expected `mov rsi, [rbp+disp]` for print_string arg 1");
        let plt: Vec<_> = relocs.iter()
            .filter(|r| r.symbol == "__twig_print_string")
            .collect();
        assert_eq!(plt.len(), 1);
    }

    #[test]
    fn call_builtin_unknown_name_refuses() {
        // Unknown helper → MalformedInstr, not panic.
        let ctx = fn_ctx("bad", &[], "void");
        let ir = vec![
            instr("call_builtin", None,
                  vec![Op::Var("frobnicate".into())]),
            instr("ret_void", None, vec![]),
        ];
        let result = compile_function(&ctx, &ir, X86_64Abi::SysV);
        assert!(result.is_err(), "expected error for unknown builtin");
    }

    #[test]
    fn call_builtin_wrong_arg_count_refuses() {
        // putchar expects exactly 1 arg; passing 0 must be rejected.
        let ctx = fn_ctx("bad_arity", &[], "void");
        let ir = vec![
            instr("call_builtin", None,
                  vec![Op::Var("putchar".into())]),
            instr("ret_void", None, vec![]),
        ];
        let result = compile_function(&ctx, &ir, X86_64Abi::SysV);
        assert!(result.is_err(), "expected error for putchar with 0 args");
    }

    #[test]
    fn call_builtin_void_with_dest_refuses() {
        // putchar returns void; supplying a dest is a malformed call site.
        let ctx = fn_ctx("bad_dest", &[], "void");
        let ir = vec![
            instr("const_i32", Some("c"), vec![Op::Int(65)]),
            instr("call_builtin", Some("r"),
                  vec![Op::Var("putchar".into()), Op::Var("c".into())]),
            instr("ret_void", None, vec![]),
        ];
        let result = compile_function(&ctx, &ir, X86_64Abi::SysV);
        assert!(result.is_err());
    }

    #[test]
    fn call_builtin_returning_without_dest_refuses() {
        // getchar must have a dest.
        let ctx = fn_ctx("bad_no_dest", &[], "void");
        let ir = vec![
            instr("call_builtin", None,
                  vec![Op::Var("getchar".into())]),
            instr("ret_void", None, vec![]),
        ];
        let result = compile_function(&ctx, &ir, X86_64Abi::SysV);
        assert!(result.is_err());
    }

    // ── LANG76 — byte memory ops + heap allocation ────────────────────────────

    #[test]
    fn alloc_bytes_calls_runtime_helper_and_stores_rax() {
        // alloc_bytes 16 -> buf
        let ctx = fn_ctx("a", &[], "i64");
        let ir = vec![
            instr("const_i64", Some("n"), vec![Op::Int(16)]),
            instr("alloc_bytes", Some("buf"),
                  vec![Op::Var("n".into())]),
            instr("ret_i64", None, vec![Op::Var("buf".into())]),
        ];
        let (bytes, relocs) = compile_function_with_relocs(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // Arg goes into RDI (SysV).
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0xBD]),
                "expected `mov rdi, [rbp+disp]` for n");
        // CALL E8 followed by 4 zero placeholder bytes.
        assert!(body_contains(&bytes, &[0xE8]),
                "expected CALL opcode E8");
        // Store RAX to dest slot: 48 89 85 ...
        assert!(body_contains(&bytes, &[0x48, 0x89, 0x85]),
                "expected `mov [rbp+disp], rax` for buf");
        let plt: Vec<_> = relocs.iter()
            .filter(|r| r.symbol == "__twig_alloc_bytes").collect();
        assert_eq!(plt.len(), 1);
    }

    #[test]
    fn load_byte_emits_add_then_movzx() {
        // load_byte ptr, off -> dest
        let params = vec![("ptr".into(), "i64".into()),
                          ("off".into(), "i64".into())];
        let ctx = fn_ctx("lb", &params, "i64");
        let ir = vec![
            instr("load_byte", Some("v"),
                  vec![Op::Var("ptr".into()), Op::Var("off".into())]),
            instr("ret_i64", None, vec![Op::Var("v".into())]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // `add rax, rcx` = 48 01 C8
        assert!(body_contains(&bytes, &[0x48, 0x01, 0xC8]),
                "expected `add rax, rcx` to combine ptr + offset");
        // `movzx rax, byte ptr [rax]` = 48 0F B6 00
        assert!(body_contains(&bytes, &[0x48, 0x0F, 0xB6, 0x00]),
                "expected `movzx rax, byte [rax]`");
    }

    #[test]
    fn store_byte_emits_mov_byte_at_dl() {
        // store_byte ptr, off, val
        let params = vec![("ptr".into(), "i64".into()),
                          ("off".into(), "i64".into()),
                          ("val".into(), "i64".into())];
        let ctx = fn_ctx("sb", &params, "void");
        let ir = vec![
            instr("store_byte", None,
                  vec![Op::Var("ptr".into()),
                       Op::Var("off".into()),
                       Op::Var("val".into())]),
            instr("ret_void", None, vec![]),
        ];
        let bytes = compile_function(&ctx, &ir, X86_64Abi::SysV).unwrap();
        // After `add rax, rcx` (48 01 C8), load val into rdx, then `mov [rax], dl`
        // which with forced REX prefix is `40 88 10` (REX with all bits 0, opcode 88,
        // ModRM mod=00 reg=010(DL) rm=000(RAX)).
        assert!(body_contains(&bytes, &[0x48, 0x01, 0xC8]),
                "expected `add rax, rcx`");
        assert!(body_contains(&bytes, &[0x40, 0x88, 0x10]),
                "expected `mov byte ptr [rax], dl` with empty REX prefix");
    }

    #[test]
    fn load_byte_missing_offset_refuses() {
        let params = vec![("ptr".into(), "i64".into())];
        let ctx = fn_ctx("bad", &params, "i64");
        let ir = vec![
            instr("load_byte", Some("v"),
                  vec![Op::Var("ptr".into())]),
            instr("ret_i64", None, vec![Op::Var("v".into())]),
        ];
        assert!(compile_function(&ctx, &ir, X86_64Abi::SysV).is_err());
    }

    #[test]
    fn store_byte_with_dest_refuses() {
        let ctx = fn_ctx("bad", &[], "void");
        let ir = vec![
            instr("const_i64", Some("p"), vec![Op::Int(0)]),
            instr("const_i64", Some("o"), vec![Op::Int(0)]),
            instr("const_i64", Some("v"), vec![Op::Int(0)]),
            instr("store_byte", Some("r"),  // illegal!
                  vec![Op::Var("p".into()),
                       Op::Var("o".into()),
                       Op::Var("v".into())]),
            instr("ret_void", None, vec![]),
        ];
        assert!(compile_function(&ctx, &ir, X86_64Abi::SysV).is_err());
    }

    #[test]
    fn call_builtin_print_i64_matches_io_out() {
        // call_builtin "print_i64", v0 should produce the same shape as
        // io_out v0 — same arg marshalling and same `__twig_print_i64` reloc.
        let ctx = fn_ctx("print_via_builtin", &[], "void");
        let ir = vec![
            instr("const_i64", Some("v0"), vec![Op::Int(99)]),
            instr("call_builtin", None,
                  vec![Op::Var("print_i64".into()), Op::Var("v0".into())]),
            instr("ret_void", None, vec![]),
        ];
        let (bytes, relocs) = compile_function_with_relocs(&ctx, &ir, X86_64Abi::SysV).unwrap();
        assert!(body_contains(&bytes, &[0x48, 0x8B, 0xBD]),
                "expected `mov rdi, [rbp+disp]` for print_i64 arg");
        let plt: Vec<_> = relocs.iter()
            .filter(|r| r.symbol == "__twig_print_i64")
            .collect();
        assert_eq!(plt.len(), 1);
    }

    // =======================================================================
    // LANG-FULL E2 (native-AOT leg): narrow-width unsigned masking
    // =======================================================================
    //
    // x86_64 mirrors the aarch64 backend: after each narrow `uⁿ` op the codegen
    // appends `movabs rcx, <mask>; and <dst>, rcx` so the result wraps mod-2ⁿ
    // (`add_u8 200, 100` → 44, not 300).  There is no in-repo x86 JIT loader, so
    // the *executed* value proof for x86_64 is the lang-aot matrix on a Linux
    // x86_64 CI runner (and the aarch64 backend proves the masked values by
    // directly executing its output).  Here we prove the mask bytes are emitted.

    fn typed_instr(op: &str, dest: Option<&str>, srcs: Vec<Op>, ty: &str) -> CIRInstr {
        CIRInstr { op: op.into(), dest: dest.map(str::to_string), srcs, ty: ty.into(), deopt_to: None }
    }

    /// Compile `const a; const b; <op> v0 = a,b; ret v0` at width `ty`, return
    /// the byte length of the generated function.
    fn narrow_binop_len(full_op: &str, ty: &str) -> usize {
        let ir = vec![
            typed_instr(&format!("const_{ty}"), Some("a"), vec![Op::Int(200)], ty),
            typed_instr(&format!("const_{ty}"), Some("b"), vec![Op::Int(100)], ty),
            typed_instr(full_op, Some("v0"), vec![Op::Var("a".into()), Op::Var("b".into())], ty),
            instr("ret_u64", None, vec![Op::Var("v0".into())]),
        ];
        compile_function(&fn_ctx("f", &[], "u64"), &ir, X86_64Abi::SysV)
            .expect("narrow binop must lower")
            .len()
    }

    #[test]
    fn narrow_add_emits_extra_mask_bytes() {
        // u8 add masks its result; u64 add does not — so the u8 function is
        // strictly longer (the `movabs rcx,mask; and rax,rcx` mask sequence).
        assert!(
            narrow_binop_len("add_u8", "u8") > narrow_binop_len("add_u64", "u64"),
            "u8 add must emit a width mask the u64 add omits",
        );
    }

    #[test]
    fn narrow_widths_all_emit_mask() {
        let wide = narrow_binop_len("add_u64", "u64");
        for ty in ["u4", "u8", "u16", "u32"] {
            assert!(
                narrow_binop_len(&format!("add_{ty}"), ty) > wide,
                "{ty} add must emit a width mask",
            );
        }
    }

    #[test]
    fn i64_op_is_never_masked() {
        // i64 is full-width — identical length to the unmasked u64 form.
        assert_eq!(
            narrow_binop_len("add_i64", "i64"),
            narrow_binop_len("add_u64", "u64"),
        );
    }

    // ---- f64 (ALGOL `real`) SSE2 — LANG-FULL E3 ----
    //
    // x86_64 is not locally runnable (no x86 ISA simulator); these are
    // structural exact-opcode checks. The encodings were verified byte-for-byte
    // against the system assembler, and the *executed* cross-backend proof is
    // the lang-aot matrix `NativeAot` column on the Linux-x86 CI runner.

    fn finstr(op: &str, dest: Option<&str>, srcs: Vec<Op>) -> CIRInstr {
        CIRInstr { op: op.into(), dest: dest.map(str::to_string), srcs, ty: "f64".into(), deopt_to: None }
    }

    /// `r := 2.5 * 2.0` lowers to `movsd` loads (F2 0F 10), `mulsd` (F2 0F 59),
    /// and a `movsd` store (F2 0F 11) — no integer `imul`.
    #[test]
    fn f64_multiply_emits_sse() {
        let ir = vec![
            finstr("const_f64", Some("a"), vec![Op::Float(2.5)]),
            finstr("const_f64", Some("b"), vec![Op::Float(2.0)]),
            finstr("mul_f64", Some("r"), vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let b = compile_function(&fn_ctx("fmul", &[], "u64"), &ir, X86_64Abi::SysV)
            .expect("f64 multiply must lower");
        assert!(contains_seq(&b, &[0xF2, 0x0F, 0x59]), "expected mulsd (F2 0F 59)");
        assert!(contains_seq(&b, &[0xF2, 0x0F, 0x10]), "expected movsd load (F2 0F 10)");
        assert!(contains_seq(&b, &[0xF2, 0x0F, 0x11]), "expected movsd store (F2 0F 11)");
    }

    /// `7.0 / 2.0` uses `divsd` (F2 0F 5E), not the integer `idiv` path.
    #[test]
    fn f64_divide_emits_divsd() {
        let ir = vec![
            finstr("const_f64", Some("a"), vec![Op::Float(7.0)]),
            finstr("const_f64", Some("b"), vec![Op::Float(2.0)]),
            finstr("div_f64", Some("r"), vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let b = compile_function(&fn_ctx("fdiv", &[], "u64"), &ir, X86_64Abi::SysV)
            .expect("f64 divide must lower");
        assert!(contains_seq(&b, &[0xF2, 0x0F, 0x5E]), "expected divsd (F2 0F 5E)");
    }

    /// `r < 4.0` uses `ucomisd` (66 0F 2E) + `setcc`, not integer `cmp`.
    #[test]
    fn f64_compare_emits_ucomisd() {
        let ir = vec![
            finstr("const_f64", Some("a"), vec![Op::Float(3.5)]),
            finstr("const_f64", Some("b"), vec![Op::Float(4.0)]),
            finstr("cmp_lt_f64", Some("r"), vec![Op::Var("a".into()), Op::Var("b".into())]),
            instr("ret_u64", None, vec![Op::Var("r".into())]),
        ];
        let b = compile_function(&fn_ctx("fcmp", &[], "u64"), &ir, X86_64Abi::SysV)
            .expect("f64 compare must lower");
        assert!(contains_seq(&b, &[0x66, 0x0F, 0x2E]), "expected ucomisd (66 0F 2E)");
    }

    fn contains_seq(hay: &[u8], needle: &[u8]) -> bool {
        hay.windows(needle.len()).any(|w| w == needle)
    }
}
