//! # `twig-aot` — Twig ahead-of-time compiler.
//!
//! Compiles a Twig source file to a native ARM64 Mach-O executable that
//! macOS can launch directly.
//!
//! ## Pipeline
//!
//! ```text
//! Twig source
//!     │  twig_ir_compiler::compile_source
//!     ▼
//! IIRModule  (interpreter-ir)
//!     │  for each fn: aot_core::{infer, specialise} → CIR
//!     │  aarch64_backend::compile_function → ARM64 machine code
//!     ▼
//! Vec<(fn_name, Vec<u8>)>
//!     │  aot_core::link::link → (text_bytes, offsets)
//!     │  aot_core::link::entry_point_offset(entry="main")
//!     ▼
//! (text_bytes, entry_off)
//!     │  code_packager::macho_object::pack_object
//!     ▼
//! object.o  (Mach-O MH_OBJECT with `_main` symbol)
//!     │  ld -arch arm64 -platform_version macos 15.0 15.0 -e _main -o exe
//!     ▼
//! native ARM64 executable that exec()s without ENOEXEC
//! ```
//!
//! ## Why we shell out to `ld`
//!
//! On macOS 15+ (Sequoia / Tahoe) the kernel attaches a "provenance"
//! tag to every executable file recording which process wrote it.
//! Files written by the system linker (`/usr/bin/ld`, Apple-signed)
//! inherit a trusted provenance and run normally; files written by
//! random user code (e.g. our crate) are SIGKILL'd by
//! `AppleSystemPolicy` regardless of how well-formed the Mach-O is.
//!
//! Delegating the final link to `ld` solves the provenance problem.
//! As a bonus, `ld` also handles dyld setup, code signing, and the
//! various subtleties of producing a runnable Apple Silicon binary —
//! we just supply the `__text` bytes and the entry symbol.
//!
//! ## Execution model
//!
//! Every Twig program's `main` function returns a `u64`.  The linker
//! produces a binary that calls `_main` and routes the `x0` return
//! through `exit()`, so the process exit code equals `main()`'s return
//! value modulo 256.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

// Pull gc-core-capi's object code into every binary/test binary built from this
// crate. The C runtime archive (`libtwig_aot_runtime.a`, embedded below) contains
// `dynval_runtime.c`, which references `__twig_gc_alloc`; twig_gc.c used to define
// it, but it has been retired. gc-core-capi provides `__twig_gc_alloc` (and the
// rest of the `__twig_gc_*` compat ABI) as `#[no_mangle]` exports. Depending on it
// as an rlib (not the staticlib) means rustc hands gc-core-capi's objects to the
// final linker so those undefined references resolve — without duplicating Rust
// std the way linking two staticlibs full of std would.
//
// In edition 2021 no `extern crate` is needed: the `#[used]` static below
// references gc-core-capi by path, which both makes the crate a link input and
// pins at least one of its symbols so the linker can never drop the whole rlib
// before resolving the C archive's late-bound `__twig_gc_alloc`.

/// Force the linker to retain gc-core-capi's objects even though nothing in this
/// crate's Rust code calls them directly — the only references are the C runtime
/// archive's undefined `__twig_gc_alloc`/`__twig_gc_safepoint`, which rustc cannot
/// see when deciding whether the rlib is "used". `#[used]` pins this function
/// pointer into the binary, keeping gc-core-capi on the final link line. The
/// `__twig_gc_*` compat aliases live in gc-core-capi's private `twig_compat`
/// module (not reachable as a Rust path here), but they share the rlib archive;
/// once the rlib is on the link line, the system linker pulls the member that
/// defines `__twig_gc_alloc` to satisfy the C archive's undefined reference. We
/// therefore anchor on the crate-root-public `__gc_alloc`, which `__twig_gc_alloc`
/// forwards to.
#[used]
static _FORCE_GC_CORE_CAPI: extern "C" fn(i64) -> i64 = gc_core_capi::__gc_alloc;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use aarch64_backend::{compile_with_globals_and_stackmap, GlobalWordReloc, Reloc};
use aarch64_encoder::{Assembler, LabelId, Reg};

/// Symbol of the generated **GC entry wrapper** — the real `_main` of an AOT image.
/// It registers the module's GC stack maps (via [`GC_INIT_STACKMAPS`]) and then
/// tail-calls the user's entry, returning its result unchanged. Transparent to the
/// program's output; it only adds the pre-`main` registration hook precise GC needs.
const GC_AOT_ENTRY: &str = "__gc_aot_entry";
/// Symbol of the generated **stack-map registration** function, called once by the
/// entry wrapper before the user program. It registers every user function's stack
/// map with the GC runtime so `__gc_collect_precise` can resolve real frames to exact
/// reference slots (Increment B); functions without records degrade to conservative.
const GC_INIT_STACKMAPS: &str = "__gc_init_stackmaps";
/// The GC runtime's per-function stack-map registration entry (gc-core-capi). The
/// generated `__gc_init_stackmaps` calls it once per user function; resolved by the
/// system linker from the embedded `libgc_core_capi.a` (an external `BL`).
const GC_REGISTER_STACKMAP: &str = "__gc_register_stackmap";

/// One `ADR` in `__gc_init_stackmaps` that must be patched to a user function's
/// runtime address (`func_start`, the first argument of [`GC_REGISTER_STACKMAP`]).
/// The instruction is emitted as a placeholder; pass 2 fills in its 21-bit byte
/// displacement once `link()` has assigned every function its `__text` offset.
///
/// **Why `ADR`, not `ADRP`+`ADD`.** `ADR` is PC-relative in *bytes*: at run time it
/// computes `PC + imm`, where both `PC` and the target are `base + offset`, so the
/// base cancels and the displacement `target_off − adr_off` (a link-time constant) is
/// correct for *any* load address — no relocation needed. `ADRP` computes
/// `page(PC) + (imm << 12)`; the `page()` rounding does **not** commute with an
/// unaligned base, and `ld` places `__text` after the Mach header inside the first
/// page, so the runtime `__text` base is *not* 4 KiB-aligned. Baking an `ADRP` page
/// immediate from link offsets would therefore mis-compute `func_start` by up to a
/// page — a silent, UAF-class defect (the registry only stores the value, so nothing
/// faults until a precise collection resolves a return address to the wrong map).
/// `ADR` sidesteps this entirely; its cost is a ±1 MiB reach, enforced fail-loud.
struct FuncAddrReloc {
    /// Word index (within `__gc_init_stackmaps`) of the `ADR` instruction.
    adr_word: usize,
    /// Name of the user function whose address this instruction loads.
    target: String,
}

/// One user function's stack map, flattened to the arrays
/// [`GC_REGISTER_STACKMAP`] takes: its byte length and, per safepoint record, the
/// return-address `pc_offset` and the live-reference frame slots.
struct FnStackMap {
    /// The function's symbol name (used to resolve `func_start` in pass 2).
    name: String,
    /// The function's own code length in bytes (`func_len`).
    len: usize,
    /// `(pc_offset, slots)` per safepoint record. `slots` are FP-relative byte
    /// offsets (may be negative), exactly as the backend emitted them.
    records: Vec<(u32, Vec<i32>)>,
}

/// Byte offset of every return address in `code` — the byte just after each `BL` /
/// `BLR`. These are the safepoint PCs a precise stack walk observes; the synthetic GC
/// wrapper needs its own so its frame (live throughout the program) resolves precisely
/// instead of forcing a conservative re-scan of its callee. Mirrors the aarch64
/// backend's own scan (which is private to that crate); AArch64 is fixed-width and the
/// wrapper carries no inline data, so every 4-byte word is a real instruction.
fn call_return_offsets(code: &[u8]) -> Vec<u32> {
    let mut out = Vec::new();
    for (i, word) in code.chunks_exact(4).enumerate() {
        let w = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
        let is_bl = (w >> 26) == 0b100101; // BL imm26
        let is_blr = (w & 0xFFFF_FC1F) == 0xD63F_0000; // BLR Rn
        if is_bl || is_blr {
            out.push(((i + 1) * 4) as u32);
        }
    }
    out
}

/// Map an [`aarch64_encoder::EncodeError`] to an [`AotError`].
fn gc_asm_err(e: aarch64_encoder::EncodeError) -> AotError {
    AotError::Linker { status: None, stderr: format!("twig-aot: GC entry codegen: {e:?}") }
}

/// Build the **GC entry wrapper** [`GC_AOT_ENTRY`] — the image's real `_main`.
///
/// ```text
///   __gc_aot_entry:
///       stp x29, x30, [sp, #-16]!     ; save fp/lr
///       mov x29, sp
///       bl  __gc_init_stackmaps       ; register every function's stack map
///       bl  <user_entry>              ; x0 = program result
///       ldp x29, x30, [sp], #16       ; restore fp/lr (x0 untouched)
///       ret                           ; return the program result verbatim
/// ```
///
/// Both `BL`s are intra-module and patched by the two-pass linker. `bl
/// __gc_init_stackmaps` may clobber caller-saved registers (x0–x18) per the ABI, but
/// it runs *before* the user entry and a twig `main` takes no arguments, so nothing
/// live is lost; the user entry's x0 result survives the `ldp`/`ret` unchanged.
fn build_gc_wrapper(user_entry: &str) -> Result<(Vec<u8>, Vec<Reloc>), AotError> {
    let mut a = Assembler::new();
    a.stp_pre(Reg::Fp, Reg::Lr, Reg::Sp, -16).map_err(gc_asm_err)?;
    a.add_imm(Reg::Fp, Reg::Sp, 0).map_err(gc_asm_err)?; // mov x29, sp
    a.bl_external(GC_INIT_STACKMAPS); // intra-module BL, patched by the linker
    a.bl_external(user_entry); // x0 = user program result
    a.ldp_post(Reg::Fp, Reg::Lr, Reg::Sp, 16).map_err(gc_asm_err)?;
    a.ret();
    let relocs = std::mem::take(&mut a.external_relocs);
    Ok((a.finish().map_err(gc_asm_err)?, relocs))
}

/// Build the **stack-map registration** function [`GC_INIT_STACKMAPS`]: for every
/// user function that has safepoint records, marshal the eight
/// [`GC_REGISTER_STACKMAP`] arguments and call it, so the GC runtime can turn a
/// return address in that function into its exact live-reference slots.
///
/// ```text
///   __gc_init_stackmaps:
///       stp x29,x30,[sp,#-16]! ; mov x29,sp
///       ; ── per function F with records ──
///       adr  x0, F                   ; x0 = func_start   (patched in pass 2)
///       mov  x1, #<F byte length>    ; x1 = func_len
///       mov  x2, #<record count>     ; x2 = num_records
///       adr  x3, <F.pc_offsets>      ; x3 = pc_offsets[]
///       mov  x4, #0                  ; x4 = frame_sizes  (NULL — unused)
///       mov  x5, #0                  ; x5 = callee_masks (NULL — unused)
///       adr  x6, <F.slot_counts>     ; x6 = slot_counts[]
///       adr  x7, <F.slots_flat>      ; x7 = slots_flat[]  (or #0 if empty)
///       bl   __gc_register_stackmap
///       ; ── … next function … ──
///       ldp x29,x30,[sp],#16 ; ret
///   <data pool: each function's pc_offsets / slot_counts / slots_flat words>
/// ```
///
/// **`func_start`** is the only runtime-computed argument: the function's absolute
/// address, materialised by an `ADR` (PC-relative, byte-granular) patched in pass 2
/// (see [`FuncAddrReloc`] for why `ADR`, not `ADRP`+`ADD`). Everything else is a
/// compile-time constant. The three arrays
/// are constant data emitted as raw words in a **pool after the final `ret`** (never
/// executed) and pointed at by `adr` (PC-relative, resolved at `finish`) — since the
/// pool is init-internal the displacement is independent of the module layout, so it
/// needs no relocation. The registry copies the slot data into owned storage, so the
/// arrays need not outlive the call.
///
/// Returns the function bytes, the external `BL __gc_register_stackmap` relocations
/// (patched by the system linker), and the [`FuncAddrReloc`]s pass 2 resolves.
#[allow(clippy::type_complexity)]
fn build_gc_init_stackmaps(
    fn_maps: &[FnStackMap],
) -> Result<(Vec<u8>, Vec<Reloc>, Vec<FuncAddrReloc>), AotError> {
    let mut a = Assembler::new();
    let mut faddr: Vec<FuncAddrReloc> = Vec::new();
    // Data-pool contents, bound after the epilogue: (label, words).
    let mut pool: Vec<(LabelId, Vec<u32>)> = Vec::new();

    // Prologue — a real frame; the body makes calls (each `bl` clobbers lr).
    a.stp_pre(Reg::Fp, Reg::Lr, Reg::Sp, -16).map_err(gc_asm_err)?;
    a.add_imm(Reg::Fp, Reg::Sp, 0).map_err(gc_asm_err)?; // mov x29, sp

    for fm in fn_maps {
        if fm.records.is_empty() {
            continue; // no safepoints → nothing to register (conservative frame)
        }
        // Flatten this function's records into the three parallel arrays.
        let mut pc_offsets: Vec<u32> = Vec::with_capacity(fm.records.len());
        let mut slot_counts: Vec<u32> = Vec::with_capacity(fm.records.len());
        let mut slots_flat: Vec<u32> = Vec::new();
        for (pc, slots) in &fm.records {
            pc_offsets.push(*pc);
            slot_counts.push(slots.len() as u32);
            slots_flat.extend(slots.iter().map(|&s| s as u32)); // i32 → u32 bit-cast
        }

        let pc_lbl = a.create_label();
        let cnt_lbl = a.create_label();
        let slots_lbl = if slots_flat.is_empty() { None } else { Some(a.create_label()) };

        // x0 = func_start — ADR placeholder, patched (base-independently) in pass 2.
        let adr_word = a.adr_placeholder(Reg::X0);
        faddr.push(FuncAddrReloc { adr_word, target: fm.name.clone() });

        a.mov_imm64(Reg::X1, fm.len as u64); //          func_len
        a.mov_imm64(Reg::X2, fm.records.len() as u64); // num_records
        a.adr(Reg::X3, pc_lbl); //                       pc_offsets
        a.mov_imm64(Reg::X4, 0); //                      frame_sizes = NULL
        a.mov_imm64(Reg::X5, 0); //                      callee_masks = NULL
        a.adr(Reg::X6, cnt_lbl); //                      slot_counts
        match slots_lbl {
            Some(l) => a.adr(Reg::X7, l), //             slots_flat
            None => a.mov_imm64(Reg::X7, 0), //          (no slots → NULL)
        }
        a.bl_external(GC_REGISTER_STACKMAP);

        pool.push((pc_lbl, pc_offsets));
        pool.push((cnt_lbl, slot_counts));
        if let Some(l) = slots_lbl {
            pool.push((l, slots_flat));
        }
    }

    // Epilogue — restore and return before the data pool.
    a.ldp_post(Reg::Fp, Reg::Lr, Reg::Sp, 16).map_err(gc_asm_err)?;
    a.ret();

    // Constant data pool — read via `adr` above, never executed (after `ret`).
    for (lbl, words) in &pool {
        a.bind(*lbl).map_err(gc_asm_err)?;
        for &w in words {
            a.emit_data_word(w);
        }
    }

    let relocs = std::mem::take(&mut a.external_relocs);
    Ok((a.finish().map_err(gc_asm_err)?, relocs, faddr))
}
use aot_core::infer::infer_types;
use aot_core::link::{entry_point_offset, link};
use aot_core::specialise::aot_specialise;
use code_packager::macho_object::{
    pack_object_with_globals_and_externals, ExternBranchReloc, GlobalByteReloc,
};
use code_packager::{
    pack_elf64_object_x86_64, pack_pe_object_x86_64,
    Target, X86RelocKind, X86RelocRecord,
};

// ── Embedded Twig AOT runtime archive ──────────────────────────────────────
//
// `build.rs` compiles `runtime/twig_runtime.c` (via the `cc` crate) into a
// static archive at `$OUT_DIR/libtwig_aot_runtime.a` and exports the path
// via `cargo:rustc-env=TWIG_RUNTIME_ARCHIVE`.  `include_bytes!` resolves
// the path at *compile time* (not runtime) and bakes the archive bytes into
// this binary.
//
// At AOT link time (when the user runs `twig-aot`), we write these bytes to
// a temp file and pass it to the system linker (`ld`).  `ld` extracts
// whatever symbols it needs (e.g. `__twig_print_i64`) from the archive and
// links them into the final executable.  This means:
//
// - No separate installation of a runtime library.
// - Portable: `twig_runtime.c` uses `<stdio.h>` (printf), which resolves
//   through `-lSystem` on macOS and `-lc` on Linux — no raw syscall numbers.
static RUNTIME_ARCHIVE: &[u8] = include_bytes!(env!("TWIG_RUNTIME_ARCHIVE"));

/// Linux x86-64 runtime archive embedded at compile time (LANG46).
///
/// On a Linux x86-64 build host, this is the real `.a` archive built by
/// `build.rs` via the `cc` crate.  On other hosts it is a 1-byte stub;
/// `compile_file_linux_x86_64` checks the length and refuses with a
/// clear error if invoked on a non-Linux host's build.
static RUNTIME_LINUX_X86_64: &[u8] =
    include_bytes!(env!("TWIG_RUNTIME_ARCHIVE_LINUX_X86_64"));

/// Windows x86-64 runtime archive embedded at compile time (LANG46).
///
/// On a Windows x86-64 build host, this is the real `.lib` archive
/// built by `build.rs` via the `cc` crate using MSVC's `cl.exe`.  On
/// other hosts it is a 1-byte stub.
static RUNTIME_WINDOWS_X86_64: &[u8] =
    include_bytes!(env!("TWIG_RUNTIME_ARCHIVE_WINDOWS_X86_64"));

/// The native garbage collector, embedded as `gc-core-capi`'s static archive
/// (`libgc_core_capi.a`), built by `build.rs` (#118b-2b — retires twig_gc.c).
///
/// On a supported host it is the real archive; on an unsupported host it is a
/// 1-byte stub (AOT is refused there anyway). At each AOT link site we write
/// these bytes to a temp `.a` and pass it to the system linker *after* the
/// runtime archive, so the emitted executable's `__twig_gc_alloc` /
/// `__twig_gc_safepoint` references — and dynval_runtime.c's `__twig_gc_alloc`,
/// pulled in from the runtime archive — resolve against the collector.
///
/// Because a static archive contributes only the members needed to satisfy an
/// undefined symbol, programs that never allocate (e.g. a plain `exit(42)`)
/// pull nothing from this archive and pay no size/duplicate-symbol cost.
static GC_CORE_ARCHIVE: &[u8] = include_bytes!(env!("GC_CORE_CAPI_ARCHIVE"));
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use iir_builtin_lowering::{
    intern_symbols, lower_box_unbox_to_runtime_calls, lower_closures_to_heap, lower_dyn_repr,
    lower_dynamic_arith, lower_global_io, lower_heap_builtins_runtime,
};
use iir_refinement_pass::{check_module as check_refinements, RefinementMode};
use jit_core::backend::FunctionContext;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by the AOT compiler.
#[derive(Debug)]
#[allow(dead_code)] // diagnostic carried via Debug formatting
pub enum AotError {
    /// `twig-ir-compiler` failed to parse / compile the source.
    Compile(String),
    /// The ARM64 backend rejected one of the functions (untyped or
    /// unsupported opcode).
    BackendRefused {
        /// Function name that the backend declined to compile.
        function: String,
    },
    /// The IIR module has no `entry_point` set.
    NoEntryPoint,
    /// `code-packager` rejected the artifact (malformed or unsupported
    /// target).
    Packager(String),
    /// Filesystem error while writing the output binary.
    Io(std::io::Error),
    /// The system linker (`ld`) returned a non-zero exit code or could
    /// not be located on `PATH`.
    Linker {
        /// `ld`'s exit code, if it ran at all.
        status: Option<i32>,
        /// `ld`'s stderr, captured for diagnostics.
        stderr: String,
    },
    /// One or more refinement proof obligations were violated (LANG42).
    ///
    /// In `Lenient` mode this fires only for `ProvenUnsafe` outcomes.
    /// In `Strict` mode it also fires for `Unknown` outcomes.
    RefinementViolations(Vec<iir_refinement_pass::RefinementError>),
}

impl std::fmt::Display for AotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AotError::Compile(s)            => write!(f, "twig compile: {s}"),
            AotError::BackendRefused { function } =>
                write!(f, "backend refused function '{function}' (untyped or unsupported op)"),
            AotError::NoEntryPoint          => write!(f, "module has no entry point"),
            AotError::Packager(s)           => write!(f, "packager: {s}"),
            AotError::Io(e)                 => write!(f, "io: {e}"),
            AotError::Linker { status, stderr } =>
                write!(f, "linker (ld) failed: status={status:?}: {stderr}"),
            AotError::RefinementViolations(errs) => {
                write!(f, "{} refinement violation(s):", errs.len())?;
                for e in errs {
                    write!(f, "\n  {e}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for AotError {}

impl From<std::io::Error> for AotError {
    fn from(e: std::io::Error) -> Self { AotError::Io(e) }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a Twig source string to a Mach-O **object file** (`MH_OBJECT`),
/// suitable for feeding to `ld`.
///
/// `module_name` is used in profile dumps and error messages; pick
/// something descriptive of the source file (e.g. its stem).
///
/// Returns the raw object-file bytes; the caller is responsible for
/// running `ld` to produce an executable.  See
/// [`compile_file_macos_arm64`] for the end-to-end flow.
pub fn compile_macos_arm64_object(source: &str, module_name: &str) -> Result<Vec<u8>, AotError> {
    let module = twig_ir_compiler::compile_source(source, module_name)
        .map_err(|e| AotError::Compile(format!("{e}")))?;
    compile_module_macos_arm64_object(&module)
}

/// Compile an already-built `IIRModule` to Mach-O object-file bytes.
///
/// When the module references global variables (LANG39), the object file
/// is produced via [`pack_object_with_globals`] which adds a `__DATA/__data`
/// section and emits `ARM64_RELOC_PAGE21` / `ARM64_RELOC_PAGEOFF12`
/// relocation records so that the system linker (`ld`) can patch the
/// `ADRP + ADD` instruction pairs that address `_twig_globals`.
pub fn compile_module_macos_arm64_object(module: &IIRModule) -> Result<Vec<u8>, AotError> {
    compile_module_macos_arm64_object_with_mode(module, RefinementMode::Lenient)
}

/// Like [`compile_module_macos_arm64_object`] but with an explicit
/// [`RefinementMode`].
///
/// Use `RefinementMode::Strict` for `(typed strict)` modules (TW05-A) or any
/// pipeline that must treat `Unknown` solver outcomes as hard errors.
pub fn compile_module_macos_arm64_object_with_mode(
    module: &IIRModule,
    refinement_mode: RefinementMode,
) -> Result<Vec<u8>, AotError> {
    // ── LANG42: refinement obligation pass ──────────────────────────────────
    //
    // Run the pass on the *original* IIR before any lowering.  After
    // `prepare_module_for_aot` runs, variable names are rewritten and the
    // correspondence between names and annotations is lost — so we must check
    // before we mutate the module.
    //
    // `check_refinements` returns an empty Vec if there are no violations.
    let refinement_errors = check_refinements(module, refinement_mode);
    if !refinement_errors.is_empty() {
        return Err(AotError::RefinementViolations(refinement_errors));
    }

    let entry = module.entry_point.as_deref().ok_or(AotError::NoEntryPoint)?;
    let (text, offsets, n_global_slots, global_relocs, extern_relocs) =
        compile_module_to_text(module)?;
    // The image's `_main` is the GC entry wrapper (`__gc_aot_entry`), which registers
    // the module's stack maps and then runs the user entry (see
    // `build_gc_entry_functions`). Fall back to the user entry only if the wrapper is
    // somehow absent, so the pipeline degrades gracefully.
    let entry_off = offsets
        .get(GC_AOT_ENTRY)
        .copied()
        .unwrap_or_else(|| entry_point_offset(&offsets, Some(entry)));

    // Use pack_object_with_globals_and_externals for all cases (LANG41).
    //
    // This function always produces a two-section Mach-O (text + data).
    // When neither globals nor externals are present the data section is
    // empty and the reloc/symbol tables are minimal — functionally
    // equivalent to the old `pack_object` path.
    //
    // External BL targets (e.g. `__twig_print_i64`) become N_UNDF | N_EXT
    // symbol-table entries + ARM64_RELOC_BRANCH26 records so the system
    // linker can patch them from the embedded runtime archive.
    pack_object_with_globals_and_externals(
        &text,
        entry_off,
        n_global_slots,
        &global_relocs,
        &extern_relocs,
        &Target::macos_arm64(),
    )
    .map_err(|e| AotError::Packager(format!("{e}")))
}

/// Returns raw ARM64 machine code bytes and a function-name→byte-offset map.
///
/// Uses the standard untyped prep pipeline (pre-lower + i64 normalization +
/// type propagation + default-any-to-i64).  The resulting bytes are a flat
/// code section with no Mach-O wrapping — suitable for in-process execution
/// via [`call_arm64_function_in_process`].
///
/// Global-variable relocation metadata (`n_global_slots`, `global_byte_relocs`)
/// is intentionally discarded here; callers that need a full Mach-O object
/// should use [`compile_module_macos_arm64_object`] instead.
pub fn compile_module_to_arm64_bytes(
    module: &IIRModule,
) -> Result<(Vec<u8>, HashMap<String, usize>), AotError> {
    // Extern relocs (5th element) are discarded for in-process execution:
    // the JIT path does not link against the runtime archive, so `io_out`
    // BL placeholders remain unpatched.  Programs using `io_out` must
    // use the AOT path (compile_module_macos_arm64_object) to run.
    let (text, offsets, _, _, _) = compile_module_to_text(module)?;
    Ok((text, offsets))
}

/// Like [`compile_module_to_arm64_bytes`] but uses the caller-supplied type
/// annotations instead of forcing everything to `u64`.
///
/// The caller is responsible for having:
/// 1. Run `pre_lower_aot_builtins_on_module` (or equivalent).
/// 2. Run a type-inference pass (e.g. `iir-type-checker::infer_and_check`).
/// 3. Set any remaining `"any"` params to a concrete type (e.g. `"i64"`).
///
/// This function then runs `propagate_aot_types` (seeded from the caller's
/// type annotations) and `default_any_to_u64` to fill in any gaps, without
/// ever running `normalize_params_to_u64`.  Because params stay at whatever
/// type the caller gave them (e.g. `i64`), arithmetic and comparisons are
/// emitted with signed `i64` semantics — correct for negative numbers.
///
/// ## Why propagation is still needed
///
/// `iir-type-checker::infer_function` seeds its SSA environment only from
/// instruction dests with existing type hints — it does **not** seed from
/// `func.params`.  As a result, instructions of the form
/// `sub dest, param_var, const_var` stay `"any"` because `param_var` is not
/// found in the environment.  `propagate_aot_types` seeds from `func.params`
/// and picks up these remaining `"any"` instructions.
pub fn compile_typed_module_to_arm64_bytes(
    module: &IIRModule,
) -> Result<(Vec<u8>, HashMap<String, usize>), AotError> {
    let mut module = module.clone();
    for func in &mut module.functions {
        pre_lower_aot_builtins(func);
        // Promote any params still marked "any" or "polymorphic" to "i64".
        //
        // The caller may have already set some params to "i64" from source-level
        // annotations (e.g. `(x : int)`).  `normalize_params_to_i64` only touches
        // params that are STILL "any" — so caller-set types are never overwritten.
        // This ensures that unannotated functions (no param_refinements) also get
        // signed-integer semantics, matching Twig's semantic model.
        normalize_params_to_i64(func);
        // Propagate types seeded from the (now-concrete) params.
        // This fills in instructions like `sub dest, param, const` that
        // iir-type-checker missed because it doesn't seed from func.params.
        propagate_aot_types(func);
        // Default any still-unresolvable arithmetic/mov hints to i64.
        // (Should be rare after propagation with typed params.)
        default_any_to_i64(func);
    }
    // Discard global and extern reloc metadata — typed callers that need a full
    // Mach-O object should use `compile_module_macos_arm64_object` instead.
    let (text, offsets, _, _, _) = compile_module_to_text_raw(&module)?;
    Ok((text, offsets))
}

/// Apply the AOT builtin pre-lowering pass to every function in `module`.
///
/// This converts `call_builtin "+" a b` → `add a b`, `call_builtin "<" a b`
/// → `cmp_lt a b`, etc.  It is exposed so callers can pre-lower an IIR
/// module before running their own type-inference pass.
pub fn pre_lower_aot_builtins_on_module(module: &mut IIRModule) {
    for func in &mut module.functions {
        pre_lower_aot_builtins(func);
    }
}

/// Execute compiled ARM64 code in-process by mapping it into executable memory,
/// calling `fn_name(arg)`, and returning the result.
///
/// ## How it works
///
/// macOS does not allow `mmap` of file-backed pages with `PROT_EXEC` from
/// user code — that path requires the `com.apple.security.cs.allow-jit`
/// entitlement and fails with `EPERM` otherwise.  The simpler approach:
///
/// 1. Allocate an anonymous `PROT_READ | PROT_WRITE` page via `mmap`.
/// 2. `memcpy` the ARM64 bytes into the mapping.
/// 3. Call `mprotect` to change protection to `PROT_READ | PROT_EXEC`.
///    This is permitted on macOS without any special entitlement.
/// 4. Cast the byte at `fn_name`'s offset to `extern "C" fn(i64) -> i64`
///    and call it.
/// 5. `munmap` the region.
///
/// The two-step write-then-protect pattern is the standard JIT technique
/// on hardened macOS without `MAP_JIT`.
///
/// ## Calling convention
///
/// The compiled function must follow AAPCS64: argument in `x0`, result in `x0`.
/// This matches how the twig AOT backend generates function prologues.
///
/// ## Platform
///
/// macOS/ARM64 only.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn call_arm64_function_in_process(
    code: &[u8],
    offsets: &HashMap<String, usize>,
    fn_name: &str,
    arg: i64,
) -> Result<i64, AotError> {
    extern "C" {
        fn mmap(
            addr: *mut std::ffi::c_void,
            len: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            offset: i64,
        ) -> *mut std::ffi::c_void;
        fn munmap(addr: *mut std::ffi::c_void, len: usize) -> i32;
        fn mprotect(addr: *mut std::ffi::c_void, len: usize, prot: i32) -> i32;
    }
    const PROT_READ:  i32 = 0x01;
    const PROT_WRITE: i32 = 0x02;
    const PROT_EXEC:  i32 = 0x04;
    const MAP_PRIVATE: i32 = 0x0002;
    const MAP_ANON:    i32 = 0x1000; // macOS anonymous mapping flag

    let fn_offset = offsets.get(fn_name).copied().ok_or_else(|| AotError::BackendRefused {
        function: fn_name.to_string(),
    })?;

    // Round length up to a multiple of the page size (4096 on ARM64).
    let len = (code.len() + 4095) & !4095;

    // Step 1: allocate a writable anonymous mapping.
    let ptr = unsafe {
        mmap(
            std::ptr::null_mut(),
            len,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON,
            -1,
            0,
        )
    };
    // mmap returns MAP_FAILED = (void*)-1 = usize::MAX on failure.
    if ptr as usize == usize::MAX {
        return Err(AotError::Linker {
            status: None,
            stderr: "mmap(PROT_WRITE) failed for in-process ARM64 execution".into(),
        });
    }

    // Step 2: copy code bytes into the mapping.
    unsafe {
        std::ptr::copy_nonoverlapping(code.as_ptr(), ptr as *mut u8, code.len());
    }

    // Step 3: make the mapping executable (read + exec, no write).
    // This is permitted on macOS without MAP_JIT or any entitlement.
    let rc = unsafe { mprotect(ptr, len, PROT_READ | PROT_EXEC) };
    if rc != 0 {
        unsafe { munmap(ptr, len) };
        return Err(AotError::Linker {
            status: None,
            stderr: "mprotect(PROT_EXEC) failed for in-process ARM64 execution".into(),
        });
    }

    // Step 4: validate fn_offset is within the code buffer before pointer
    // arithmetic, then jump to it and call the function.
    // AAPCS64: argument in x0, result in x0.
    if fn_offset >= code.len() {
        // Offset out of range — compiler bug or mismatched (code, offsets) pair.
        // munmap before returning to avoid leaking the executable mapping.
        unsafe { munmap(ptr, len) };
        return Err(AotError::BackendRefused {
            function: format!(
                "{fn_name}: offset {fn_offset} >= code length {}",
                code.len()
            ),
        });
    }
    let fn_ptr = (ptr as *const u8).wrapping_add(fn_offset);
    let f: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(fn_ptr) };
    let result = f(arg);

    unsafe { munmap(ptr, len) };
    Ok(result)
}

/// Compile a Twig source file to a runnable ARM64 Mach-O executable on
/// disk by:
///
/// 1. Generating the `.o` object file (`compile_macos_arm64_object`).
/// 2. Writing it to a temp file.
/// 3. Invoking `ld` to produce the final executable at `out_path`.
/// 4. Marking the output `0o755`.
///
/// See [`AotError::Linker`] for `ld` invocation failures.
///
/// **Platform note:** this function is a no-op stub on non-Unix platforms
/// (Windows) — it always returns [`AotError::Linker`] with a helpful
/// message.  Cross-compiling for macOS from Windows is not yet supported.
#[cfg(not(unix))]
pub fn compile_file_macos_arm64(
    _src_path: &Path,
    _out_path: &Path,
) -> Result<(), AotError> {
    Err(AotError::Linker {
        status: None,
        stderr: "twig-aot: native macOS compilation requires a Unix host \
                 (macOS or Linux with a cross-toolchain)"
            .to_string(),
    })
}

/// Compile a Twig source file to a runnable ARM64 Mach-O executable on
/// disk (Unix-only implementation).
///
/// See the stub above for the non-Unix signature.
#[cfg(unix)]
pub fn compile_file_macos_arm64(
    src_path: &Path,
    out_path: &Path,
) -> Result<(), AotError> {
    let source = std::fs::read_to_string(src_path)?;
    let stem = src_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("twig");
    let object_bytes = compile_macos_arm64_object(&source, stem)?;
    link_macos_arm64_executable(&object_bytes, stem, out_path)
}

/// Compile a pre-built `IIRModule` to a macOS ARM64 Mach-O executable.
///
/// Frontend-agnostic counterpart to `compile_file_macos_arm64`.  Lets
/// non-Twig frontends (Nib, Brainfuck, …) reuse twig-aot's macOS path.
#[cfg(unix)]
pub fn compile_module_to_macos_executable(
    module: &IIRModule,
    out_path: &Path,
) -> Result<(), AotError> {
    let object_bytes = compile_module_macos_arm64_object(module)?;
    let stem = out_path.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    link_macos_arm64_executable(&object_bytes, stem, out_path)
}

/// Stub for non-Unix hosts.
#[cfg(not(unix))]
pub fn compile_module_to_macos_executable(
    _module: &IIRModule,
    _out_path: &Path,
) -> Result<(), AotError> {
    Err(AotError::Linker {
        status: None,
        stderr: "twig-aot: native macOS compilation requires a Unix host".into(),
    })
}

/// Link a macOS ARM64 Mach-O object file (produced by any frontend)
/// into a runnable executable via the system `ld`.
#[cfg(unix)]
pub fn link_macos_arm64_executable(
    obj_bytes: &[u8],
    stem: &str,
    out_path: &Path,
) -> Result<(), AotError> {
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt;

    let mut tmp_obj = tempfile::Builder::new()
        .prefix(&format!("twig-aot-{stem}-"))
        .suffix(".o")
        .tempfile()?;
    tmp_obj.write_all(obj_bytes)?;
    invoke_ld(tmp_obj.path(), out_path)?;
    drop(tmp_obj);

    let mut perms = std::fs::metadata(out_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(out_path, perms)?;
    Ok(())
}

/// Run Apple's system linker on `object_path`, producing `out_path`.
///
/// The arguments are deliberately conservative:
/// - `-arch arm64`            — explicit target arch
/// - `-platform_version macos 15.0 15.0` — minOS + SDK declaration
/// - `-e _main`               — entry symbol (matches our object's symtab)
/// - `<runtime.a>`            — Twig AOT runtime archive (provides `__twig_print_i64` etc.)
/// - `-lSystem`               — macOS C runtime (printf, exit, …)
/// - `-o <out>`               — output path
///
/// We intentionally **do not** pass `-static`.  Modern macOS heavily
/// privileges binaries that link against `libSystem` (the standard C
/// runtime) — they get the trusted toolchain provenance and pass the
/// kernel's security policy.
///
/// The Twig AOT runtime archive (`libtwig_aot_runtime.a`, compiled from
/// `runtime/twig_runtime.c` by `build.rs`) is embedded in this binary as
/// [`RUNTIME_ARCHIVE`] and written to a temp file for each link invocation.
/// `ld` extracts whatever symbols it needs (e.g. `__twig_print_i64`) from
/// the archive; the static-archive format means only referenced object files
/// are pulled in, so programs that don't use `io_out` don't pay for the
/// print helper.
///
/// `ld` sets up: `LC_LOAD_DYLINKER`, `LC_LOAD_DYLIB libSystem`,
/// `LC_DYLD_CHAINED_FIXUPS`, ad-hoc code signature, etc.
fn invoke_ld(object_path: &Path, out_path: &Path) -> Result<(), AotError> {
    use std::io::Write as _;

    // Write the embedded runtime archive to a secure temp file.
    //
    // Security: we use `tempfile::Builder` (O_EXCL + random suffix) rather than
    // a PID-derived path.  A PID-based name is predictable and can be raced by
    // an attacker with write access to `$TMPDIR` (TOCTOU / symlink attack):
    //
    // - Symlink write-through: if a symlink at the predicted path already exists,
    //   `fs::write` would follow it and overwrite an arbitrary file.
    // - Replace-after-write: the attacker races to replace the just-written
    //   archive with a malicious one before `ld` opens it, injecting arbitrary
    //   machine code into the produced binary.
    //
    // `NamedTempFile` prevents both: the kernel creates the file atomically with
    // `O_EXCL`, and the random name is not guessable.  The file is deleted when
    // `runtime_tmp` drops (after `ld` exits below).
    let mut runtime_tmp = tempfile::Builder::new()
        .prefix("twig_aot_runtime_")
        .suffix(".a")
        .tempfile()?;
    runtime_tmp.write_all(RUNTIME_ARCHIVE)?;

    // The GC archive (gc-core-capi) is written to a second temp `.a` and passed
    // to `ld` *after* the runtime archive (#118b-2b). Ordering matters for a
    // one-pass archive linker: `dynval_runtime.o` (in the runtime archive) has
    // the undefined `__twig_gc_alloc`, so the runtime archive must be seen first
    // and the GC archive after, letting `ld` pull the collector member that
    // satisfies it. The file stays alive until after `ld` runs, then drops.
    let mut gc_tmp = tempfile::Builder::new()
        .prefix("twig_aot_gc_core_")
        .suffix(".a")
        .tempfile()?;
    gc_tmp.write_all(GC_CORE_ARCHIVE)?;

    // `-lSystem` is non-negotiable on modern macOS: `ld` refuses to
    // produce a dynamic executable without linking the C runtime.
    // The runtime archive itself uses `printf` (from libSystem), so
    // linking both is required and correct.
    //
    // `-L<sdk>/usr/lib` tells ld where to find `libSystem.tbd`.  We
    // probe `xcrun --sdk macosx --show-sdk-path` first, falling back
    // to the conventional `/usr/lib` if Xcode CLT isn't installed.
    let sdk_lib = sdk_lib_path();

    let output = std::process::Command::new("ld")
        .arg("-arch").arg("arm64")
        .arg("-platform_version").arg("macos").arg("15.0").arg("15.0")
        .arg("-e").arg("_main")
        .arg("-L").arg(&sdk_lib)
        .arg("-lSystem")
        .arg("-o").arg(out_path)
        .arg(object_path)
        .arg(runtime_tmp.path()) // runtime archive: provides __twig_print_i64 etc.
        .arg(gc_tmp.path())      // GC archive: provides __twig_gc_alloc / _safepoint
        .output()
        .map_err(|e| AotError::Linker {
            status: None,
            stderr: format!("ld not found on PATH or could not be spawned: {e}"),
        })?;

    // Temp archives drop here — NamedTempFile deletes them after `ld` exits.
    drop(runtime_tmp);
    drop(gc_tmp);

    if !output.status.success() {
        return Err(AotError::Linker {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

/// Locate `<sdk>/usr/lib` so `ld` can find `libSystem.tbd`.
///
/// First tries `xcrun --sdk macosx --show-sdk-path`, which works on any
/// machine with the Xcode Command Line Tools installed.  Falls back to
/// `/usr/lib` for machines where `xcrun` is missing or fails.
fn sdk_lib_path() -> PathBuf {
    if let Ok(o) = std::process::Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
    {
        if o.status.success() {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !s.is_empty() {
                return PathBuf::from(s).join("usr").join("lib");
            }
        }
    }
    PathBuf::from("/usr/lib")
}

// ===========================================================================
// x86-64 multi-target driver (LANG46 phase 2)
// ===========================================================================
//
// Mirrors the macOS ARM64 path above, but for Linux x86-64 (ELF + cc) and
// Windows x86-64 (PE/COFF + link.exe / lld-link / gcc).

use x86_64_backend::{
    compile_function_with_globals_and_stackmap as x86_64_compile_with_stackmap, X86_64Abi,
};

/// Per-function compile for x86-64, then concatenate function bytes into a
/// single `.text`, **patch cross-function call sites in place**, and surface
/// only the truly-external relocations for the system linker.
///
/// Mirrors `compile_module_to_text_raw`'s two-pass strategy from the
/// AArch64 path.  Returns `(linked_text, fn_offsets, entry_byte_offset,
/// external_relocs)`.
///
/// Pass strategy:
///
/// 1. Compile each function independently with `x86_64-backend`.  Every
///    cross-function `call` instruction emits a `CALL rel32` placeholder
///    (`E8 00 00 00 00`) and records an `ExternalReloc { kind: PltRel32,
///    symbol: <callee name>, patch_offset: <disp32 byte offset within fn> }`.
/// 2. Concatenate function bytes via `aot_core::link::link`, recording each
///    function's byte offset in the linked text.
/// 3. Walk every recorded reloc.  Lift its `patch_offset` to the
///    linked-text offset (function base + per-fn offset).  If the symbol
///    names another function in this module (`offsets.contains_key`),
///    patch the disp32 slot directly: `disp32 = callee_byte_offset -
///    patch_offset - 4`.  The reloc is consumed; the linker never sees it.
/// 4. Anything left (e.g. `__twig_print_i64`) is a true external; emit it
///    as an `X86RelocRecord` so the packager turns it into a `R_X86_64_PLT32`
///    / `IMAGE_REL_AMD64_REL32` record.
///
/// The PC-relative formula is the same one the linker would apply:
/// the encoder writes zero into the disp32 slot, then we set it to
/// `target_offset - patch_offset - 4` (the `-4` is the addend stored in
/// the reloc, and equals the byte distance from the disp32 slot to the
/// end of the instruction — `E8` opcode is 1 byte + 4-byte disp32, so
/// the instruction ends 4 bytes after the disp32 slot start).
// Return tuple bundles the emitted text bytes, the symbol→offset map, the text
// size, and the relocation list — a cohesive "compiled module" result; a named
// struct would add indirection without clarifying this internal helper.
/// Build the **x86-64 GC entry wrapper** [`GC_AOT_ENTRY`] — the image's real `main`.
///
/// The ELF/PE packager exports a global `main` symbol at whatever `entry_off` the
/// caller supplies (libc's `_start` calls `main`), so — exactly like the aarch64 Mach-O
/// entry redirect — making the wrapper the entry is a pure offset redirect; no rename.
///
/// ```text
///   __gc_aot_entry (exported as `main`):
///       push rbp ; mov rbp, rsp
///       call __gc_init_stackmaps        ; register every function's stack map
///       call <user entry>              ; rax = program result
///       mov rsp, rbp ; pop rbp ; ret   ; return rax verbatim → crt0 → exit(rax)
/// ```
///
/// Both `call`s are intra-module `CALL rel32`s the two-pass linker patches in place.
/// `rsp` is 16-aligned at each call (crt0 enters `main` 16-aligned-minus-8; `push rbp`
/// restores alignment; a 32-byte reservation keeps it aligned). The user entry's `rax`
/// survives the epilogue. `call __gc_init_stackmaps` runs before the user entry and a
/// twig `main` takes no arguments, so clobbering caller-saved registers is harmless.
///
/// Under **MsX64** (Windows) the wrapper reserves the 32-byte **shadow space** every
/// caller must give a callee, matching the rest of the backend (`shadow_space()`); SysV
/// needs none. The reservation is a 16-byte multiple, so alignment holds either way.
fn build_gc_wrapper_x86_64(
    user_entry: &str,
    abi: X86_64Abi,
) -> Result<(Vec<u8>, Vec<x86_64_encoder::ExternalReloc>), AotError> {
    use x86_64_encoder::{Assembler, ExternalRelocKind, Reg};
    // Win64 mandates a 32-byte caller-provided shadow (home) space; SysV needs none.
    let shadow: i32 = match abi {
        X86_64Abi::MsX64 => 32,
        X86_64Abi::SysV => 0,
    };
    let mut a = Assembler::new();
    a.push(Reg::Rbp);
    a.mov_r64_r64(Reg::Rbp, Reg::Rsp);
    if shadow != 0 {
        a.sub_imm32(Reg::Rsp, shadow); // Win64 shadow / home space for the two calls
    }
    a.call_rel32(GC_INIT_STACKMAPS, ExternalRelocKind::PltRel32);
    a.call_rel32(user_entry, ExternalRelocKind::PltRel32);
    a.mov_r64_r64(Reg::Rsp, Reg::Rbp); // deallocates the shadow reservation
    a.pop(Reg::Rbp);
    a.ret();
    let relocs = std::mem::take(&mut a.external_relocs);
    let bytes = a.finish().map_err(|e| AotError::Linker {
        status: None,
        stderr: format!("twig-aot: x86_64 GC wrapper codegen: {e:?}"),
    })?;
    Ok((bytes, relocs))
}

/// Build the **no-op** `__gc_init_stackmaps` for x86-64 (a bare `ret`). A later PR
/// (AOT00-T1 x86_64 PR-x3) fills it in to register each function's stack map; this
/// increment lands the wrapper mechanism and proves it transparent first.
fn build_gc_noop_init_x86_64() -> Result<Vec<u8>, AotError> {
    let mut a = x86_64_encoder::Assembler::new();
    a.ret();
    a.finish().map_err(|e| AotError::Linker {
        status: None,
        stderr: format!("twig-aot: x86_64 GC init codegen: {e:?}"),
    })
}

/// One `LEA r64, [RIP+…]` in the x86-64 `__gc_init_stackmaps` that must be patched to a
/// user function's runtime address (`func_start`, the first `__gc_register_stackmap`
/// argument). `RIP`-relative in bytes, so the displacement is a link-time constant
/// independent of the load base (see [`FuncAddrReloc`] for the aarch64 rationale — the
/// `ADR` analogue); patched in pass 2b, no relocation.
struct FuncAddrRelocX86 {
    /// Byte offset (within `__gc_init_stackmaps`) of the `LEA`'s `disp32` slot.
    disp_slot: usize,
    /// Name of the user function whose address this `LEA` loads.
    target: String,
}

/// Map an [`x86_64_encoder::EncodeError`] to an [`AotError`].
fn gc_asm_err_x86(e: x86_64_encoder::EncodeError) -> AotError {
    AotError::Linker { status: None, stderr: format!("twig-aot: x86_64 GC init codegen: {e:?}") }
}

/// Return byte offset of every call-return address in `relocs` — a `PltRel32` reloc's
/// `patch_offset + 4` (the byte after the 5-byte `CALL rel32`). The x86-64 way to
/// recover a function's safepoints (variable-width ISA → no post-scan); used to give
/// the synthetic wrapper its own (empty-slot) records so its frame resolves precisely.
fn call_return_offsets_x86_64(relocs: &[x86_64_encoder::ExternalReloc]) -> Vec<u32> {
    relocs
        .iter()
        .filter(|r| matches!(r.kind, x86_64_encoder::ExternalRelocKind::PltRel32))
        .map(|r| (r.patch_offset + 4) as u32)
        .collect()
}

/// Build the **SysV** x86-64 `__gc_init_stackmaps`: for every function with records,
/// marshal the eight `__gc_register_stackmap` arguments and call it. (Windows/MsX64
/// keeps the no-op init — its precise walk is a follow-up — so Windows degrades to a
/// safe conservative collection.)
///
/// ```text
///   __gc_init_stackmaps:
///       push rbp ; mov rbp, rsp
///       ; ── per function F with records (System V) ──
///       lea  rdi, [rip + F]          ; func_start   (patched in pass 2b)
///       mov  rsi, <F byte length>    ; func_len
///       mov  rdx, <record count>     ; num_records
///       lea  rcx, [rip + F.pc]       ; pc_offsets[]
///       xor  r8, r8                  ; frame_sizes  = NULL
///       xor  r9, r9                  ; callee_masks = NULL
///       lea  rax,[rip+F.slots|0]; push rax   ; arg8 slots_flat  (16-aligned: 2 pushes)
///       lea  rax,[rip+F.counts]; push rax    ; arg7 slot_counts
///       call __gc_register_stackmap
///       add  rsp, 16
///       ; ── … next function … ──
///       mov rsp,rbp ; pop rbp ; ret
///   <data pool: each function's pc_offsets / slot_counts / slots_flat words>
/// ```
///
/// `rsp` is 16-aligned at each `call` (`push rbp` aligns; the two argument pushes add
/// 16). Everything but `func_start` is a compile-time constant; the three arrays live in
/// a pool after the final `ret` (never executed), addressed by `RIP`-relative `lea`
/// resolved at `finish()`. The registry copies the slots, so the pool need not outlive
/// the call.
#[allow(clippy::type_complexity)]
fn build_gc_init_stackmaps_x86_64(
    fn_maps: &[FnStackMap],
) -> Result<(Vec<u8>, Vec<x86_64_encoder::ExternalReloc>, Vec<FuncAddrRelocX86>), AotError> {
    use x86_64_encoder::{Assembler, LabelId, Reg};
    let mut a = Assembler::new();
    let mut faddr: Vec<FuncAddrRelocX86> = Vec::new();
    let mut pool: Vec<(LabelId, Vec<u32>)> = Vec::new();

    a.push(Reg::Rbp);
    a.mov_r64_r64(Reg::Rbp, Reg::Rsp);

    for fm in fn_maps {
        if fm.records.is_empty() {
            continue; // no safepoints → nothing to register (conservative frame)
        }
        let mut pc_offsets: Vec<u32> = Vec::with_capacity(fm.records.len());
        let mut slot_counts: Vec<u32> = Vec::with_capacity(fm.records.len());
        let mut slots_flat: Vec<u32> = Vec::new();
        for (pc, slots) in &fm.records {
            pc_offsets.push(*pc);
            slot_counts.push(slots.len() as u32);
            slots_flat.extend(slots.iter().map(|&s| s as u32)); // i32 → u32 bit-cast
        }
        let pc_lbl = a.create_label();
        let cnt_lbl = a.create_label();
        let slots_lbl = if slots_flat.is_empty() { None } else { Some(a.create_label()) };

        // Register args 1–6.
        let disp_slot = a.lea_rip_placeholder(Reg::Rdi); // func_start (patched pass 2b)
        faddr.push(FuncAddrRelocX86 { disp_slot, target: fm.name.clone() });
        a.mov_r64_imm64(Reg::Rsi, fm.len as u64); //          func_len
        a.mov_r64_imm64(Reg::Rdx, fm.records.len() as u64); // num_records
        a.lea_rip_label(Reg::Rcx, pc_lbl); //                 pc_offsets
        a.xor_(Reg::R8, Reg::R8); //                          frame_sizes  = NULL
        a.xor_(Reg::R9, Reg::R9); //                          callee_masks = NULL
        // Stack args 7,8 — push arg8 then arg7 so [rsp]=arg7, [rsp+8]=arg8 (SysV).
        match slots_lbl {
            Some(l) => a.lea_rip_label(Reg::Rax, l), // arg8 = slots_flat
            None => a.xor_(Reg::Rax, Reg::Rax), //     arg8 = NULL
        }
        a.push(Reg::Rax);
        a.lea_rip_label(Reg::Rax, cnt_lbl); //         arg7 = slot_counts
        a.push(Reg::Rax);
        a.call_rel32(GC_REGISTER_STACKMAP, x86_64_encoder::ExternalRelocKind::PltRel32);
        a.add_imm32(Reg::Rsp, 16); // pop the two stack args

        pool.push((pc_lbl, pc_offsets));
        pool.push((cnt_lbl, slot_counts));
        if let Some(l) = slots_lbl {
            pool.push((l, slots_flat));
        }
    }

    a.mov_r64_r64(Reg::Rsp, Reg::Rbp);
    a.pop(Reg::Rbp);
    a.ret();

    // Constant data pool — read via `lea` above, never executed (after `ret`).
    for (lbl, words) in &pool {
        a.bind(*lbl).map_err(gc_asm_err_x86)?;
        for &w in words {
            a.emit_data_u32(w);
        }
    }

    let relocs = std::mem::take(&mut a.external_relocs);
    Ok((a.finish().map_err(gc_asm_err_x86)?, relocs, faddr))
}

#[allow(clippy::type_complexity)]
fn compile_module_x86_64_to_text(
    module: &IIRModule,
    abi: X86_64Abi,
) -> Result<(Vec<u8>, HashMap<String, usize>, usize, Vec<X86RelocRecord>), AotError> {
    let global_slots = collect_global_slots(module);

    // Pass 1: compile each function with the x86_64 backend.
    let mut fn_results: Vec<(String, Vec<u8>, Vec<x86_64_encoder::ExternalReloc>)> =
        Vec::with_capacity(module.functions.len());

    // Per user function, the GC stack map the SysV registration function will register
    // (collected only for SysV; MsX64 uses the no-op init, so records are unused there).
    let mut fn_maps: Vec<FnStackMap> = Vec::with_capacity(module.functions.len());
    for fn_ in &module.functions {
        let ctx = FunctionContext {
            name: &fn_.name,
            params: &fn_.params,
            return_type: &fn_.return_type,
        };
        let inferred = infer_types(fn_);
        let cir = aot_specialise(fn_, Some(&inferred));
        let (bytes, relocs, stack_map) =
            x86_64_compile_with_stackmap(&ctx, &cir, abi, &global_slots)
                .map_err(|_| AotError::BackendRefused { function: fn_.name.clone() })?;
        let records = stack_map.into_iter().map(|r| (r.pc_offset, r.slots)).collect();
        fn_maps.push(FnStackMap { name: fn_.name.clone(), len: bytes.len(), records });
        fn_results.push((fn_.name.clone(), bytes, relocs));
    }

    // ── Inject the GC entry wrapper + (no-op) stack-map registration (AOT00-T1
    //    x86_64 PR-x2) ──────────────────────────────────────────────────────────
    //
    // Mirrors the aarch64 path: the wrapper `__gc_aot_entry` becomes the image entry
    // (the packager exports the global `main` symbol at `entry_off`, so redirecting
    // `entry_off` to the wrapper's offset makes it `main` — no rename). It calls the
    // (currently no-op) `__gc_init_stackmaps`, then the user entry, returning its rax.
    // Both calls are intra-module and patched in place by pass 2 below.
    let entry = module.entry_point.as_deref().ok_or(AotError::NoEntryPoint)?;
    // Reserved-symbol guard: `link()` is last-write-wins and the wrapper emits
    // `call <entry>`, so a user function OR entry named `__gc_aot_entry` /
    // `__gc_init_stackmaps` would shadow the synthetic symbol or make the wrapper call
    // itself/the no-op init. Reject rather than miscompile (same as the aarch64 path).
    for reserved in [GC_AOT_ENTRY, GC_INIT_STACKMAPS] {
        if entry == reserved || fn_results.iter().any(|(name, _, _)| name == reserved) {
            return Err(AotError::Linker {
                status: None,
                stderr: format!(
                    "twig-aot: symbol '{reserved}' is reserved for the GC entry wrapper \
                     and cannot be a module function or entry point",
                ),
            });
        }
    }
    // Build the wrapper first (both ABIs), then — on SysV — register it too (empty
    // ref-slot map) so the precise walk never conservatively re-scans the user entry's
    // frame (the increment-C fix; [[feedback_precise_walk_maps_every_frame_in_chain]]).
    let (wrapper_bytes, wrapper_relocs) = build_gc_wrapper_x86_64(entry, abi)?;

    // `__gc_init_stackmaps`: real registration on System V; a no-op on Windows/MsX64
    // (whose precise walk is a follow-up), so Windows degrades to a safe conservative
    // collection. `fn_addr_relocs` are the SysV `func_start` LEAs pass 2b patches.
    let (init_bytes, init_relocs, fn_addr_relocs) = if matches!(abi, X86_64Abi::SysV) {
        let wrapper_records = call_return_offsets_x86_64(&wrapper_relocs)
            .into_iter()
            .map(|pc| (pc, Vec::new()))
            .collect();
        fn_maps.push(FnStackMap {
            name: GC_AOT_ENTRY.to_string(),
            len: wrapper_bytes.len(),
            records: wrapper_records,
        });
        build_gc_init_stackmaps_x86_64(&fn_maps)?
    } else {
        (build_gc_noop_init_x86_64()?, Vec::new(), Vec::new())
    };
    fn_results.push((GC_INIT_STACKMAPS.to_string(), init_bytes, init_relocs));
    fn_results.push((GC_AOT_ENTRY.to_string(), wrapper_bytes, wrapper_relocs));

    // Concatenate function bytes and record per-function offsets.
    let plain: Vec<(String, Vec<u8>)> = fn_results.iter()
        .map(|(name, bytes, _)| (name.clone(), bytes.clone()))
        .collect();
    let (mut linked, offsets) = link(&plain);

    // The image's `main` is the GC entry wrapper; fall back to the user entry only if
    // the wrapper is somehow absent, so the pipeline degrades gracefully.
    let entry_off = offsets
        .get(GC_AOT_ENTRY)
        .copied()
        .unwrap_or_else(|| entry_point_offset(&offsets, Some(entry)));

    // Pass 2: lift per-function reloc offsets into linked-text offsets, and
    // patch cross-function calls in place.  Keep only truly-external relocs
    // (e.g. `__twig_print_i64`) for the packager.
    let mut external_relocs: Vec<X86RelocRecord> = Vec::new();
    for (fn_name, _bytes, fn_relocs) in &fn_results {
        let base = *offsets.get(fn_name.as_str())
            .ok_or_else(|| AotError::Linker {
                status: None,
                stderr: format!(
                    "twig-aot: internal error: function '{fn_name}' missing from \
                     link offsets during x86_64 reloc lift",
                ),
            })?;
        for r in fn_relocs {
            let patch_offset_linked = base + r.patch_offset;

            // Is this a call into a function defined in the SAME module?
            // Only `PltRel32` relocs (CALL rel32) qualify for in-place
            // patching; `PcRel32`/`GotPcRel32` are RIP-relative loads/stores
            // that target globals or external data, never internal function
            // bodies, so they always pass through to the packager.
            let internal_call = matches!(r.kind, x86_64_encoder::ExternalRelocKind::PltRel32)
                && offsets.contains_key(r.symbol.as_str());

            if internal_call {
                let callee_off = *offsets.get(r.symbol.as_str()).unwrap();
                // PC-relative displacement from the *end* of the
                // instruction.  patch_offset names the disp32 slot's
                // first byte; the end of the instruction is 4 bytes
                // after that.
                let disp: i64 = (callee_off as i64) - (patch_offset_linked as i64) - 4;
                if !(i32::MIN as i64..=i32::MAX as i64).contains(&disp) {
                    return Err(AotError::Linker {
                        status: None,
                        stderr: format!(
                            "twig-aot: cross-function call displacement {disp} \
                             from '{fn_name}' to '{}' exceeds 32-bit range",
                            r.symbol),
                    });
                }
                let bytes = (disp as i32).to_le_bytes();
                linked[patch_offset_linked    ] = bytes[0];
                linked[patch_offset_linked + 1] = bytes[1];
                linked[patch_offset_linked + 2] = bytes[2];
                linked[patch_offset_linked + 3] = bytes[3];
                // Reloc resolved in place; don't forward to packager.
                continue;
            }

            // External reloc — packager records it for the system linker.
            external_relocs.push(X86RelocRecord {
                patch_offset: patch_offset_linked as u32,
                symbol: r.symbol.clone(),
                kind: match r.kind {
                    x86_64_encoder::ExternalRelocKind::PltRel32   => X86RelocKind::PltRel32,
                    x86_64_encoder::ExternalRelocKind::PcRel32    => X86RelocKind::PcRel32,
                    x86_64_encoder::ExternalRelocKind::GotPcRel32 => X86RelocKind::GotPcRel32,
                },
                addend: r.addend,
            });
        }
    }

    // ── Pass 2b: patch `__gc_init_stackmaps`' func_start LEAs (SysV) ───────────
    //
    // Each `lea rdi, [rip + disp32]` loads a user function's absolute runtime address.
    // `LEA` computes `RIP + disp32`; `RIP` (= instruction end) and the target are both
    // `base + offset`, so `disp32 = target_off − (slot + 4)` is correct for any load
    // base — the same base-independence the aarch64 `ADR` func_start relies on. No `ld`
    // relocation. (Empty on MsX64, whose init is the no-op.)
    if !fn_addr_relocs.is_empty() {
        let init_off = *offsets.get(GC_INIT_STACKMAPS).ok_or_else(|| AotError::Linker {
            status: None,
            stderr: "twig-aot: internal error: __gc_init_stackmaps missing from link offsets"
                .to_string(),
        })?;
        for r in &fn_addr_relocs {
            let target_off = *offsets.get(r.target.as_str()).ok_or_else(|| AotError::Linker {
                status: None,
                stderr: format!(
                    "twig-aot: internal error: func_start target '{}' missing from link offsets",
                    r.target
                ),
            })?;
            let slot = init_off + r.disp_slot; // disp32 slot in the linked text
            // disp32 = target − RIP, RIP = end of the LEA = slot + 4.
            let disp = target_off as i64 - (slot as i64 + 4);
            if !(i32::MIN as i64..=i32::MAX as i64).contains(&disp) {
                return Err(AotError::Linker {
                    status: None,
                    stderr: format!(
                        "twig-aot: func_start LEA displacement {disp} for '{}' exceeds 32-bit range",
                        r.target
                    ),
                });
            }
            linked[slot..slot + 4].copy_from_slice(&(disp as i32).to_le_bytes());
        }
    }

    Ok((linked, offsets, entry_off, external_relocs))
}

/// Compile an `IIRModule` to a Linux x86-64 ELF object file (`*.o`).
///
/// Returns the raw object bytes ready to hand to `cc` / `ld`.
pub fn compile_module_linux_x86_64_object(module: &IIRModule) -> Result<Vec<u8>, AotError> {
    let mut module = module.clone();
    prepare_module_for_aot(&mut module);
    let global_slots = collect_global_slots(&module);
    let (text, _, entry_off, relocs) =
        compile_module_x86_64_to_text(&module, X86_64Abi::SysV)?;
    pack_elf64_object_x86_64(
        &text, entry_off, global_slots.len(), &relocs, &Target::linux_x64(),
    ).map_err(|e| AotError::Packager(format!("{e}")))
}

/// Convenience: compile Twig source text directly to an ELF object.
pub fn compile_linux_x86_64_object(source: &str, module_name: &str) -> Result<Vec<u8>, AotError> {
    let module = twig_ir_compiler::compile_source(source, module_name)
        .map_err(|e| AotError::Compile(format!("{e}")))?;
    compile_module_linux_x86_64_object(&module)
}

/// Compile an `IIRModule` to a Windows x86-64 PE/COFF object file (`*.obj`).
pub fn compile_module_windows_x86_64_object(module: &IIRModule) -> Result<Vec<u8>, AotError> {
    let mut module = module.clone();
    prepare_module_for_aot(&mut module);
    let global_slots = collect_global_slots(&module);
    let (text, _, entry_off, relocs) =
        compile_module_x86_64_to_text(&module, X86_64Abi::MsX64)?;
    pack_pe_object_x86_64(
        &text, entry_off, global_slots.len(), &relocs, &Target::windows_x64(),
    ).map_err(|e| AotError::Packager(format!("{e}")))
}

/// Convenience: compile Twig source text directly to a PE/COFF object.
pub fn compile_windows_x86_64_object(source: &str, module_name: &str) -> Result<Vec<u8>, AotError> {
    let module = twig_ir_compiler::compile_source(source, module_name)
        .map_err(|e| AotError::Compile(format!("{e}")))?;
    compile_module_windows_x86_64_object(&module)
}

// ---------------------------------------------------------------------------
// Cross-OS object emission (works from any host — no linking step)
// ---------------------------------------------------------------------------

/// Logical target the `emit_object_to_disk` helper writes for.
///
/// `MacosArm64` is included for completeness; the macOS pipeline goes
/// through `aarch64-backend` + `macho_object` rather than the x86_64
/// path, but we still expose object emission so all three targets have
/// a consistent --emit-object surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitObjectTarget {
    /// macOS ARM64 (`.o` Mach-O object — same as `compile_module_macos_arm64_object`).
    MacosArm64,
    /// Linux x86-64 (`.o` ELF64 object — works on any host).
    LinuxX86_64,
    /// Windows x86-64 (`.obj` PE/COFF object — works on any host).
    WindowsX86_64,
}

impl EmitObjectTarget {
    fn object_extension(self) -> &'static str {
        match self {
            EmitObjectTarget::MacosArm64    => "o",
            EmitObjectTarget::LinuxX86_64   => "o",
            EmitObjectTarget::WindowsX86_64 => "obj",
        }
    }
    fn runtime_extension(self) -> &'static str {
        match self {
            EmitObjectTarget::MacosArm64    => "a",
            EmitObjectTarget::LinuxX86_64   => "a",
            EmitObjectTarget::WindowsX86_64 => "lib",
        }
    }
    fn runtime_bytes(self) -> &'static [u8] {
        match self {
            EmitObjectTarget::MacosArm64    => RUNTIME_ARCHIVE,
            EmitObjectTarget::LinuxX86_64   => RUNTIME_LINUX_X86_64,
            EmitObjectTarget::WindowsX86_64 => RUNTIME_WINDOWS_X86_64,
        }
    }
}

/// Outputs produced by `emit_object_to_disk`.
///
/// `runtime_archive_path` is `None` when the target's runtime archive
/// is a stub on this build host (i.e. the host couldn't build it at
/// `twig-aot` crate compile time).  In that case the caller must
/// provide their own runtime when linking; the message printed by the
/// CLI says exactly that.
#[derive(Debug, Clone)]
pub struct EmittedObject {
    /// Where the object file was written.
    pub object_path: PathBuf,
    /// Where the runtime archive was written, if available on this host.
    pub runtime_archive_path: Option<PathBuf>,
    /// The target this object was produced for.
    pub target: EmitObjectTarget,
}

/// Emit a Twig program as a relocatable object file (`.o` / `.obj`) on
/// disk, plus the matching runtime archive if available on this build
/// host.  Works for any (host, target) combination — the only host-
/// dependent piece is whether the runtime archive was built at
/// `twig-aot` crate compile time.
///
/// The caller writes a separate link command (`cc`, `link.exe`, etc.)
/// against the resulting files on a machine that can host the target.
///
/// `out_base` is the path *without* an extension — `.o`/`.obj` and
/// `_runtime.a`/`_runtime.lib` are appended.  E.g. passing
/// `out_base = "build/foo"` for a Linux target writes:
/// - `build/foo.o`         (the object file)
/// - `build/foo_runtime.a` (the runtime archive, if available)
pub fn emit_object_to_disk(
    src: &Path,
    out_base: &Path,
    target: EmitObjectTarget,
) -> Result<EmittedObject, AotError> {
    use std::io::Write as _;
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("twig");

    // Produce the object bytes (no linking).
    let object_bytes = match target {
        EmitObjectTarget::MacosArm64    => compile_macos_arm64_object(&source, stem)?,
        EmitObjectTarget::LinuxX86_64   => compile_linux_x86_64_object(&source, stem)?,
        EmitObjectTarget::WindowsX86_64 => compile_windows_x86_64_object(&source, stem)?,
    };

    let object_path = with_extension(out_base, target.object_extension());
    if let Some(parent) = object_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut f = std::fs::File::create(&object_path)?;
    f.write_all(&object_bytes)?;
    drop(f);

    // Write the runtime archive when this host produced a real one at
    // build time.  Stub archives (one byte) are skipped — the caller
    // gets a clear message via the CLI.
    let rt_bytes = target.runtime_bytes();
    let runtime_archive_path = if rt_bytes.len() > 4 {
        let rt_path = {
            let mut p = out_base.to_path_buf();
            let name = p.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("twig")
                .to_owned();
            p.set_file_name(format!("{name}_runtime.{}", target.runtime_extension()));
            p
        };
        let mut f = std::fs::File::create(&rt_path)?;
        f.write_all(rt_bytes)?;
        Some(rt_path)
    } else {
        None
    };

    Ok(EmittedObject {
        object_path,
        runtime_archive_path,
        target,
    })
}

/// Helper: produce `<base>.<ext>` regardless of whether `base` already
/// has an extension.  (`Path::with_extension` *replaces* the extension,
/// which surprises a user who passes `out_base = "foo.exe"`.)
fn with_extension(base: &Path, ext: &str) -> PathBuf {
    let mut p = base.to_path_buf();
    let new_name = match p.file_name().and_then(|s| s.to_str()) {
        Some(name) => format!("{name}.{ext}"),
        None       => format!("twig.{ext}"),
    };
    p.set_file_name(new_name);
    p
}

// ---------------------------------------------------------------------------
// End-to-end pipelines: Twig source → object → system linker → executable
// ---------------------------------------------------------------------------

/// Compile a Twig source file to a runnable Linux x86-64 ELF executable.
///
/// Pipeline: source → IR → x86_64-backend (SysV ABI) → ELF object →
/// `cc` (the system C compiler) → executable.  Uses `cc` rather than
/// `ld` directly so that libc startup files (`crt1.o`, `crti.o`, etc.)
/// and dynamic-linker setup are handled correctly.
///
/// Requires a Linux build host (the embedded runtime archive is host-
/// specific per LANG46 phase 1).  Returns an error with a clear "no
/// Linux runtime archive on this host" message if invoked from a build
/// that wasn't done on Linux.
#[cfg(target_os = "linux")]
pub fn compile_file_linux_x86_64(src: &Path, out: &Path) -> Result<(), AotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("twig");
    let obj_bytes = compile_linux_x86_64_object(&source, stem)?;
    link_linux_x86_64_executable(&obj_bytes, stem, out)
}

/// Compile a pre-built `IIRModule` to a Linux x86-64 ELF executable.
///
/// Same backend pipeline as `compile_file_linux_x86_64` but accepts an
/// already-parsed `IIRModule` instead of a source file.  This is the
/// hook the `lang-aot` driver uses to share twig-aot's AOT pipeline
/// across language frontends (Nib, Brainfuck, …) — each frontend
/// produces an `IIRModule`, hands it here, and the rest of the chain
/// runs unchanged.
#[cfg(target_os = "linux")]
pub fn compile_module_to_linux_executable(
    module: &IIRModule,
    out: &Path,
) -> Result<(), AotError> {
    let obj_bytes = compile_module_linux_x86_64_object(module)?;
    let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    link_linux_x86_64_executable(&obj_bytes, stem, out)
}

/// Link a Linux x86-64 ELF object file (produced by any frontend) into
/// a runnable executable via the system C compiler (`cc`).  Embeds the
/// twig-aot runtime archive for `__twig_print_i64` and friends.
#[cfg(target_os = "linux")]
pub fn link_linux_x86_64_executable(
    obj_bytes: &[u8],
    stem: &str,
    out: &Path,
) -> Result<(), AotError> {
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt;

    if RUNTIME_LINUX_X86_64.len() <= 4 {
        return Err(AotError::Linker {
            status: None,
            stderr: "twig-aot: no Linux x86-64 runtime archive on this host \
                     (build twig-aot on a Linux x86-64 host)".into(),
        });
    }

    let mut obj_tmp = tempfile::Builder::new()
        .prefix(&format!("twig-aot-{stem}-"))
        .suffix(".o")
        .tempfile()?;
    obj_tmp.write_all(obj_bytes)?;

    let mut rt_tmp = tempfile::Builder::new()
        .prefix("twig_aot_runtime_")
        .suffix(".a")
        .tempfile()?;
    rt_tmp.write_all(RUNTIME_LINUX_X86_64)?;

    // GC archive (gc-core-capi), passed *after* the runtime archive so `cc`'s
    // left-to-right archive resolution pulls the collector member that satisfies
    // dynval_runtime.o's undefined `__twig_gc_alloc` (#118b-2b).
    let mut gc_tmp = tempfile::Builder::new()
        .prefix("twig_aot_gc_core_")
        .suffix(".a")
        .tempfile()?;
    gc_tmp.write_all(GC_CORE_ARCHIVE)?;

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let output = std::process::Command::new(&cc)
        .arg("-o").arg(out)
        .arg(obj_tmp.path())
        .arg(rt_tmp.path())
        .arg(gc_tmp.path())
        // `-lc -lm` for the C runtime; `-lpthread -ldl` because the gc-core-capi
        // staticlib bundles Rust std, which references these on Linux. They come
        // last so the archive members that need them are already selected.
        .arg("-lc").arg("-lm").arg("-lpthread").arg("-ldl")
        .output()
        .map_err(|e| AotError::Linker {
            status: None,
            stderr: format!("twig-aot: {cc} not found on PATH: {e}"),
        })?;
    drop(obj_tmp);
    drop(rt_tmp);
    drop(gc_tmp);

    if !output.status.success() {
        return Err(AotError::Linker {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let mut perms = std::fs::metadata(out)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(out, perms)?;
    Ok(())
}

/// Compile a Twig source file to a runnable Windows x86-64 PE executable.
///
/// Pipeline: source → IR → x86_64-backend (MS x64 ABI) → PE/COFF object
/// → linker probe (`link.exe` → `lld-link.exe` → `gcc.exe`) → `.exe`.
///
/// Requires a Windows build host with at least one supported linker on
/// `PATH`.  Returns an error with a clear "no Windows linker found"
/// message if none is available.
#[cfg(target_os = "windows")]
pub fn compile_file_windows_x86_64(src: &Path, out: &Path) -> Result<(), AotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("twig");
    let obj_bytes = compile_windows_x86_64_object(&source, stem)?;
    link_windows_x86_64_executable(&obj_bytes, stem, out)
}

/// Compile a pre-built `IIRModule` to a Windows x86-64 PE executable.
///
/// Frontend-agnostic counterpart to `compile_file_windows_x86_64` — see
/// `compile_module_to_linux_executable` for the design rationale.
#[cfg(target_os = "windows")]
pub fn compile_module_to_windows_executable(
    module: &IIRModule,
    out: &Path,
) -> Result<(), AotError> {
    let obj_bytes = compile_module_windows_x86_64_object(module)?;
    let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    link_windows_x86_64_executable(&obj_bytes, stem, out)
}

/// Link a Windows x86-64 PE/COFF object file into a runnable `.exe` via
/// the first system linker found on `PATH` (`link.exe` → `lld-link.exe`
/// → `gcc.exe`).  Embeds the twig-aot runtime archive.
#[cfg(target_os = "windows")]
pub fn link_windows_x86_64_executable(
    obj_bytes: &[u8],
    stem: &str,
    out: &Path,
) -> Result<(), AotError> {
    use std::io::Write as _;

    if RUNTIME_WINDOWS_X86_64.len() <= 4 {
        return Err(AotError::Linker {
            status: None,
            stderr: "twig-aot: no Windows x86-64 runtime archive on this host \
                     (build twig-aot on a Windows x86-64 host)".into(),
        });
    }

    let mut obj_tmp = tempfile::Builder::new()
        .prefix(&format!("twig-aot-{stem}-"))
        .suffix(".obj")
        .tempfile()?;
    obj_tmp.write_all(obj_bytes)?;

    let mut rt_tmp = tempfile::Builder::new()
        .prefix("twig_aot_runtime_")
        .suffix(".lib")
        .tempfile()?;
    rt_tmp.write_all(RUNTIME_WINDOWS_X86_64)?;

    // GC archive (gc-core-capi), embedded and passed after the runtime archive
    // so the linker resolves dynval_runtime.obj's `__twig_gc_alloc` (#118b-2b).
    let mut gc_tmp = tempfile::Builder::new()
        .prefix("twig_aot_gc_core_")
        .suffix(".lib")
        .tempfile()?;
    gc_tmp.write_all(GC_CORE_ARCHIVE)?;

    let linker = find_windows_linker().ok_or_else(|| AotError::Linker {
        status: None,
        stderr: "twig-aot: no Windows linker found on PATH \
                 (tried link.exe, lld-link.exe, gcc.exe)".into(),
    })?;

    let output = match linker.kind {
        WinLinkerKind::Link | WinLinkerKind::LldLink => {
            std::process::Command::new(&linker.path)
                .arg(format!("/OUT:{}", out.display()))
                .arg("/ENTRY:main")
                .arg("/SUBSYSTEM:CONSOLE")
                .arg(obj_tmp.path())
                .arg(rt_tmp.path())
                .arg(gc_tmp.path())
                .arg("libcmt.lib")
                .arg("legacy_stdio_definitions.lib")
                .output()
        }
        WinLinkerKind::Gcc => {
            std::process::Command::new(&linker.path)
                .arg("-o").arg(out)
                .arg(obj_tmp.path())
                .arg(rt_tmp.path())
                .arg(gc_tmp.path())
                .output()
        }
    }.map_err(|e| AotError::Linker {
        status: None,
        stderr: format!("twig-aot: linker spawn failed: {e}"),
    })?;
    drop(obj_tmp);
    drop(rt_tmp);
    drop(gc_tmp);

    if !output.status.success() {
        return Err(AotError::Linker {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

#[cfg(target_os = "windows")]
struct WinLinker {
    path: PathBuf,
    kind: WinLinkerKind,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
enum WinLinkerKind { Link, LldLink, Gcc }

/// Probe `PATH` for a supported Windows linker.  Returns the first one
/// found in priority order (link.exe → lld-link.exe → gcc.exe).
///
/// The probe checks the program's banner output rather than just
/// whether it can be spawned — git-bash environments ship a POSIX
/// `link(1)` utility on `PATH` that has the same name as MSVC's
/// `link.exe` but the wrong CLI grammar.
#[cfg(target_os = "windows")]
fn find_windows_linker() -> Option<WinLinker> {
    use std::process::Command;
    for (name, kind) in [
        ("link.exe", WinLinkerKind::Link),
        ("lld-link.exe", WinLinkerKind::LldLink),
        ("gcc.exe", WinLinkerKind::Gcc),
    ] {
        let probe = match kind {
            WinLinkerKind::Link | WinLinkerKind::LldLink => {
                // MSVC link.exe and lld-link.exe both print a banner
                // to stderr when invoked without args, then exit non-zero.
                Command::new(name).output()
            }
            WinLinkerKind::Gcc => Command::new(name).arg("--version").output(),
        };
        let Ok(o) = probe else { continue; };
        let stdout = String::from_utf8_lossy(&o.stdout);
        let stderr = String::from_utf8_lossy(&o.stderr);
        let banner = format!("{stdout}{stderr}");
        let is_real = match kind {
            // MSVC link.exe banner contains "Microsoft (R) Incremental Linker".
            // POSIX `link(1)` (coreutils) prints "link: missing operand" instead.
            WinLinkerKind::Link    => banner.contains("Microsoft") && banner.contains("Linker"),
            WinLinkerKind::LldLink => banner.contains("LLD") || banner.contains("lld-link"),
            WinLinkerKind::Gcc     => banner.contains("gcc") || banner.contains("GCC"),
        };
        if is_real {
            return Some(WinLinker { path: PathBuf::from(name), kind });
        }
    }
    None
}


// ---------------------------------------------------------------------------
// AOT preparation pipeline
// ---------------------------------------------------------------------------
//
// The Twig IR compiler emits `call_builtin "<op>" arg…` for every primitive
// operation and leaves all `type_hint` fields as `"any"`.  The ARM64 backend
// requires typed CIR instructions (`add_u64`, `cmp_lt_u64`, `mov_u64`, …).
//
// This preparation pipeline bridges the gap in three steps:
//
//  Step 1 — Pre-lower builtins.
//    Converts `call_builtin "+" a b` → `add a b`, `call_builtin "_move" n` →
//    `mov n`, etc.  This happens at the IIR level so that type propagation can
//    see the named ops and infer concrete types.
//
//  Step 2 — Normalise param types.
//    The Twig IR compiler declares every param as `"any"`.  We promote them to
//    `"i64"` — Twig integers are semantically signed 64-bit values, so using
//    the signed default ensures that comparisons (e.g. `cmp_lt`) emit signed
//    ARM64 condition codes (`B.LT`) rather than unsigned ones (`B.CC`), which
//    matters for programs that test against negative numbers.  `infer_types`
//    (from `aot-core`) seeds its environment from `func.params`, so this is
//    the right place to override the "any" sentinel.
//
//  Step 3 — Propagate and default types.
//    A lightweight fixed-point pass populates `type_hint` on every instruction
//    whose type can be determined from its operands and the seeded param types.
//    Any remaining `"any"` hints on arithmetic and `mov` instructions are
//    then defaulted to `"i64"` so that `aot_specialise` never sees an
//    untyped instruction and never emits `type_assert` guards (which the ARM64
//    backend lowers to `udf` hard-traps).

/// Builtin-name → IIR-op for AOT.
const AOT_BUILTIN_MAP: &[(&str, &str)] = &[
    ("+",  "add"),  ("-",  "sub"),   ("*",  "mul"),   ("/",  "div"),
    ("=",  "cmp_eq"), ("<", "cmp_lt"), (">", "cmp_gt"),
    ("<=", "cmp_le"), (">=", "cmp_ge"),
    ("not", "not"),  ("_move", "mov"),
];

/// Step 1: lower `call_builtin "<op>"` instructions to named IIR ops.
fn pre_lower_aot_builtins(func: &mut IIRFunction) {
    let old = std::mem::take(&mut func.instructions);
    func.instructions = old.into_iter().map(|instr| {
        if instr.op != "call_builtin" { return instr; }
        let name = match instr.srcs.first() {
            Some(Operand::Var(n)) => n.as_str(),
            _ => return instr,
        };
        let Some((_, op)) = AOT_BUILTIN_MAP.iter().find(|(b, _)| *b == name) else {
            return instr;
        };
        let args: Vec<Operand> = instr.srcs[1..].to_vec();
        IIRInstr::new(*op, instr.dest.clone(), args, &instr.type_hint)
    }).collect();
}

/// Step 2: promote `"any"` / `"polymorphic"` param types to `"i64"`.
///
/// Twig integers are semantically signed 64-bit values.  Using `i64` as the
/// default ensures that comparison instructions like `cmp_lt` emit signed ARM64
/// condition codes (`B.LT` / `B.GE`) rather than unsigned ones (`B.CC` / `B.CS`).
/// This matters for any program that compares against negative numbers — e.g.
/// `(if (< x 0) …)` — where unsigned semantics would treat `-5` as a huge
/// positive number and take the wrong branch.
fn normalize_params_to_i64(func: &mut IIRFunction) {
    for (_, ty) in &mut func.params {
        if ty == "any" || ty == "polymorphic" {
            *ty = "i64".to_string();
        }
    }
}

/// Step 3a: fixed-point type propagation seeded from params.
///
/// Applies inference rules:
/// - `const` with `Int` → `"i64"`, `Bool` → `"bool"`
/// - `cmp_*` → **operand type** (not `"bool"`)
/// - `add`, `sub`, `mul`, `div`, `mov`, `neg`, `not` → type of first operand
///
/// ## Why `cmp_*` uses operand type, not `"bool"`
///
/// The ARM64 backend's `emit_cmp` uses the CIR mnemonic suffix to choose
/// between signed (`i64`, `i32`, …) and unsigned (`u64`, `u32`, …) condition
/// codes.  `cmp_lt_bool` produces an unsigned comparison — correct when all
/// values are non-negative, but wrong for negative numbers.
///
/// Returning the operand type lets the specialiser emit `cmp_lt_u64` for the
/// untyped path (u64 params) and `cmp_lt_i64` for the typed path (i64 params),
/// selecting the right condition code in each case.  The boolean result value
/// (0 or 1 stored to the dest register) is the same either way.
///
/// Runs in a loop until stable.  Function params (already "i64" or a typed hint)
/// seed the environment together with already-typed instructions.
fn propagate_aot_types(func: &mut IIRFunction) {
    // Build initial env from params + any already-typed instructions.
    let mut env: HashMap<String, String> = HashMap::new();
    for (name, ty) in &func.params {
        if ty != "any" && ty != "polymorphic" {
            env.insert(name.clone(), ty.clone());
        }
    }
    for instr in &func.instructions {
        if let Some(dest) = &instr.dest {
            if instr.type_hint != "any" && instr.type_hint != "polymorphic" {
                env.insert(dest.clone(), instr.type_hint.clone());
            }
        }
    }

    // Fixed-point: keep iterating until no new types are inferred.
    loop {
        let mut changed = false;

        for instr in func.instructions.iter_mut() {
            if instr.type_hint != "any" { continue; }
            let Some(dest) = instr.dest.as_ref() else { continue; };

            let inferred = infer_aot_type(instr, &env);
            if let Some(ty) = inferred {
                instr.type_hint = ty.clone();
                if env.insert(dest.clone(), ty).as_deref() != Some(&instr.type_hint) {
                    changed = true;
                }
            }
        }

        if !changed { break; }
    }
}

/// Try to infer a concrete type for `instr` given the current SSA `env`.
///
/// Returns `None` when the instruction's type cannot yet be determined
/// (e.g. because a source variable is still `"any"`).
fn infer_aot_type(instr: &IIRInstr, env: &HashMap<String, String>) -> Option<String> {
    match instr.op.as_str() {
        "const" => match instr.srcs.first() {
            Some(Operand::Int(_))  => Some("i64".into()),
            Some(Operand::Bool(_)) => Some("bool".into()),
            _                       => None,
        },
        // Comparisons: use the operand type so the ARM64 backend can choose
        // signed vs unsigned condition codes.  `cmp_lt_u64` → unsigned CMP,
        // `cmp_lt_i64` → signed CMP.  Both store a bool 0/1 result.
        // Fall back to "bool" only when operand types are still unresolved —
        // that way the untyped path (which resolves to u64) still works.
        "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
            resolve_src_aot_type(instr.srcs.first(), env)
                .or_else(|| Some("bool".into()))
        }
        // Arithmetic + move: use the type of the first source operand.
        "add" | "sub" | "mul" | "div" | "mov" | "neg" | "not" => {
            resolve_src_aot_type(instr.srcs.first(), env)
        }
        _ => None,
    }
}

/// Resolve the type of a source `Operand` for AOT purposes.
fn resolve_src_aot_type(src: Option<&Operand>, env: &HashMap<String, String>) -> Option<String> {
    match src? {
        Operand::Var(name) => {
            let ty = env.get(name)?;
            if ty != "any" && ty != "polymorphic" {
                Some(ty.clone())
            } else {
                None
            }
        }
        Operand::Int(_)  => Some("i64".into()),
        Operand::Bool(_) => Some("bool".into()),
        _                => None,
    }
}

/// Step 0b: remove dead string-literal `const` instructions.
///
/// The twig-ir-compiler emits a pattern like:
/// ```text
/// const  %n1 = Var("x")            -- name register (string literal)
/// call_builtin "global_set" %n1 %v -- will be lowered by lower_global_io
/// ```
///
/// After `lower_global_io` runs, `call_builtin "global_set"` becomes
/// `global_store Str("x") Var("%v")` — the global name is now inline.
/// The `const %n1` instruction becomes dead code (its register is never
/// read again).  If left in place, `aot_specialise` converts it to
/// `const_str`, which the ARM64 backend cannot lower.
///
/// This pass removes `const` instructions whose source is `Operand::Var(_)`
/// (the string-literal-as-Var encoding) AND whose dest register never
/// appears in any other instruction's `srcs`.  Instructions that are
/// still referenced (e.g. as arg to an un-lowered `call_builtin`, or as
/// a `make_closure` name register) are retained.
fn strip_dead_string_consts(func: &mut IIRFunction) {
    use std::collections::HashSet;

    // Build a set of every Var-register name that is read in any src position.
    let used: HashSet<String> = func.instructions
        .iter()
        .flat_map(|instr| instr.srcs.iter())
        .filter_map(|op| {
            if let Operand::Var(n) = op { Some(n.clone()) } else { None }
        })
        .collect();

    // Remove const instructions of the form `%dest = Var("text")` whose
    // dest is not in `used`.  These are dead name-register loads.
    func.instructions.retain(|instr| {
        if instr.op == "const" {
            if let Some(Operand::Var(_)) = instr.srcs.first() {
                if let Some(dest) = &instr.dest {
                    return used.contains(dest);
                }
            }
        }
        true // keep everything else
    });
}

/// After `lower_string_literals_for_aot`, some `push_aot_string_literal` blocks
/// (alloc_bytes + store_byte sequences) become dead because their `buf_var` is
/// never used by anything other than the `store_byte` writes into that buffer.
/// This happens when every `str_eq`/`str_cmp` op whose operands were in the
/// `strings` map got folded to a `const` integer, leaving the allocation with
/// no live consumer.
///
/// Dead blocks inflate the CIR variable count; on aarch64 the frame-size limit
/// is 504 bytes (7-bit signed immediate × 8 for stp_pre/ldp_post), so a
/// function with several string-literal comparisons exceeds that limit and the
/// backend returns `BackendError::FrameTooLarge`.  This pass eliminates those
/// dead blocks before register allocation.
///
/// A block is dead when its `__aot_str{N}_buf` variable does not appear in the
/// srcs of any instruction other than `store_byte` (writes into the buffer are
/// not observable consumers; they only make sense if the buffer is later read).
fn strip_dead_aot_string_allocs(func: &mut IIRFunction) {
    use std::collections::HashSet;

    // ① Collect all __aot_str{N}_buf vars produced by alloc_bytes.
    let aot_bufs: HashSet<String> = func.instructions
        .iter()
        .filter(|instr| instr.op == "alloc_bytes")
        .filter_map(|instr| instr.dest.as_ref())
        .filter(|dest| dest.starts_with("__aot_str") && dest.ends_with("_buf"))
        .cloned()
        .collect();

    if aot_bufs.is_empty() {
        return;
    }

    // ② Collect alias-mov instructions: `mov alias = buf_var` where buf_var ∈ aot_bufs.
    //    These are emitted by `lower_string_literals_for_aot` to make the original
    //    `str_const` dest variable available for use in `call` / `ret` / similar.
    //
    //    E4-dyn (E4d-4): a string variable promoted to a runtime handle is the
    //    dest of `str_const` — and therefore of an alias `mov alias = buf` — in
    //    MORE THAN ONE basic block, so a single alias name maps to SEVERAL buffers
    //    (one per branch). We must keep every `(alias, buf)` pair: a map keyed by
    //    alias would retain only the last buffer and wrongly strip the others as
    //    dead, so at run time a branch that selects an earlier buffer would read
    //    freed/empty memory (the E4-dyn foothold only dodged this by testing the
    //    branch whose buffer happened to be defined last).
    let alias_pairs: Vec<(String, String)> = func.instructions
        .iter()
        .filter(|instr| instr.op == "mov")
        .filter_map(|instr| {
            let alias = instr.dest.as_ref()?;
            let src = match instr.srcs.first()? {
                Operand::Var(v) => v,
                _ => return None,
            };
            if aot_bufs.contains(src) {
                Some((alias.clone(), src.clone()))
            } else {
                None
            }
        })
        .collect();
    // The set of alias names (str vars fed by a buffer) — used to exclude the
    // alias-defining `mov` from the buffer-liveness scan.
    let alias_names: HashSet<String> =
        alias_pairs.iter().map(|(alias, _)| alias.clone()).collect();

    // ③ Find alias dests that are "live" — referenced in srcs of any instruction
    //    other than write-only buffer ops and the alias-mov itself.
    //    A live alias means the buf(s) it points to are needed at runtime.
    let live_alias_dests: HashSet<String> = func.instructions
        .iter()
        .filter(|instr| {
            if instr.op == "store_byte" || instr.op == "field_store" {
                return false;
            }
            // Exclude the alias-defining mov itself from the liveness scan.
            if instr.op == "mov" {
                if let Some(dest) = &instr.dest {
                    if alias_names.contains(dest) {
                        return false;
                    }
                }
            }
            true
        })
        .flat_map(|instr| instr.srcs.iter())
        .filter_map(|op| if let Operand::Var(n) = op { Some(n.clone()) } else { None })
        .filter(|n| alias_names.contains(n))
        .collect();

    // ④ A buf is live if directly referenced (non-write-only, non-alias-mov) OR if
    //    its alias dest is live (i.e. the pointer is passed to a `call` etc.).
    let live_bufs: HashSet<String> = {
        let direct: HashSet<String> = func.instructions
            .iter()
            .filter(|instr| {
                if instr.op == "store_byte" || instr.op == "field_store" {
                    return false;
                }
                // The alias-mov doesn't constitute a "read" of the buffer data.
                if instr.op == "mov" {
                    if let Some(dest) = &instr.dest {
                        if alias_names.contains(dest) {
                            return false;
                        }
                    }
                }
                true
            })
            .flat_map(|instr| instr.srcs.iter())
            .filter_map(|op| if let Operand::Var(n) = op { Some(n.clone()) } else { None })
            .filter(|n| aot_bufs.contains(n))
            .collect();

        // EVERY buffer bound to a live alias stays live — not just the last one
        // that happened to be `mov`d into that alias name (the E4d-4 fix).
        let via_alias: HashSet<String> = alias_pairs
            .iter()
            .filter(|(alias, _)| live_alias_dests.contains(alias.as_str()))
            .map(|(_, buf)| buf.clone())
            .collect();

        direct.union(&via_alias).cloned().collect()
    };

    let dead_bufs: HashSet<String> = aot_bufs.difference(&live_bufs).cloned().collect();
    if dead_bufs.is_empty() {
        return;
    }

    // Derive the string prefix (__aot_str{N}) from each dead buf name so we can
    // match all associated vars (_len, _buf, _off{i}, _byte{i}).
    let dead_prefixes: HashSet<String> = dead_bufs
        .iter()
        .map(|buf| buf.strip_suffix("_buf").unwrap_or(buf).to_string())
        .collect();

    // Remove instructions that belong to dead string blocks:
    //  - Any instruction whose dest starts with `{prefix}_` (const for len/tlen/off/byte,
    //    alloc_bytes for buf).
    //  - Any `store_byte` or `field_store` instruction whose first src is a dead buf.
    //  - Any alias-mov `mov alias = dead_buf` (safe because alias dest is not live).
    func.instructions.retain(|instr| {
        if let Some(dest) = &instr.dest {
            for prefix in &dead_prefixes {
                if dest.starts_with(&format!("{prefix}_")) {
                    return false;
                }
            }
        }
        if instr.op == "store_byte" || instr.op == "field_store" {
            if let Some(Operand::Var(ptr)) = instr.srcs.first() {
                if dead_bufs.contains(ptr) {
                    return false;
                }
            }
        }
        // Strip alias-movs whose source buffer is dead: `mov alias = dead_buf`.
        // Safe because a live alias keeps ALL of its buffers live (see ④), so a
        // dead source buffer here can only belong to a dead alias.
        if instr.op == "mov" {
            if let Some(Operand::Var(src)) = instr.srcs.first() {
                if dead_bufs.contains(src) {
                    return false;
                }
            }
        }
        true
    });
}

/// Lower the landed E4 literal-output string ops to native byte-buffer I/O.
///
/// Buffer layout (LANG-STR-RT): every heap string begins with an 8-byte
/// little-endian length followed by the raw UTF-8 bytes:
///
///   offset 0  : int64_t length    (written by `field_store buf, 0, len`)
///   offset 8  : char bytes[0..n)  (written by N × `store_byte buf, 8+i, byte`)
///
/// This header lets the runtime helpers (`__twig_str_eq`, etc.) and the
/// fallback `str_len` / `print_str` code read the length without any
/// out-of-band bookkeeping — a pointer to the buffer is sufficient.
/// The static-fold path (literal strings tracked in the `strings` map)
/// still avoids runtime loads for `str_len` by folding to a `const`.
fn push_aot_string_literal(
    lowered: &mut Vec<IIRInstr>,
    next: &mut usize,
    literal: String,
) -> (String, String, String) {
    let base = format!("__aot_str{next}");
    *next += 1;
    let len_var  = format!("{base}_len");
    let tlen_var = format!("{base}_tlen");
    let buf_var  = format!("{base}_buf");

    let n = literal.len() as i64;
    // length constant (used by str_len static fold and field_store header)
    lowered.push(IIRInstr::new(
        "const",
        Some(len_var.clone()),
        vec![Operand::Int(n)],
        "i64",
    ));
    // total allocation = header (8 bytes) + string bytes.
    // saturating_add prevents n+8 from wrapping negative on a theoretical
    // 9-exabyte literal, avoiding a heap under-allocation.
    lowered.push(IIRInstr::new(
        "const",
        Some(tlen_var.clone()),
        vec![Operand::Int(n.saturating_add(8))],
        "i64",
    ));
    lowered.push(IIRInstr::new(
        "alloc_bytes",
        Some(buf_var.clone()),
        vec![Operand::Var(tlen_var)],
        "i64",
    ));
    // write the 8-byte length header at field index 0 (byte offset 0)
    lowered.push(IIRInstr::new(
        "field_store",
        None,
        vec![
            Operand::Var(buf_var.clone()),
            Operand::Int(0),
            Operand::Var(len_var.clone()),
        ],
        "void",
    ));
    // write string bytes at offset 8 + idx
    for (idx, byte) in literal.bytes().enumerate() {
        let off_var  = format!("{base}_off{idx}");
        let byte_var = format!("{base}_byte{idx}");
        lowered.push(IIRInstr::new(
            "const",
            Some(off_var.clone()),
            vec![Operand::Int(8 + idx as i64)],
            "i64",
        ));
        lowered.push(IIRInstr::new(
            "const",
            Some(byte_var.clone()),
            vec![Operand::Int(byte as i64)],
            "i64",
        ));
        lowered.push(IIRInstr::new(
            "store_byte",
            None,
            vec![
                Operand::Var(buf_var.clone()),
                Operand::Var(off_var),
                Operand::Var(byte_var),
            ],
            "void",
        ));
    }

    (buf_var, len_var, literal)
}

/// E4-dyn (E4d-4): compute the set of string variables that are chosen by
/// control flow — the destination of a `str`-typed instruction in **more than
/// one basic block**.  For these, the compile-time last-writer-wins literal
/// tracking (`strings`) cannot resolve a single value, so we must NOT register
/// them there; `print_str`/`str_len` then fall into their existing **runtime**
/// paths (`field_load` the length header from the buffer at run time) instead of
/// the static-length fast path — which would use the wrong branch's length
/// whenever the two literals differ in length.
///
/// This mirrors `iir-to-llvm`'s `collect_slot_vars` and `iir-to-wasm`'s
/// `collect_runtime_str_vars` exactly (same basic-block rule): a `label` starts
/// a new block, and a terminator (`jmp`/`jmp_if_false`/`jmp_if_true`/`ret`/
/// `ret_void`) ends one.  A str reassigned twice *straight-line* stays in one
/// block and keeps the literal fast path — the linear tracking is right there.
///
/// The native buffer built by `push_aot_string_literal` already has the
/// `[i64 len][bytes]` layout and the var's stack slot already holds the
/// buffer's address (a runtime handle) via `mov dest = buf`, so no backend
/// change is needed — dropping the `strings` registration is the whole fix, and
/// it applies to both the aarch64 and x86_64 backends at once.
fn collect_runtime_str_vars_for_aot(func: &IIRFunction) -> HashSet<String> {
    let mut str_blocks: HashMap<&str, HashSet<usize>> = HashMap::new();
    let mut block: usize = 0;
    for instr in &func.instructions {
        let op = instr.op.as_str();
        if op == "label" {
            block += 1;
        }
        if let Some(dest) = &instr.dest {
            if instr.type_hint == "str" {
                str_blocks.entry(dest.as_str()).or_default().insert(block);
            }
        }
        if matches!(op, "jmp" | "jmp_if_false" | "jmp_if_true" | "ret" | "ret_void") {
            block += 1;
        }
    }
    str_blocks
        .into_iter()
        .filter(|(_, blocks)| blocks.len() >= 2)
        .map(|(name, _)| name.to_string())
        .collect()
}

fn lower_string_literals_for_aot(func: &mut IIRFunction) {
    // E4-dyn (E4d-4): variables whose string value is chosen by control flow.
    // These are kept OUT of the compile-time `strings` map so their length is
    // read at run time from the buffer header, not folded to a (possibly wrong)
    // branch's constant.
    let runtime_str_vars = collect_runtime_str_vars_for_aot(func);
    let mut lowered = Vec::with_capacity(func.instructions.len());
    let mut strings: HashMap<String, (String, String, String)> = HashMap::new();
    let mut ints: HashMap<String, i64> = HashMap::new();
    let mut next = 0usize;

    for instr in std::mem::take(&mut func.instructions) {
        if instr.op == "const" {
            if let (Some(dest), Some(Operand::Int(value))) =
                (instr.dest.as_ref(), instr.srcs.first())
            {
                ints.insert(dest.clone(), *value);
            }
            lowered.push(instr);
            continue;
        }

        if instr.op == "mov" {
            if let (Some(dest), Some(Operand::Var(src))) =
                (instr.dest.as_ref(), instr.srcs.first())
            {
                if let Some(value) = ints.get(src).copied() {
                    ints.insert(dest.clone(), value);
                }
                if let Some(string) = strings.get(src).cloned() {
                    strings.insert(dest.clone(), string);
                }
            }
            lowered.push(instr);
            continue;
        }

        if matches!(instr.op.as_str(), "add" | "sub" | "mul" | "div") {
            if let Some(dest) = instr.dest.as_ref() {
                let left = int_metadata_value(instr.srcs.first(), &ints);
                let right = int_metadata_value(instr.srcs.get(1), &ints);
                let value = match (instr.op.as_str(), left, right) {
                    ("add", Some(left), Some(right)) => left.checked_add(right),
                    ("sub", Some(left), Some(right)) => left.checked_sub(right),
                    ("mul", Some(left), Some(right)) => left.checked_mul(right),
                    ("div", Some(left), Some(right)) if right != 0 => left.checked_div(right),
                    _ => None,
                };
                if let Some(value) = value {
                    ints.insert(dest.clone(), value);
                }
            }
            lowered.push(instr);
            continue;
        }

        if instr.op == "str_const" {
            let dest = match instr.dest.clone() {
                Some(dest) => dest,
                None => {
                    lowered.push(instr);
                    continue;
                }
            };
            let literal = match instr.srcs.first() {
                Some(Operand::Str(literal)) => literal.clone(),
                _ => {
                    lowered.push(instr);
                    continue;
                }
            };
            if !is_printable_ascii_str(&literal) {
                lowered.push(instr);
                continue;
            }

            let (buf_var, len_var, lit) = push_aot_string_literal(&mut lowered, &mut next, literal);
            // Emit `mov dest = buf_var` so that `dest` is defined for any instruction
            // that uses it directly (e.g. `call strlen(dest)`).  The alias is stripped by
            // `strip_dead_aot_string_allocs` when `dest` is not observed outside the
            // already-folded string ops, so this does not inflate the frame for fold-only
            // strings.  `dest`'s slot now holds the buffer's address — a runtime handle to
            // a `[i64 len][bytes]` block, identical to a runtime string parameter.
            lowered.push(IIRInstr::new(
                "mov",
                Some(dest.clone()),
                vec![Operand::Var(buf_var)],
                "i64",
            ));
            // E4-dyn (E4d-4): only register a *single-block* (foldable) string in
            // the compile-time literal map.  A branch-selected string is left out,
            // so `print_str`/`str_len`/`str_eq`/`str_cmp` on it take their runtime
            // paths (reading the length header at run time) rather than baking in
            // one branch's last-written literal — which would print the wrong
            // length whenever the branches' strings differ in length.
            if !runtime_str_vars.contains(&dest) {
                strings.insert(dest.clone(), (dest, len_var, lit));
            }
            continue;
        }

        if instr.op == "str_concat" {
            let Some(dest) = instr.dest.clone() else {
                lowered.push(instr);
                continue;
            };
            let [Operand::Var(left), Operand::Var(right)] = instr.srcs.as_slice() else {
                lowered.push(instr);
                continue;
            };
            let left_lit = strings.get(left).map(|(_, _, l)| l.clone());
            let right_lit = strings.get(right).map(|(_, _, r)| r.clone());
            // Compile-time fold: when BOTH operands are statically known literals and
            // their concatenation is printable ASCII, bake the joined literal directly
            // into the data segment — no runtime call, and the result is itself a known
            // literal (so downstream `str_len`/`str_index` keep folding).
            if let (Some(ll), Some(rl)) = (&left_lit, &right_lit) {
                let literal = format!("{ll}{rl}");
                if is_printable_ascii_str(&literal) {
                    let (buf_var, len_var, lit) =
                        push_aot_string_literal(&mut lowered, &mut next, literal);
                    lowered.push(IIRInstr::new(
                        "mov",
                        Some(dest.clone()),
                        vec![Operand::Var(buf_var)],
                        "i64",
                    ));
                    strings.insert(dest.clone(), (dest, len_var, lit));
                    continue;
                }
            }
            // E4-dyn runtime path: at least one operand is a runtime string handle (an
            // `INPUT` result, a call result, a branch-selected string) with no compile-
            // time literal — or the joined literal isn't printable ASCII. Delegate to
            // `__twig_str_concat(a, b)`, which reads both `[i64 len][bytes]` headers and
            // returns a fresh block handle. `dest` is deliberately NOT recorded in
            // `strings`, so downstream `print_str`/`str_len` take their runtime
            // header-reading paths rather than folding a length that isn't known.
            lowered.push(IIRInstr::new(
                "call_builtin",
                Some(dest),
                vec![
                    Operand::Var("str_concat".into()),
                    Operand::Var(left.clone()),
                    Operand::Var(right.clone()),
                ],
                &instr.type_hint,
            ));
            continue;
        }

        if instr.op == "str_slice" {
            let Some(dest) = instr.dest.clone() else {
                lowered.push(instr);
                continue;
            };
            let [Operand::Var(src), Operand::Var(start), Operand::Var(end)] = instr.srcs.as_slice()
            else {
                lowered.push(instr);
                continue;
            };
            let Some((_, _, literal)) = strings.get(src).cloned() else {
                lowered.push(instr);
                continue;
            };
            let (Some(start), Some(end)) = (ints.get(start).copied(), ints.get(end).copied())
            else {
                lowered.push(instr);
                continue;
            };
            if start < 0 || end < start || end as usize > literal.len() {
                lowered.push(IIRInstr::new("type_assert", None, vec![], "void"));
                let (bv, lv, lt) = push_aot_string_literal(&mut lowered, &mut next, String::new());
                lowered.push(IIRInstr::new("mov", Some(dest.clone()), vec![Operand::Var(bv)], "i64"));
                strings.insert(dest.clone(), (dest, lv, lt));
                continue;
            }
            let slice = literal.as_bytes()[start as usize..end as usize].to_vec();
            let Ok(literal) = String::from_utf8(slice) else {
                lowered.push(instr);
                continue;
            };
            if !is_printable_ascii_str(&literal) {
                lowered.push(instr);
                continue;
            }
            let (buf_var, len_var, lit) = push_aot_string_literal(&mut lowered, &mut next, literal);
            lowered.push(IIRInstr::new("mov", Some(dest.clone()), vec![Operand::Var(buf_var)], "i64"));
            strings.insert(dest.clone(), (dest, len_var, lit));
            continue;
        }

        if instr.op == "str_len" {
            let Some(dest) = instr.dest.clone() else {
                lowered.push(instr);
                continue;
            };
            let Some(Operand::Var(src)) = instr.srcs.first() else {
                lowered.push(instr);
                continue;
            };
            if let Some((_, _, literal)) = strings.get(src).cloned() {
                // Compile-time fold: length is statically known.
                ints.insert(dest.clone(), literal.len() as i64);
                lowered.push(IIRInstr::new(
                    "const",
                    Some(dest),
                    vec![Operand::Int(literal.len() as i64)],
                    &instr.type_hint,
                ));
            } else {
                // Runtime fallback: read the 8-byte length from offset 0 of the
                // LANG-STR-RT buffer (field index 0 → byte offset 0).
                lowered.push(IIRInstr::new(
                    "field_load",
                    Some(dest),
                    vec![Operand::Var(src.clone()), Operand::Int(0)],
                    &instr.type_hint,
                ));
            }
            continue;
        }

        if instr.op == "str_index" {
            let Some(dest) = instr.dest.clone() else {
                lowered.push(instr);
                continue;
            };
            let [Operand::Var(src), Operand::Var(idx)] = instr.srcs.as_slice() else {
                lowered.push(instr);
                continue;
            };
            let Some((_, _, literal)) = strings.get(src).cloned() else {
                lowered.push(instr);
                continue;
            };
            let Some(idx) = ints.get(idx).copied() else {
                lowered.push(instr);
                continue;
            };
            let Some(byte) = usize::try_from(idx)
                .ok()
                .and_then(|idx| literal.as_bytes().get(idx))
                .copied()
            else {
                lowered.push(IIRInstr::new("type_assert", None, vec![], "void"));
                lowered.push(IIRInstr::new(
                    "const",
                    Some(dest),
                    vec![Operand::Int(0)],
                    &instr.type_hint,
                ));
                continue;
            };
            lowered.push(IIRInstr::new(
                "const",
                Some(dest),
                vec![Operand::Int(byte as i64)],
                &instr.type_hint,
            ));
            continue;
        }

        if instr.op == "str_eq" {
            let Some(dest) = instr.dest.clone() else {
                lowered.push(instr);
                continue;
            };
            let [Operand::Var(left), Operand::Var(right)] = instr.srcs.as_slice() else {
                lowered.push(instr);
                continue;
            };
            let left_lit = strings.get(left).map(|(_, _, l)| l.clone());
            let right_lit = strings.get(right).map(|(_, _, r)| r.clone());
            if let (Some(ll), Some(rl)) = (left_lit, right_lit) {
                // Both statically known: fold to a compile-time constant.
                lowered.push(IIRInstr::new(
                    "const",
                    Some(dest),
                    vec![Operand::Int((ll == rl) as i64)],
                    &instr.type_hint,
                ));
            } else {
                // At least one operand is a runtime string (function parameter
                // or return value).  Delegate to __twig_str_eq which reads the
                // LANG-STR-RT length header and memcmp's the data.
                lowered.push(IIRInstr::new(
                    "call_builtin",
                    Some(dest),
                    vec![
                        Operand::Var("str_eq".into()),
                        Operand::Var(left.clone()),
                        Operand::Var(right.clone()),
                    ],
                    &instr.type_hint,
                ));
            }
            continue;
        }

        if instr.op == "str_cmp" {
            let Some(dest) = instr.dest.clone() else {
                lowered.push(instr);
                continue;
            };
            let [Operand::Var(left), Operand::Var(right)] = instr.srcs.as_slice() else {
                lowered.push(instr);
                continue;
            };
            let Some((_, _, left_literal)) = strings.get(left).cloned() else {
                lowered.push(instr);
                continue;
            };
            let Some((_, _, right_literal)) = strings.get(right).cloned() else {
                lowered.push(instr);
                continue;
            };
            let value = match left_literal.as_bytes().cmp(right_literal.as_bytes()) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            };
            ints.insert(dest.clone(), value);
            lowered.push(IIRInstr::new(
                "const",
                Some(dest),
                vec![Operand::Int(value)],
                &instr.type_hint,
            ));
            continue;
        }

        if instr.op == "print_str" {
            let Some(Operand::Var(src)) = instr.srcs.first() else {
                lowered.push(instr);
                continue;
            };
            // Data starts at buf + 8 (LANG-STR-RT: first 8 bytes = i64 length).
            // Whether the string came from a literal (tracked in `strings`) or
            // a runtime parameter, both use the same buffer layout, so the
            // same add-8 + call_builtin sequence works for both paths.
            let ps_idx = next;
            next += 1;
            let off_var  = format!("__aot_ps{ps_idx}_off");
            let data_var = format!("__aot_ps{ps_idx}_dat");
            lowered.push(IIRInstr::new(
                "const",
                Some(off_var.clone()),
                vec![Operand::Int(8)],
                "i64",
            ));
            lowered.push(IIRInstr::new(
                "add",
                Some(data_var.clone()),
                vec![Operand::Var(src.clone()), Operand::Var(off_var)],
                "i64",
            ));
            if let Some((_, len_var, _)) = strings.get(src).cloned() {
                // Length is statically known; avoid a runtime load.
                lowered.push(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![
                        Operand::Var("print_string".into()),
                        Operand::Var(data_var),
                        Operand::Var(len_var),
                    ],
                    "void",
                ));
            } else {
                // Runtime string: read the length header (field index 0).
                let len_var = format!("__aot_ps{ps_idx}_len");
                lowered.push(IIRInstr::new(
                    "field_load",
                    Some(len_var.clone()),
                    vec![Operand::Var(src.clone()), Operand::Int(0)],
                    "i64",
                ));
                lowered.push(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![
                        Operand::Var("print_string".into()),
                        Operand::Var(data_var),
                        Operand::Var(len_var),
                    ],
                    "void",
                ));
            }
            continue;
        }

        lowered.push(instr);
    }

    func.instructions = lowered;
}

fn is_printable_ascii_str(s: &str) -> bool {
    s.bytes()
        .all(|b| matches!(b, b'\n' | b'\r' | b'\t' | 0x20..=0x7e))
}

fn int_metadata_value(src: Option<&Operand>, ints: &HashMap<String, i64>) -> Option<i64> {
    match src? {
        Operand::Int(value) => Some(*value),
        Operand::Var(name) => ints.get(name).copied(),
        _ => None,
    }
}

/// Step 3b: default any remaining `"any"` hints on arithmetic / move
/// instructions to `"i64"`.
///
/// This handles instructions whose sources are still `"any"` after the
/// propagation pass — most commonly the results of `call` instructions whose
/// return type is not tracked.  Defaulting to `"i64"` is correct for Twig:
/// all Twig integer literals and arithmetic results are semantically signed
/// 64-bit values.  Using `i64` here ensures that comparisons downstream
/// (e.g. `cmp_lt`) emit signed ARM64 condition codes and produce correct
/// results for negative numbers, regardless of whether the source was
/// annotated or not.
fn default_any_to_i64(func: &mut IIRFunction) {
    for instr in &mut func.instructions {
        if instr.type_hint != "any" { continue; }
        match instr.op.as_str() {
            "add" | "sub" | "mul" | "div" | "mod" | "mov" | "neg" | "not" => {
                instr.type_hint = "i64".to_string();
            }
            _ => {}
        }
    }
}

/// Apply all preparation steps to every function in `module`.
///
/// Steps:
///  0. `lower_global_io` — converts `call_builtin "global_set"` /
///     `"global_get"` to `global_store` / `global_load` (LANG39).
///     Must run first so the const-string look-back can see all instructions.
///  0a-string. `lower_string_literals_for_aot` — converts E4 literal-output
///     `str_const` + `print_str` into existing native heap-byte I/O
///     (`alloc_bytes`, `store_byte`, and `call_builtin "print_string"`) and
///     folds direct literal `str_len` to an integer constant.
///  0b. `strip_dead_string_consts` — removes `const %n = Var("name")`
///     instructions that are now dead after step 0.  Without this pass,
///     `aot_specialise` converts them to `const_str` which the ARM64 backend
///     cannot lower (there is no stack-slot for a string pointer).
///  0c. `strip_dead_aot_string_allocs` — removes `alloc_bytes` + `store_byte`
///     blocks for `__aot_str{N}` buffers whose `buf_var` is never read by any
///     non-`store_byte` instruction.  This shrinks the CIR variable count for
///     functions with many folded string comparisons, keeping the aarch64 stack
///     frame within the 504-byte stp/ldp limit.
///  1. `pre_lower_aot_builtins` — lowers `call_builtin "+"` → `add`, etc.
///  2. `normalize_params_to_i64` — promotes untyped params to `i64`.
///  3. `propagate_aot_types` — fixed-point type propagation.
///  4. `default_any_to_i64` — defaults unresolved arithmetic types to `i64`.
fn prepare_module_for_aot(module: &mut IIRModule) {
    // Phase 0: lower global_set / global_get → global_store / global_load.
    lower_global_io(module);

    // Phase 0a: lower lispy heap builtins to **runtime calls** (LANG77,
    // McCarthy L3b-2b).  A lisp frontend (McCarthy Lisp, Twig) emits
    // `call_builtin "cons"/"car"/"cdr"`; this rewrite renames them to
    // `dyn_cons`/`dyn_car`/`dyn_cdr`, which the backends dispatch to
    // `__dyn_*` in the linked C lisp runtime
    // (`twig-aot/runtime/dynval_runtime.c`).  Unlike the structural
    // `lower_heap_builtins` (alloc + field_*, used by the managed wasm/jvm/
    // clr/beam backends), this keeps the value NaN-box **tagged** — which is
    // what later enables `pair?`/`ATOM`/`EQ`/symbols (L3b-2c).  It only
    // touches those exact builtin names, so a module without them — every
    // Twig/Nib/Brainfuck program today — is left unchanged.
    // Phase 0-clo: closures (E6d-7a). NativeAot has no native closure model, so
    // lower `alloc_closure`/`call_closure` to the cons-heap form + a synthesized
    // `__dyn_call_closure` dispatcher — all `cons`/`car`/`cdr`/`call` the runtime
    // heap path below already lowers. Must run BEFORE `lower_heap_builtins_runtime`
    // so the `cons`/`car`/`cdr` it emits become `dyn_*` runtime calls. A no-op for
    // a closure-free module.
    lower_closures_to_heap(module);

    lower_heap_builtins_runtime(module);

    // Phase 0a‴: dynamic integer arithmetic over `any` (LANG-FULL E6d-2).
    // A dynamic frontend emits `call_builtin "+"/"-"/…` whose operands are boxed
    // `DynValue`s; expand each to `unbox → typed op → box` (the same generic ops
    // `cons`/`car` use). The typed backends have no "add two tagged words" opcode.
    // Runs after `lower_heap_builtins_runtime` (so a `car` result is an
    // identifiable boxed `ref<any>`) and before `lower_dyn_repr`.
    lower_dynamic_arith(module);

    // Phase 0a″: compile-time symbol interning (LANG77 / L3b-2c-3).
    // Rewrite each `const Var(name):symbol` to the finished tagged immediate
    // `(id << 32) | TAG_SYMBOL`, with module-wide ids (so the same name → the
    // same id → `EQ` is word equality). Runs before `lower_dyn_repr` so the
    // representation pass sees finished symbol immediates. A no-op for modules
    // without symbol literals.
    intern_symbols(module);

    // Phase 0a′: type-directed lisp-value representation (LANG77 / L3b-2c).
    // After cons/car/cdr are `dyn_*` calls, box the integer atoms that flow
    // into them (so their NaN-box tag is `000`, not the heap tag a raw int's
    // low bits would collide with) and unbox the program result at the exit
    // boundary.  Gate-free and type-directed: a module with no `dyn_*` calls
    // (every Twig/Nib/Brainfuck program) has nothing to box and is unchanged.
    lower_dyn_repr(module);

    // Phase 0a⁗: tagged-i64 representation of the generic `box`/`unbox` ops that
    // `lower_dynamic_arith` emitted (E6d-2b). The structural backends lower those
    // ops directly, but the native/LLVM tagged-word world has no such opcode — a
    // tagged word is `n << 3`, produced/consumed by `__dyn_box_int` /
    // `__dyn_unbox_int`. Rewrite the residual ops to those runtime calls, which
    // `V1_BUILTINS` dispatches to `bl/call __dyn_box_int` / `__dyn_unbox_int`.
    lower_box_unbox_to_runtime_calls(module);

    for func in &mut module.functions {
        lower_string_literals_for_aot(func);
        // Phase 0b: remove dead name-register `const` instructions.
        // `lower_global_io` leaves `const %n = Var("x")` in place even after
        // the `global_set`/`global_get` that consumed it is rewritten.
        // These become `const_str` in CIR which the ARM64 backend rejects.
        strip_dead_string_consts(func);
        strip_dead_aot_string_allocs(func);
        pre_lower_aot_builtins(func);
        normalize_params_to_i64(func);
        propagate_aot_types(func);
        default_any_to_i64(func);
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Run the per-function AOT pipeline and link into a single text section.
///
/// Clones `module`, runs the full AOT preparation pipeline (builtin
/// pre-lowering + u64 type normalisation + type propagation + default-any),
/// and then delegates to [`compile_module_to_text_raw`] for the actual
/// two-pass compile + link.
///
/// This is the "untyped u64" path: all params and arithmetic are treated as
/// `u64`.  For signed `i64` semantics use [`compile_typed_module_to_arm64_bytes`].
#[allow(clippy::type_complexity)] // cohesive compiled-module tuple; see compile_module_x86_64_to_text
fn compile_module_to_text(
    module: &IIRModule,
) -> Result<(Vec<u8>, HashMap<String, usize>, usize, Vec<GlobalByteReloc>, Vec<ExternBranchReloc>), AotError> {
    // We work on a clone so the caller's `IIRModule` is never mutated.
    // `prepare_module_for_aot` runs four passes (LANG39 adds step 0):
    //   0. `lower_global_io` — `call_builtin "global_set/get"` → `global_store/load`
    //   1. `call_builtin "+"` → `add`, etc.   (see `pre_lower_aot_builtins`)
    //   2. param types "any" → "i64"           (see `normalize_params_to_i64`)
    //   3. propagate + default remaining "any" (see `propagate_aot_types` /
    //                                            `default_any_to_i64`)
    let mut module = module.clone();
    prepare_module_for_aot(&mut module);
    compile_module_to_text_raw(&module)
}

// ---------------------------------------------------------------------------
// Global-slot scanning (LANG39)
// ---------------------------------------------------------------------------

/// Scan all functions in a (post-`lower_global_io`) IIR module for
/// `global_load` / `global_store` instructions and assign each unique global
/// name a consecutive slot index (0, 1, 2, …).
///
/// Returns a map from global name → slot index.  If no globals are used the
/// map is empty.
fn collect_global_slots(module: &IIRModule) -> HashMap<String, usize> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for fn_ in &module.functions {
        for instr in &fn_.instructions {
            if instr.op == "global_load" || instr.op == "global_store" {
                if let Some(Operand::Str(name)) = instr.srcs.first() {
                    let next = map.len();
                    map.entry(name.clone()).or_insert(next);
                }
            }
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Inner two-pass compile + link
// ---------------------------------------------------------------------------

/// Inner two-pass compile + link, operating on an already-prepared `IIRModule`.
///
/// Unlike [`compile_module_to_text`] this function **does not** clone or
/// prepare the module — it trusts that the caller has already run the
/// appropriate preparation (either [`prepare_module_for_aot`] for the untyped
/// u64 path, or just [`pre_lower_aot_builtins`] for the typed i64 path).
///
/// Two-pass strategy for cross-function calls:
///
/// Pass 1 — compile each function independently.  Cross-function `call`
///   instructions emit `BL #0` placeholder instructions and record
///   [`Reloc`] entries (callee name + word index in the per-function binary).
///   `global_load`/`global_store` instructions emit `ADRP + ADD` placeholder
///   pairs and record [`GlobalWordReloc`] entries (word indices for Mach-O
///   ARM64 relocations).
///
/// Link — concatenate all function binaries into one flat code section and
///   record each function's byte offset.
///
/// Pass 2 — patch every placeholder `BL` with the correct PC-relative
///   offset using the now-known function offsets.  `GlobalWordReloc` entries
///   are converted to [`GlobalByteReloc`] byte offsets (for `pack_object_with_globals`).
///
/// ARM64 `BL` encoding: opcode `0x94000000`, 26-bit signed PC-relative
/// offset in units of 4 bytes (instruction words).
///
/// Returns `(linked_text, fn_offsets, n_global_slots, global_byte_relocs, extern_branch_relocs)`.
///
/// `extern_branch_relocs` — BL instructions in the linked text that target
/// symbols not defined in this module (e.g. `__twig_print_i64`).  The
/// packager ([`pack_object_with_globals_and_externals`]) converts them into
/// `N_UNDF | N_EXT` symbol-table entries and `ARM64_RELOC_BRANCH26` records
/// so the system linker can patch them from the Twig AOT runtime archive.
#[allow(clippy::type_complexity)] // cohesive compiled-module tuple; see compile_module_x86_64_to_text
fn compile_module_to_text_raw(
    module: &IIRModule,
) -> Result<(Vec<u8>, HashMap<String, usize>, usize, Vec<GlobalByteReloc>, Vec<ExternBranchReloc>), AotError> {
    // ── Collect global names (LANG39) ─────────────────────────────────────────
    let global_slots = collect_global_slots(module);
    let n_global_slots = global_slots.len();

    // ── Pass 1: compile all functions, collecting cross-function + global relocs ─
    // Each entry: (fn_name, per-function bytes, ExternalRelocs, GlobalWordRelocs)
    #[allow(clippy::type_complexity)]
    let mut fn_results: Vec<(String, Vec<u8>, Vec<Reloc>, Vec<GlobalWordReloc>)> =
        Vec::with_capacity(module.functions.len());

    // Per user function, the GC stack map the registration function will register.
    let mut fn_maps: Vec<FnStackMap> = Vec::with_capacity(module.functions.len());
    for fn_ in &module.functions {
        let (bytes, ext_relocs, glob_relocs, records) =
            compile_one_with_globals(fn_, &global_slots)
                .ok_or_else(|| AotError::BackendRefused { function: fn_.name.clone() })?;
        fn_maps.push(FnStackMap { name: fn_.name.clone(), len: bytes.len(), records });
        fn_results.push((fn_.name.clone(), bytes, ext_relocs, glob_relocs));
    }

    // ── Inject the GC entry wrapper + stack-map registration (LANG16/AOT00-T1) ─
    //
    // Two synthetic functions are appended to the compile set: `__gc_init_stackmaps`
    // (registers every user function's stack map with the GC runtime) and the entry
    // wrapper `__gc_aot_entry` (calls init, then the user entry). They are ordinary
    // functions, so `link()` gives them offsets and pass 2 patches the wrapper's `BL`s
    // (→ init, → user entry) exactly like any other intra-module call. The init's
    // `adrp`+`add` pairs that load each `func_start` are patched by a dedicated pass
    // below (`fn_addr_relocs`). The macOS object path points its `_main` entry symbol
    // at `__gc_aot_entry` (see the caller); in-process execution keeps calling the user
    // entry directly, so it is unaffected. Injecting for every module keeps the text
    // layout uniform.
    let user_entry = module.entry_point.as_deref().unwrap_or("main");
    // Guard the reserved GC symbols. `link()` is last-write-wins on duplicate names
    // and the synthetic functions are appended last, so a user function named
    // `__gc_aot_entry` / `__gc_init_stackmaps` would be silently shadowed. Worse, the
    // wrapper emits `bl <user_entry>`, so if the module's *entry point* is a reserved
    // name the wrapper resolves it to the synthetic symbol itself: naming the entry
    // `__gc_aot_entry` makes the wrapper `bl` itself (infinite recursion at start-up),
    // and `__gc_init_stackmaps` makes it call the registration function instead of the
    // program. Both defeat the wrapper. Guard BOTH the defined function names and the
    // entry name, and reject rather than miscompile — the reserved symbols belong to
    // the injected GC entry alone.
    for reserved in [GC_AOT_ENTRY, GC_INIT_STACKMAPS] {
        if user_entry == reserved || fn_results.iter().any(|(name, _, _, _)| name == reserved) {
            return Err(AotError::Linker {
                status: None,
                stderr: format!(
                    "twig-aot: symbol '{reserved}' is reserved for the GC entry wrapper \
                     and cannot be a module function or entry point",
                ),
            });
        }
    }
    // Build the wrapper first, then register IT too (with an empty ref-slot map).
    // The wrapper is `main`'s caller and is live on the stack during every
    // collection, so if it were unmapped the precise walk would resolve `main`
    // precisely but then, finding `main`'s *return address* (into the unmapped
    // wrapper) unmapped, fall back to conservatively re-scanning `main`'s whole
    // frame — re-pinning exactly the non-reference look-alikes precise roots exist
    // to reclaim. Mapping the wrapper (it holds no references, so its records name
    // no slots) keeps the entire generated call chain precise; the conservative
    // fallback then only ever covers genuine runtime/libc frames, which hold no
    // twig heap references in named slots.
    let (wrapper_bytes, wrapper_relocs) = build_gc_wrapper(user_entry)?;
    let wrapper_records = call_return_offsets(&wrapper_bytes)
        .into_iter()
        .map(|pc| (pc, Vec::new()))
        .collect();
    fn_maps.push(FnStackMap {
        name: GC_AOT_ENTRY.to_string(),
        len: wrapper_bytes.len(),
        records: wrapper_records,
    });

    let (init_bytes, init_relocs, fn_addr_relocs) = build_gc_init_stackmaps(&fn_maps)?;
    fn_results.push((GC_INIT_STACKMAPS.to_string(), init_bytes, init_relocs, Vec::new()));
    fn_results.push((GC_AOT_ENTRY.to_string(), wrapper_bytes, wrapper_relocs, Vec::new()));

    // ── LANG41: external symbols are resolved by the system linker ───────────
    //
    // Unlike the LANG40 approach (which injected a macOS-specific helper with
    // raw `SVC #0x80` syscalls), LANG41 lets BL instructions that target
    // symbols outside this module remain unpatched.  Pass 2 below collects
    // them as `ExternBranchReloc` entries; `pack_object_with_globals_and_externals`
    // emits them as `N_UNDF | N_EXT` symbol-table entries and
    // `ARM64_RELOC_BRANCH26` records so `ld` can patch them from the embedded
    // Twig AOT runtime archive (`libtwig_aot_runtime.a`).
    //
    // The runtime archive (compiled from `runtime/twig_runtime.c` by `build.rs`)
    // provides `__twig_print_i64` using `printf` from libc — portable across
    // macOS (`-lSystem`) and Linux (`-lc`).
    let mut extern_branch_relocs: Vec<ExternBranchReloc> = Vec::new();

    // ── Link: concatenate binaries and record byte offsets ──────────────────
    let plain_binaries: Vec<(String, Vec<u8>)> = fn_results
        .iter()
        .map(|(name, bytes, _, _)| (name.clone(), bytes.clone()))
        .collect();
    let (mut linked, offsets) = link(&plain_binaries);

    // ── Collect global byte relocs (LANG39) ───────────────────────────────
    //
    // Convert GlobalWordReloc (word indices relative to each function) to
    // GlobalByteReloc (byte offsets relative to the linked text section) by
    // adding the function's byte offset and multiplying word index by 4.
    let mut global_byte_relocs: Vec<GlobalByteReloc> = Vec::new();
    for (fn_name, _bytes, _, glob_relocs) in &fn_results {
        // Use an explicit error rather than `unwrap_or(&0)`.  A missing entry
        // in `offsets` here would mean a function was compiled but not linked,
        // which indicates a linker bug — silently using offset 0 would produce
        // relocation records pointing to the wrong instructions.
        let fn_byte_offset = *offsets.get(fn_name.as_str())
            .ok_or_else(|| AotError::Linker {
                status: None,
                stderr: format!(
                    "twig-aot: internal error: function '{}' missing from link offsets \
                     during global reloc collection",
                    fn_name
                ),
            })?;
        for gr in glob_relocs {
            global_byte_relocs.push(GlobalByteReloc {
                adrp_byte_offset: (fn_byte_offset + gr.adrp_word * 4) as u32,
                add_byte_offset:  (fn_byte_offset + gr.add_word  * 4) as u32,
            });
        }
    }

    // ── Pass 2: patch cross-function BL placeholders ──────────────────────
    for (fn_name, _bytes, relocs, _) in &fn_results {
        let fn_offset = *offsets.get(fn_name.as_str()).unwrap_or(&0);

        for reloc in relocs {
            // Byte offset of the placeholder `BL` instruction in the linked buffer.
            // Use checked arithmetic: a pathologically large function could make
            // word_idx * 4 or the addition overflow on 32-bit targets (or in
            // release builds on 64-bit targets where wrapping arithmetic is silent).
            let call_byte_offset = reloc.word_idx
                .checked_mul(4)
                .and_then(|off| fn_offset.checked_add(off))
                .ok_or_else(|| AotError::Linker {
                    status: None,
                    stderr: format!(
                        "twig-aot: BL relocation offset arithmetic overflow for '{}' in '{}'",
                        reloc.symbol, fn_name
                    ),
                })?;
            // Byte offset of the callee's first instruction.
            let Some(&callee_offset) = offsets.get(reloc.symbol.as_str()) else {
                // Callee not in this module — record as an external branch
                // relocation.  The system linker will patch this BL from
                // the Twig AOT runtime archive (or any dylib that defines
                // the symbol).  The `BL #0` placeholder (`0x94000000`)
                // already has the correct opcode; `ld` overwrites only the
                // 26-bit immediate.
                extern_branch_relocs.push(ExternBranchReloc {
                    byte_offset: call_byte_offset as u32,
                    symbol: reloc.symbol.clone(),
                });
                continue; // leave the BL #0 placeholder in place
            };

            // PC-relative offset in instruction words (4 bytes each).
            // The PC of `BL` is `call_byte_offset`.
            let delta_bytes = callee_offset as i64 - call_byte_offset as i64;
            let delta_words = delta_bytes / 4;

            // ARM64 BL has a 26-bit signed immediate → range ±128 MiB.
            // Guard against modules that exceed this limit so we never silently
            // patch a BL with a wrong offset (which would produce a binary that
            // jumps to an arbitrary address).
            const BL_MAX: i64 =  (1i64 << 25) - 1; //  33_554_431 words ≈ +128 MiB
            const BL_MIN: i64 = -(1i64 << 25);      // -33_554_432 words ≈ -128 MiB
            if !(BL_MIN..=BL_MAX).contains(&delta_words) {
                // The call target is >128 MiB away — this should never happen
                // for programs that fit in a single flat binary, but if it does
                // we surface it as a linker error rather than patching garbage.
                return Err(AotError::Linker {
                    status: None,
                    stderr: format!(
                        "twig-aot: BL relocation out of range for '{sym}' \
                         called from '{fn_name}' (delta={delta_words} words, \
                         allowed range [{BL_MIN}, {BL_MAX}]); \
                         module exceeds 128 MiB text size limit",
                        sym = reloc.symbol,
                    ),
                });
            }
            let imm26 = (delta_words as u32) & 0x03FFFFFF;
            let bl_word: u32 = 0x94000000 | imm26;

            // Patch the 4 bytes at call_byte_offset (little-endian).
            linked[call_byte_offset..call_byte_offset + 4]
                .copy_from_slice(&bl_word.to_le_bytes());
        }
    }

    // ── Pass 2b: patch `__gc_init_stackmaps`' func_start ADRs ─────────────────
    //
    // Each `ADR x0, F` loads a user function's absolute runtime address into x0 for
    // the `__gc_register_stackmap` call. `ADR` computes `PC + imm` at run time, so
    // `imm = target_off − adr_off` (both are `base + offset`; the base cancels) is
    // correct for *any* load address — no relocation record, no page-alignment
    // assumption (see [`FuncAddrReloc`] for why this beats `ADRP`+`ADD`). The bytes
    // carry no `ld` relocation, so they survive linking unchanged. `ADR` reaches
    // ±1 MiB; a target beyond that is rejected fail-loud rather than mis-encoded.
    if !fn_addr_relocs.is_empty() {
        let init_off = *offsets.get(GC_INIT_STACKMAPS).ok_or_else(|| AotError::Linker {
            status: None,
            stderr: "twig-aot: internal error: __gc_init_stackmaps missing from link offsets"
                .to_string(),
        })?;
        for r in &fn_addr_relocs {
            let target_off = *offsets.get(r.target.as_str()).ok_or_else(|| AotError::Linker {
                status: None,
                stderr: format!(
                    "twig-aot: internal error: func_start target '{}' missing from link offsets",
                    r.target
                ),
            })?;
            let Some(adr_byte) = r.adr_word.checked_mul(4).and_then(|o| init_off.checked_add(o))
            else {
                return Err(AotError::Linker {
                    status: None,
                    stderr: format!("twig-aot: func_start reloc offset overflow for '{}'", r.target),
                });
            };

            // Byte displacement from the ADR to the target function (base cancels).
            let disp = target_off as i64 - adr_byte as i64;
            const ADR_MAX: i64 = (1 << 20) - 1; // signed 21-bit → ±1 MiB in bytes
            const ADR_MIN: i64 = -(1 << 20);
            if !(ADR_MIN..=ADR_MAX).contains(&disp) {
                return Err(AotError::Linker {
                    status: None,
                    stderr: format!(
                        "twig-aot: func_start ADR out of range for '{}' (disp={disp} bytes); \
                         module __text exceeds the ±1 MiB ADR reach",
                        r.target
                    ),
                });
            }

            // Patch ADR: immlo → bits [30:29], immhi → bits [23:5]; keep opcode + Rd.
            let adr_old = u32::from_le_bytes(linked[adr_byte..adr_byte + 4].try_into().unwrap());
            let imm21 = (disp as u32) & 0x001F_FFFF;
            let immlo = imm21 & 0x3;
            let immhi = imm21 >> 2;
            let adr_new =
                (adr_old & !((0x3 << 29) | (0x0007_FFFF << 5))) | (immlo << 29) | (immhi << 5);
            linked[adr_byte..adr_byte + 4].copy_from_slice(&adr_new.to_le_bytes());
        }
    }

    Ok((linked, offsets, n_global_slots, global_byte_relocs, extern_branch_relocs))
}

/// Compile one `IIRFunction` to ARM64 machine code, returning the bytes,
/// any cross-function call relocations, global-access relocations, and the
/// function's **GC stack-map records** — `(pc_offset, live-ref frame slots)` per
/// safepoint — which `__gc_init_stackmaps` registers with the GC runtime.
/// Returns `None` if the function contains opcodes the backend doesn't support.
#[allow(clippy::type_complexity)]
fn compile_one_with_globals(
    fn_: &IIRFunction,
    global_slots: &HashMap<String, usize>,
) -> Option<(Vec<u8>, Vec<Reloc>, Vec<GlobalWordReloc>, Vec<(u32, Vec<i32>)>)> {
    let inferred = infer_types(fn_);
    let cir = aot_specialise(fn_, Some(&inferred));
    let ctx = FunctionContext {
        name:        &fn_.name,
        params:      &fn_.params,
        return_type: &fn_.return_type,
    };
    let (bytes, ext, glob, stack_map) =
        compile_with_globals_and_stackmap(&ctx, &cir, global_slots).ok()?;
    // Flatten each record to the fields the registration codegen needs; drop the
    // walker-only frame_size / callee_saved_mask (unused by the current ABI call).
    let records = stack_map
        .into_iter()
        .map(|r| (r.pc_offset, r.slots))
        .collect();
    Some((bytes, ext, glob, records))
}

// ===========================================================================
// Tests
// ===========================================================================

// LANG77 golden divergence guard for the native lisp runtime.  Lives as a
// lib unit-test module (not an integration test) so the build-script's
// `cargo:rustc-link-lib=static=twig_aot_runtime` directive reliably places
// the runtime archive on this binary's link line — letting the test call the
// `__dyn_*` C functions directly.
#[cfg(test)]
mod dynval_runtime_golden;

#[cfg(test)]
mod tests {
    use super::*;

    // ── AOT00-T1 x86_64 PR-x2: GC entry wrapper + no-op init ──────────────────

    /// The x86_64 wrapper is well-formed: `push rbp` … `ret`, with two intra-module
    /// `CALL rel32` relocations (→ init, → user entry) for the linker to patch.
    #[test]
    fn gc_wrapper_x86_64_is_well_formed() {
        for abi in [X86_64Abi::SysV, X86_64Abi::MsX64] {
            let (bytes, relocs) = build_gc_wrapper_x86_64("main", abi).expect("codegen");
            assert_eq!(bytes.first(), Some(&0x55), "starts with push rbp");
            assert_eq!(bytes.last(), Some(&0xC3), "ends with ret");
            let targets: Vec<&str> = relocs
                .iter()
                .filter(|r| matches!(r.kind, x86_64_encoder::ExternalRelocKind::PltRel32))
                .map(|r| r.symbol.as_str())
                .collect();
            assert!(targets.contains(&GC_INIT_STACKMAPS), "calls the init ({abi:?})");
            assert!(targets.contains(&"main"), "calls the user entry ({abi:?})");
        }
        // MsX64 reserves the 32-byte shadow space (a `sub rsp, imm32` after the
        // prologue); SysV does not → the MsX64 wrapper is strictly longer.
        let sysv = build_gc_wrapper_x86_64("main", X86_64Abi::SysV).unwrap().0;
        let ms = build_gc_wrapper_x86_64("main", X86_64Abi::MsX64).unwrap().0;
        assert!(ms.len() > sysv.len(), "MsX64 reserves shadow space");
    }

    /// The no-op x86_64 init is a bare `ret`.
    #[test]
    fn gc_noop_init_x86_64_is_ret() {
        assert_eq!(build_gc_noop_init_x86_64().expect("codegen"), vec![0xC3]);
    }

    /// After injection, the x86_64 image entry is the wrapper: `entry_off` equals the
    /// wrapper's link offset, and the byte there is `push rbp` (0x55). So the packager
    /// exports `main` at the wrapper, and libc's `_start` runs the GC entry.
    #[test]
    fn x86_64_entry_is_the_gc_wrapper() {
        let main = IIRFunction::new(
            "main", vec![], "u64",
            vec![
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "u64"),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u64"),
            ],
        );
        let mut m = IIRModule::new("m", "twig");
        m.add_or_replace(main);
        m.entry_point = Some("main".into());

        let (linked, offsets, entry_off, _relocs) =
            compile_module_x86_64_to_text(&m, X86_64Abi::SysV).expect("compiles");
        assert_eq!(
            entry_off,
            *offsets.get(GC_AOT_ENTRY).expect("wrapper present"),
            "entry redirected to the wrapper",
        );
        assert_eq!(linked[entry_off], 0x55, "wrapper begins with push rbp");
        assert!(offsets.contains_key("__gc_init_stackmaps"), "init injected");
        // The user's `main` is still present (called by the wrapper), at a different
        // offset than the entry wrapper.
        assert_ne!(offsets["main"], entry_off);
    }

    /// A module whose entry is a reserved GC symbol is rejected on the x86_64 path too.
    #[test]
    fn x86_64_reserved_entry_is_rejected() {
        let f = IIRFunction::new(
            GC_AOT_ENTRY, vec![], "u64",
            vec![
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0)], "u64"),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u64"),
            ],
        );
        let mut m = IIRModule::new("m", "twig");
        m.add_or_replace(f);
        m.entry_point = Some(GC_AOT_ENTRY.into());
        let err = compile_module_x86_64_to_text(&m, X86_64Abi::SysV);
        assert!(
            matches!(&err, Err(AotError::Linker { stderr, .. }) if stderr.contains("reserved")),
            "expected reserved-name error, got {err:?}",
        );
    }

    /// PR-x3: `build_gc_init_stackmaps_x86_64` registers exactly the functions with
    /// records — one `call __gc_register_stackmap` + one `func_start` LEA per such
    /// function — and embeds their arrays.
    #[test]
    fn gc_init_x86_64_registers_functions_with_records() {
        let maps = vec![
            FnStackMap { name: "f".into(), len: 64, records: vec![(8, vec![-16]), (20, vec![-16])] },
            FnStackMap { name: "leaf".into(), len: 8, records: vec![] }, // skipped
            FnStackMap { name: "g".into(), len: 16, records: vec![(4, vec![])] },
        ];
        let (bytes, relocs, faddr) = build_gc_init_stackmaps_x86_64(&maps).expect("codegen");
        assert_eq!(
            relocs.iter().filter(|r| r.symbol == GC_REGISTER_STACKMAP).count(),
            2,
            "one call per function with records (f, g)",
        );
        assert_eq!(faddr.len(), 2, "one func_start LEA per registered function");
        let targets: Vec<&str> = faddr.iter().map(|r| r.target.as_str()).collect();
        assert!(targets.contains(&"f") && targets.contains(&"g"));
        assert!(!targets.contains(&"leaf"));
        assert_eq!(bytes.last(), None.or(bytes.last())); // no panic on empty edge
    }

    /// PR-x3 (UAF-critical): after linking a module whose `main` calls a helper, the
    /// `func_start` `LEA rdi, [rip+disp32]` in `__gc_init_stackmaps` decodes to `main`'s
    /// exact `__text` offset for **any** load base — `LEA` adds its disp to RIP, so the
    /// base cancels (like the aarch64 `ADR`). Verified without executing, by decoding.
    #[test]
    fn x86_64_func_start_lea_resolves_to_target_offset() {
        let helper = IIRFunction::new(
            "helper", vec![], "u64",
            vec![
                IIRInstr::new("const", Some("h".into()), vec![Operand::Int(7)], "u64"),
                IIRInstr::new("ret", None, vec![Operand::Var("h".into())], "u64"),
            ],
        );
        let main = IIRFunction::new(
            "main", vec![], "u64",
            vec![
                IIRInstr::new("call", Some("r".into()), vec![Operand::Var("helper".into())], "u64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u64"),
            ],
        );
        let mut m = IIRModule::new("m", "twig");
        m.add_or_replace(helper);
        m.add_or_replace(main);
        m.entry_point = Some("main".into());

        let (linked, offsets, ..) =
            compile_module_x86_64_to_text(&m, X86_64Abi::SysV).expect("compiles");
        let init_off = *offsets.get(GC_INIT_STACKMAPS).expect("init present");
        let main_off = *offsets.get("main").expect("main present");
        let init_end = offsets.values().copied().filter(|&o| o > init_off).min().unwrap_or(linked.len());

        // Find the first `lea rdi, [rip+disp32]` = 48 8D 3D <disp32> in the init — the
        // func_start of the first registered function (`main`).
        let mut found = None;
        let mut i = init_off;
        while i + 7 <= init_end {
            if linked[i] == 0x48 && linked[i + 1] == 0x8D && linked[i + 2] == 0x3D {
                let disp = i32::from_le_bytes(linked[i + 3..i + 7].try_into().unwrap());
                // resolved = RIP + disp; RIP = end of the 7-byte LEA = i + 7.
                found = Some((i + 7) as i64 + disp as i64);
                break;
            }
            i += 1;
        }
        assert_eq!(
            found,
            Some(main_off as i64),
            "func_start LEA must resolve to main's __text offset {main_off}",
        );
    }



    /// The GC entry wrapper is well-formed: the two `BL`s (→ init, → user entry)
    /// are recorded as relocations for the linker to patch intra-module, and it
    /// ends in `RET`.
    #[test]
    fn gc_wrapper_is_well_formed() {
        let (wrap_bytes, wrap_relocs) = build_gc_wrapper("main").expect("codegen");
        assert_eq!(wrap_bytes.len() % 4, 0, "whole instructions");
        // Two BLs: one to the init, one to the user entry — both need patching.
        let targets: Vec<&str> = wrap_relocs.iter().map(|r| r.symbol.as_str()).collect();
        assert!(targets.contains(&GC_INIT_STACKMAPS), "wrapper calls the init");
        assert!(targets.contains(&"main"), "wrapper calls the user entry");
        // The wrapper is a real frame: it ends in `RET` (last word).
        let last = &wrap_bytes[wrap_bytes.len() - 4..];
        assert_eq!(last, 0xD65F03C0u32.to_le_bytes());
    }

    /// The wrapper targets whatever the module's entry is named, not a hardcoded
    /// `main`, so a program with a differently-named entry still links.
    #[test]
    fn gc_wrapper_targets_the_named_entry() {
        let (_bytes, relocs) = build_gc_wrapper("_start").expect("codegen");
        assert!(relocs.iter().any(|r| r.symbol == "_start"));
        assert!(!relocs.iter().any(|r| r.symbol == "main"));
    }

    /// `__gc_init_stackmaps` registers exactly the functions that have records:
    /// one `BL __gc_register_stackmap` and one `func_start` adrp+add reloc per such
    /// function, and it embeds each function's arrays as a data pool.
    #[test]
    fn gc_init_registers_functions_with_records() {
        let maps = vec![
            // Two safepoints, one live ref slot each (slot offset 16).
            FnStackMap {
                name: "f".into(),
                len: 64,
                records: vec![(8, vec![16]), (20, vec![16])],
            },
            // A function with NO records — must be skipped entirely.
            FnStackMap { name: "leaf".into(), len: 8, records: vec![] },
            // One safepoint, no ref slots (slots_flat empty → x7 = NULL).
            FnStackMap { name: "g".into(), len: 16, records: vec![(4, vec![])] },
        ];
        let (bytes, relocs, faddr) = build_gc_init_stackmaps(&maps).expect("codegen");

        // One register call + one func_start reloc per function WITH records (f, g).
        assert_eq!(
            relocs.iter().filter(|r| r.symbol == GC_REGISTER_STACKMAP).count(),
            2,
            "one bl __gc_register_stackmap per function with records",
        );
        assert_eq!(faddr.len(), 2, "one func_start ADR per registered function");
        let targets: Vec<&str> = faddr.iter().map(|r| r.target.as_str()).collect();
        assert!(targets.contains(&"f") && targets.contains(&"g"));
        assert!(!targets.contains(&"leaf"), "record-less function is not registered");
        // The embedded arrays are present verbatim: f's pc_offsets 8 and 20 appear as
        // little-endian data words somewhere in the pool.
        let has_word = |w: u32| bytes.windows(4).any(|c| c == w.to_le_bytes());
        assert!(has_word(8) && has_word(20), "f's pc_offsets embedded");
        assert!(bytes.len() % 4 == 0);
    }

    /// A module with no functions (only the synthetic entry) still produces a valid,
    /// callable init: prologue + epilogue + `RET`, no register calls, no relocs.
    #[test]
    fn gc_init_with_no_records_is_a_bare_frame() {
        let (bytes, relocs, faddr) = build_gc_init_stackmaps(&[]).expect("codegen");
        assert!(relocs.is_empty() && faddr.is_empty());
        // stp / mov / ldp / ret = 4 instructions.
        assert_eq!(bytes.len(), 16);
        assert_eq!(&bytes[bytes.len() - 4..], 0xD65F03C0u32.to_le_bytes(), "ends in RET");
    }

    /// The pass-2 `func_start` patch is correct: after linking a module whose `main`
    /// calls a helper, the `ADR` in `__gc_init_stackmaps` decodes to a byte
    /// displacement equal to `main_off − adr_off` — i.e. the registered `func_start`
    /// really points at the function it names, *for any load base* (an `ADR` adds its
    /// immediate to the runtime PC, so the base cancels — no page-alignment
    /// assumption, unlike an `ADRP`). This is the UAF-critical piece: a wrong
    /// `func_start` would resolve return addresses to the wrong (or no) stack map.
    /// Verified without executing, by decoding the bytes.
    #[test]
    fn func_start_adr_resolves_to_target_offset() {
        use interpreter_ir::function::IIRFunction;
        use interpreter_ir::instr::{IIRInstr, Operand};
        use interpreter_ir::module::IIRModule;

        let helper = IIRFunction::new(
            "helper", vec![], "u64",
            vec![
                IIRInstr::new("const", Some("h".into()), vec![Operand::Int(7)], "u64"),
                IIRInstr::new("ret", None, vec![Operand::Var("h".into())], "u64"),
            ],
        );
        let main = IIRFunction::new(
            "main", vec![], "u64",
            vec![
                IIRInstr::new("call", Some("r".into()), vec![Operand::Var("helper".into())], "u64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u64"),
            ],
        );
        let mut module = IIRModule::new("m", "twig");
        module.add_or_replace(helper);
        module.add_or_replace(main);
        module.entry_point = Some("main".into());

        let (text, offsets, ..) = compile_module_to_text(&module).expect("compiles");
        let init_off = *offsets.get(GC_INIT_STACKMAPS).expect("init present");
        let main_off = *offsets.get("main").expect("main present") as i64;

        // Find init's extent (up to the next function in offset order).
        let init_end = offsets
            .values()
            .copied()
            .filter(|&o| o > init_off)
            .min()
            .unwrap_or(text.len());

        // Scan init for the single func_start ADR (Rd == x0). The data-pool ADRs use
        // other Rd (x3/x6/x7), so the lone Rd-x0 ADR is `main`'s func_start.
        let mut found = None;
        let mut i = init_off;
        while i + 4 <= init_end {
            let w = u32::from_le_bytes(text[i..i + 4].try_into().unwrap());
            // ADR: bits[31]=0, bits[28:24]=10000, opcode mask 0x9F00_0000 == 0x1000_0000.
            if w & 0x9F00_0000 == 0x1000_0000 && (w & 0x1F) == 0 {
                // Decode ADR's signed 21-bit BYTE displacement.
                let immlo = ((w >> 29) & 0x3) as i64;
                let immhi = ((w >> 5) & 0x0007_FFFF) as i64;
                let mut imm21 = (immhi << 2) | immlo;
                if imm21 & (1 << 20) != 0 {
                    imm21 -= 1 << 21; // sign-extend
                }
                // ADR result = PC + imm; the base cancels, so with offsets: i + imm.
                found = Some(i as i64 + imm21);
                break;
            }
            i += 4;
        }
        assert_eq!(
            found,
            Some(main_off),
            "func_start ADR must resolve to main's __text offset {main_off}",
        );
    }

    /// A module that defines one of the reserved GC entry symbols is rejected, not
    /// silently miscompiled (the injected wrapper would otherwise shadow it and could
    /// recurse into itself at start-up).
    #[test]
    fn reserved_gc_entry_name_is_rejected() {
        for reserved in [GC_AOT_ENTRY, GC_INIT_STACKMAPS] {
            let mut m = IIRModule::new("collide", "twig");
            m.entry_point = Some(reserved.to_string());
            let f = IIRFunction::new(
                reserved,
                vec![],
                "i64",
                vec![
                    IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0)], "i64"),
                    IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
                ],
            );
            m.functions.push(f);
            let err = compile_module_macos_arm64_object(&m);
            assert!(
                matches!(&err, Err(AotError::Linker { stderr, .. }) if stderr.contains("reserved")),
                "expected a reserved-name error for {reserved}, got {err:?}",
            );
        }
    }

    /// Naming the ENTRY POINT a reserved GC symbol is rejected even when no user
    /// function is literally named that — otherwise the wrapper's `bl <user_entry>`
    /// would resolve to the synthetic symbol (self-recursion / no-op) at start-up.
    #[test]
    fn reserved_gc_entry_point_without_matching_fn_is_rejected() {
        for reserved in [GC_AOT_ENTRY, GC_INIT_STACKMAPS] {
            let mut m = IIRModule::new("collide", "twig");
            // The entry names a reserved symbol, but the module's only function is an
            // ordinary `main` — nothing is literally named `reserved`.
            m.entry_point = Some(reserved.to_string());
            let f = IIRFunction::new(
                "main",
                vec![],
                "i64",
                vec![
                    IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0)], "i64"),
                    IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
                ],
            );
            m.functions.push(f);
            let err = compile_module_macos_arm64_object(&m);
            assert!(
                matches!(&err, Err(AotError::Linker { stderr, .. }) if stderr.contains("reserved")),
                "expected a reserved-entry error for {reserved}, got {err:?}",
            );
        }
    }

    #[test]
    fn module_with_no_entry_point_errors() {
        let mut m = IIRModule::new("noent", "twig");
        m.entry_point = None;
        assert!(matches!(
            compile_module_macos_arm64_object(&m),
            Err(AotError::NoEntryPoint)
        ));
    }

    #[test]
    fn global_define_compiles_ok() {
        // `(define x 5)` is a top-level value define — the ir compiler emits a
        // `global_set` call_builtin.  LANG39 (lower_global_io + ARM64 global
        // load/store + pack_object_with_globals) now handles this end-to-end,
        // so the program must compile to a valid Mach-O object file.
        let src = "(define x 5) x";
        let result = compile_macos_arm64_object(src, "globals_test");
        assert!(result.is_ok(), "global define should compile with LANG39; got: {:?}", result.err());

        // The output must be a Mach-O MH_OBJECT (filetype == 1).
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE], "Mach-O magic");
        let filetype = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(filetype, 1, "MH_OBJECT");
    }

    #[test]
    fn fib_compiles_ok() {
        // End-to-end: the classic fibonacci program must compile to an ARM64
        // Mach-O object file without error.  This exercises the full AOT
        // preparation pipeline (builtin pre-lowering + type normalisation +
        // propagation) and the two-pass linker (cross-function BL patching
        // for the recursive fib → fib calls).
        let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)";
        let result = compile_macos_arm64_object(src, "fib");
        assert!(result.is_ok(), "fib should compile; got: {:?}", result.err());
    }

    // ---- LANG41: io_out / __twig_print_i64 via runtime library ----

    #[test]
    fn print_program_compiles_ok() {
        // A Twig program that calls `io_out` must compile end-to-end to a valid
        // Mach-O MH_OBJECT without error.  The `print` builtin lowers to
        // `io_out` in the IIR compiler, which becomes `BL __twig_print_i64` in
        // the ARM64 backend.
        //
        // Under LANG41, `__twig_print_i64` is NOT injected as a synthetic
        // function; instead, the object file contains an N_UNDF | N_EXT
        // symbol-table entry and an ARM64_RELOC_BRANCH26 record so the system
        // linker (`ld`) can resolve the symbol from the embedded runtime archive
        // (`libtwig_aot_runtime.a`).
        //
        // NOTE: (print "hello") may fail type-checking; use (print 42) which
        // produces a typed i64 value.
        let src = "(print 42)";
        let result = compile_macos_arm64_object(src, "print_test");
        assert!(
            result.is_ok(),
            "io_out program should compile with LANG41; got: {:?}",
            result.err()
        );

        // Verify Mach-O magic and file type.
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE], "Mach-O magic");
        let filetype = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(filetype, 1, "MH_OBJECT");
    }

    #[test]
    fn string_literal_print_lowers_to_heap_byte_io() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HELLO".into())], "str"),
                IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
                IIRInstr::new("const", Some("r".into()), vec![Operand::Int(0)], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        assert!(
            f.instructions.iter().all(|i| i.op != "str_const" && i.op != "print_str"),
            "native lowering should remove literal string ops: {:?}",
            f.instructions
        );
        assert!(
            f.instructions.iter().any(|i| i.op == "alloc_bytes"),
            "string literal should allocate a byte buffer"
        );
        assert_eq!(
            f.instructions.iter().filter(|i| i.op == "store_byte").count(),
            5,
            "HELLO should lower to five byte stores"
        );
        assert!(
            f.instructions.iter().any(|i| {
                i.op == "call_builtin"
                    && matches!(i.srcs.first(), Some(Operand::Var(name)) if name == "print_string")
            }),
            "print_str should lower to call_builtin print_string"
        );
    }

    #[test]
    fn string_literal_len_lowers_to_i64_const() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HELLO".into())], "str"),
                IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        let len_const = f.instructions.iter().find(|i| i.dest.as_deref() == Some("n"));
        assert!(
            matches!(len_const, Some(i) if i.op == "const" && i.srcs == vec![Operand::Int(5)]),
            "str_len over a literal should fold to const 5: {:?}",
            f.instructions
        );
        assert!(
            f.instructions.iter().all(|i| i.op != "str_len"),
            "native lowering should remove folded str_len: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_index_lowers_to_i64_const() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("ABC".into())], "str"),
                IIRInstr::new("const", Some("i".into()), vec![Operand::Int(1)], "i64"),
                IIRInstr::new("str_index", Some("b".into()), vec![
                    Operand::Var("s".into()),
                    Operand::Var("i".into()),
                ], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        let byte_const = f.instructions.iter().find(|i| i.dest.as_deref() == Some("b"));
        assert!(
            matches!(byte_const, Some(i) if i.op == "const" && i.srcs == vec![Operand::Int(66)]),
            "str_index over a literal should fold to byte 66: {:?}",
            f.instructions
        );
        assert!(
            f.instructions.iter().all(|i| i.op != "str_index"),
            "native lowering should remove folded str_index: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_index_out_of_bounds_lowers_to_trap() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("ABC".into())], "str"),
                IIRInstr::new("const", Some("i".into()), vec![Operand::Int(3)], "i64"),
                IIRInstr::new("str_index", Some("b".into()), vec![
                    Operand::Var("s".into()),
                    Operand::Var("i".into()),
                ], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        assert!(
            f.instructions.iter().any(|i| i.op == "type_assert"),
            "out-of-bounds str_index should lower to an unconditional native trap: {:?}",
            f.instructions
        );
        let byte_const = f.instructions.iter().find(|i| i.dest.as_deref() == Some("b"));
        assert!(
            matches!(byte_const, Some(i) if i.op == "const" && i.srcs == vec![Operand::Int(0)]),
            "trap path should still seed the destination for downstream typing: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_index_uses_integer_mov_metadata() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("ABC".into())], "str"),
                IIRInstr::new("const", Some("tmp".into()), vec![Operand::Int(2)], "i64"),
                IIRInstr::new("mov", Some("i".into()), vec![Operand::Var("tmp".into())], "i64"),
                IIRInstr::new("str_index", Some("b".into()), vec![
                    Operand::Var("s".into()),
                    Operand::Var("i".into()),
                ], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        let byte_const = f.instructions.iter().find(|i| i.dest.as_deref() == Some("b"));
        assert!(
            matches!(byte_const, Some(i) if i.op == "const" && i.srcs == vec![Operand::Int(67)]),
            "str_index should see integer metadata through mov: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_index_uses_computed_len_metadata() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new(
                    "str_const",
                    Some("s".into()),
                    vec![Operand::Str("ABCDE".into())],
                    "str",
                ),
                IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
                IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
                IIRInstr::new(
                    "sub",
                    Some("i".into()),
                    vec![Operand::Var("n".into()), Operand::Var("one".into())],
                    "i64",
                ),
                IIRInstr::new(
                    "str_index",
                    Some("b".into()),
                    vec![Operand::Var("s".into()), Operand::Var("i".into())],
                    "i64",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        let byte_const = f.instructions.iter().find(|i| i.dest.as_deref() == Some("b"));
        assert!(
            matches!(byte_const, Some(i) if i.op == "const" && i.srcs == vec![Operand::Int(69)]),
            "str_index should see integer metadata through str_len + sub: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_concat_len_lowers_to_i64_const() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("AB".into())], "str"),
                IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("CDE".into())], "str"),
                IIRInstr::new("str_concat", Some("s".into()), vec![
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ], "str"),
                IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        let len_const = f.instructions.iter().find(|i| i.dest.as_deref() == Some("n"));
        assert!(
            matches!(len_const, Some(i) if i.op == "const" && i.srcs == vec![Operand::Int(5)]),
            "str_len over a literal concat should fold to const 5: {:?}",
            f.instructions
        );
        assert!(
            f.instructions.iter().all(|i| i.op != "str_concat" && i.op != "str_len"),
            "native lowering should remove folded str_concat + str_len: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_slice_index_lowers_to_i64_const() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new(
                    "str_const",
                    Some("s".into()),
                    vec![Operand::Str("ABCDE".into())],
                    "str",
                ),
                IIRInstr::new("const", Some("start".into()), vec![Operand::Int(1)], "i64"),
                IIRInstr::new("const", Some("end".into()), vec![Operand::Int(4)], "i64"),
                IIRInstr::new(
                    "str_slice",
                    Some("sub".into()),
                    vec![
                        Operand::Var("s".into()),
                        Operand::Var("start".into()),
                        Operand::Var("end".into()),
                    ],
                    "str",
                ),
                IIRInstr::new("const", Some("i".into()), vec![Operand::Int(1)], "i64"),
                IIRInstr::new(
                    "str_index",
                    Some("b".into()),
                    vec![Operand::Var("sub".into()), Operand::Var("i".into())],
                    "i64",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        let byte_const = f
            .instructions
            .iter()
            .find(|i| i.dest.as_deref() == Some("b"));
        assert!(
            matches!(byte_const, Some(i) if i.op == "const" && i.srcs == vec![Operand::Int(67)]),
            "str_slice feeding str_index should fold to byte 67: {:?}",
            f.instructions
        );
        assert!(
            f.instructions
                .iter()
                .all(|i| i.op != "str_slice" && i.op != "str_index"),
            "native lowering should remove folded str_slice + str_index: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_eq_lowers_to_i64_const() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("HELLO".into())], "str"),
                IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("HELLO".into())], "str"),
                IIRInstr::new("str_eq", Some("ok".into()), vec![
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("ok".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        let eq_const = f.instructions.iter().find(|i| i.dest.as_deref() == Some("ok"));
        assert!(
            matches!(eq_const, Some(i) if i.op == "const" && i.srcs == vec![Operand::Int(1)]),
            "str_eq over equal literals should fold to const 1: {:?}",
            f.instructions
        );
        assert!(
            f.instructions.iter().all(|i| i.op != "str_eq"),
            "native lowering should remove folded str_eq: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_cmp_lowers_to_i64_const() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("ALPHA".into())], "str"),
                IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("BETA".into())], "str"),
                IIRInstr::new("str_cmp", Some("ord".into()), vec![
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("ord".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        let cmp_const = f.instructions.iter().find(|i| i.dest.as_deref() == Some("ord"));
        assert!(
            matches!(cmp_const, Some(i) if i.op == "const" && i.srcs == vec![Operand::Int(-1)]),
            "str_cmp over ordered literals should fold to const -1: {:?}",
            f.instructions
        );
        assert!(
            f.instructions.iter().all(|i| i.op != "str_cmp"),
            "native lowering should remove folded str_cmp: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_print_compiles_to_macho_object() {
        let main = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HELLO".into())], "str"),
                IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
                IIRInstr::new("const", Some("r".into()), vec![Operand::Int(0)], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
            ],
        );
        let mut module = IIRModule::new("native_string_print", "lang");
        module.functions.push(main);
        module.entry_point = Some("main".into());

        let bytes = compile_module_macos_arm64_object(&module)
            .unwrap_or_else(|e| panic!("native string print should compile: {e}"));
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE], "Mach-O magic");
        let filetype = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(filetype, 1, "MH_OBJECT");
    }

    #[test]
    fn print_program_is_valid_macho() {
        // The compiled object should be a reasonable size — header (312 bytes)
        // plus user code, symbol table, and reloc records.  Under LANG41 the
        // 208-byte injected helper is gone; the object is smaller but still
        // well above 200 bytes.
        //
        // This test catches silent compilation failures (e.g. the packager
        // returning an empty buffer) rather than verifying exact byte counts.
        let src = "(print 99)";
        let result = compile_macos_arm64_object(src, "print_size_test");
        if let Ok(bytes) = result {
            // Minimum: 312 (header) + ~60 (user fn) + 8 (1 reloc) + 48 (3 syms)
            //        + 39 (strtab) ≈ 467 bytes.  Use 300 as the floor.
            assert!(
                bytes.len() >= 300,
                "Mach-O should be ≥ 300 bytes (LANG41: no helper injection); \
                 got {} bytes",
                bytes.len()
            );
        }
    }

    #[test]
    fn empty_main_compiles_to_object_bytes() {
        use interpreter_ir::function::IIRFunction;
        use interpreter_ir::instr::{IIRInstr, Operand};

        let main = IIRFunction::new(
            "main", vec![], "i64",
            vec![
                IIRInstr::new("const", Some("v0".into()),
                              vec![Operand::Int(0)], "i64"),
                IIRInstr::new("ret", None,
                              vec![Operand::Var("v0".into())], "i64"),
            ],
        );
        let mut m = IIRModule::new("hello", "twig");
        m.add_or_replace(main);
        m.entry_point = Some("main".into());

        let bytes = compile_module_macos_arm64_object(&m).expect("ok");
        // Mach-O magic for 64-bit LE is 0xCFFAEDFE.  This is an MH_OBJECT.
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
        let filetype = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(filetype, 1, "MH_OBJECT");
    }

    // ── LANG42: refinement checker wired into twig-aot ────────────────────
    //
    // The Twig IR compiler already parses `(param : (Int lo hi))` syntax and
    // wires the annotation into `IIRFunction::param_refinements`.  LANG42 adds
    // the pass that *checks* those annotations before codegen.
    //
    // These tests use the source-string path (`compile_macos_arm64_object`) so
    // we don't need to add `lang-refined-types` to twig-aot's dev-dependencies.

    #[test]
    fn refinement_violation_becomes_aot_error() {
        // The worked example from the LANG42 spec (top-level expression form,
        // same pattern as `fib_compiles_ok` which avoids BackendRefused):
        //
        //   (define (ascii-info (codepoint : (Int 0 128))) codepoint)
        //   (ascii-info 200)        ← 200 violates [0, 128)
        //
        // The refinement pass fires BEFORE prepare_module_for_aot so we catch
        // the violation even though the module might fail codegen later.
        //
        // 200 violates (Int 0 128) → ProvenUnsafe → AotError::RefinementViolations.
        let src = "(define (ascii-info (codepoint : (Int 0 128))) codepoint) \
                   (ascii-info 200)";
        let result = compile_macos_arm64_object(src, "refinement_test");
        assert!(
            matches!(result, Err(AotError::RefinementViolations(_))),
            "expected RefinementViolations, got: {:?}",
            result.as_ref().err(),
        );
        if let Err(AotError::RefinementViolations(errs)) = result {
            assert!(!errs.is_empty(), "at least one error expected");
            assert_eq!(errs[0].counter_example, 200, "counter-example should be 200");
        }
    }

    #[test]
    fn safe_annotated_program_compiles_ok() {
        // (ascii-info 42) — 42 ∈ [0, 128) → ProvenSafe → no refinement error.
        // Same top-level expression form as fib_compiles_ok.
        let src = "(define (ascii-info (codepoint : (Int 0 128))) codepoint) \
                   (ascii-info 42)";
        let result = compile_macos_arm64_object(src, "refinement_safe");
        assert!(
            result.is_ok(),
            "safe annotated program should compile; got: {:?}",
            result.err(),
        );
    }

    // ---- Multi-function cross-function patching (x86_64) ----

    /// Compile a two-function module where `main` calls `helper`, then
    /// inspect the resulting linked text to verify:
    ///
    /// 1. The CALL site contains a non-zero rel32 displacement (NOT the
    ///    encoder's `00 00 00 00` placeholder).
    /// 2. The displacement points at `helper`'s byte offset in the
    ///    linked text.
    /// 3. No external relocation for `helper` is emitted to the packager
    ///    (it was resolved in place).
    /// 4. External relocs for runtime helpers (e.g. `__twig_print_i64`)
    ///    DO still pass through to the packager.
    #[test]
    fn x86_64_cross_function_call_patched_in_place() {
        use interpreter_ir::instr::{IIRInstr, Operand};

        // Module: helper() returns 7; main() returns helper().
        // Plain (non-recursive) so the test isn't sensitive to inlining
        // or constant-folding passes.
        let helper = interpreter_ir::function::IIRFunction::new(
            "helper", vec![], "u64",
            vec![
                IIRInstr::new("const", Some("v0".into()),
                              vec![Operand::Int(7)], "u64"),
                IIRInstr::new("ret", None,
                              vec![Operand::Var("v0".into())], "u64"),
            ],
        );
        let main = interpreter_ir::function::IIRFunction::new(
            "main", vec![], "u64",
            vec![
                IIRInstr::new("call", Some("r".into()),
                              vec![Operand::Var("helper".into())], "u64"),
                IIRInstr::new("ret", None,
                              vec![Operand::Var("r".into())], "u64"),
            ],
        );
        let mut module = IIRModule::new("twofn", "twig");
        module.add_or_replace(helper);
        module.add_or_replace(main);
        module.entry_point = Some("main".into());

        let (linked, offsets, _entry_off, ext) =
            compile_module_x86_64_to_text(&module, X86_64Abi::SysV).unwrap();

        // helper symbol resolved internally → no external reloc for it.
        assert!(
            !ext.iter().any(|r| r.symbol == "helper"),
            "internal call to 'helper' should be patched in place, not exported"
        );

        // Find the CALL site in main: opcode 0xE8 followed by 4 disp bytes.
        // After patching, those 4 bytes should NOT all be zero.
        let main_off = *offsets.get("main").unwrap();
        let helper_off = *offsets.get("helper").unwrap();
        // The call lives somewhere in main's body; we search for the 0xE8
        // opcode within main's range.
        let main_end = main_off + linked.len(); // upper bound; main is the last fn here
        let mut call_site = None;
        for i in main_off..(main_end.saturating_sub(5)).min(linked.len() - 5) {
            if linked[i] == 0xE8 {
                // Check the next 4 bytes aren't all zero (we want a patched call).
                if linked[i+1..i+5] != [0, 0, 0, 0] {
                    call_site = Some(i);
                    break;
                }
            }
        }
        let call_site = call_site.expect("expected a patched CALL rel32 in main's body");

        // disp32 starts at call_site+1; verify it equals helper - (call_site+1) - 4.
        let actual_disp = i32::from_le_bytes([
            linked[call_site + 1], linked[call_site + 2],
            linked[call_site + 3], linked[call_site + 4],
        ]);
        let expected_disp = (helper_off as i64) - (call_site as i64 + 1) - 4;
        assert_eq!(actual_disp as i64, expected_disp,
            "CALL disp32 should target 'helper' (off={helper_off}); \
             site_disp_slot={}, expected disp={expected_disp}, got={actual_disp}",
             call_site + 1);
    }

    /// Confirm that calls to a NON-module symbol (e.g. `__twig_print_i64`)
    /// remain as external relocs, even when multi-function patching is
    /// otherwise active.
    #[test]
    fn x86_64_external_call_remains_in_relocs() {
        use interpreter_ir::instr::{IIRInstr, Operand};

        // main() { io_out 42 ; return 0 }
        let main = interpreter_ir::function::IIRFunction::new(
            "main", vec![], "u64",
            vec![
                IIRInstr::new("const", Some("v".into()),
                              vec![Operand::Int(42)], "u64"),
                IIRInstr::new("io_out", None,
                              vec![Operand::Var("v".into())], "void"),
                IIRInstr::new("const", Some("z".into()),
                              vec![Operand::Int(0)], "u64"),
                IIRInstr::new("ret", None,
                              vec![Operand::Var("z".into())], "u64"),
            ],
        );
        let mut module = IIRModule::new("ioonly", "twig");
        module.add_or_replace(main);
        module.entry_point = Some("main".into());

        let (_linked, _offsets, _entry_off, ext) =
            compile_module_x86_64_to_text(&module, X86_64Abi::SysV).unwrap();

        // Exactly one external reloc, naming the runtime helper.
        let plt: Vec<_> = ext.iter()
            .filter(|r| r.symbol == "__twig_print_i64").collect();
        assert_eq!(plt.len(), 1,
            "expected one external reloc for __twig_print_i64, got {ext:?}");
    }

    // ---- --emit-object cross-OS object emission ----

    #[test]
    fn emit_object_to_disk_writes_linux_o() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("hello.twig");
        std::fs::write(&src, "42").unwrap();
        let base = dir.path().join("out");

        let emitted = emit_object_to_disk(&src, &base, EmitObjectTarget::LinuxX86_64)
            .expect("emit ok");

        assert!(emitted.object_path.exists());
        assert_eq!(
            emitted.object_path.extension().and_then(|s| s.to_str()),
            Some("o"),
            "Linux objects use .o extension",
        );

        // ELF magic at byte 0.
        let bytes = std::fs::read(&emitted.object_path).unwrap();
        assert_eq!(&bytes[0..4], &[0x7F, b'E', b'L', b'F']);
    }

    #[test]
    fn emit_object_to_disk_writes_windows_obj() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("hello.twig");
        std::fs::write(&src, "42").unwrap();
        let base = dir.path().join("out");

        let emitted = emit_object_to_disk(&src, &base, EmitObjectTarget::WindowsX86_64)
            .expect("emit ok");

        assert!(emitted.object_path.exists());
        assert_eq!(
            emitted.object_path.extension().and_then(|s| s.to_str()),
            Some("obj"),
            "Windows objects use .obj extension",
        );

        // IMAGE_FILE_MACHINE_AMD64 (0x8664) LE at byte 0.
        let bytes = std::fs::read(&emitted.object_path).unwrap();
        assert_eq!(&bytes[0..2], &[0x64, 0x86]);
    }

    #[test]
    fn emit_object_runtime_path_is_none_when_archive_is_stub() {
        // Iterate the three targets.  Each host produces a real runtime
        // archive for its own target and stubs for the others; emit_object
        // should write a real `_runtime.*` file only when the bytes are
        // a real archive (>4 bytes).  Verify both branches are exercised.
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("hello.twig");
        std::fs::write(&src, "42").unwrap();

        let mut had_real = false;
        let mut had_stub = false;
        for tgt in [EmitObjectTarget::MacosArm64,
                    EmitObjectTarget::LinuxX86_64,
                    EmitObjectTarget::WindowsX86_64] {
            let base = dir.path().join(format!("out_{tgt:?}"));
            let e = emit_object_to_disk(&src, &base, tgt).expect("emit ok");
            if e.runtime_archive_path.is_some() { had_real = true; }
            else { had_stub = true; }
        }
        assert!(had_real, "at least one target must have a real runtime archive on the host");
        assert!(had_stub, "at least one target should be a stub on this host");
    }

    // ---- LANG-STR-RT: runtime string ABI (length-prefixed buffers) ----

    #[test]
    fn string_param_len_lowers_to_field_load() {
        // `str_len` on a variable not in the `strings` map (simulates a function
        // parameter) must fall back to `field_load s, 0 → dest` which reads the
        // 8-byte length header from the LANG-STR-RT buffer at runtime.
        let mut f = IIRFunction::new(
            "strlen",
            vec![("s".into(), "str".into())],
            "i64",
            vec![
                IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        assert!(
            f.instructions.iter().all(|i| i.op != "str_len"),
            "str_len on a parameter should be removed by lowering: {:?}",
            f.instructions
        );
        let field_load = f.instructions.iter().find(|i| i.dest.as_deref() == Some("n"));
        assert!(
            matches!(
                field_load,
                Some(i) if i.op == "field_load"
                    && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == "s")
                    && matches!(i.srcs.get(1), Some(Operand::Int(0)))
            ),
            "runtime str_len should lower to field_load s, 0: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_param_eq_lowers_to_call_builtin_str_eq() {
        // `str_eq` where at least one operand is a runtime string (not in the
        // `strings` map) must lower to `call_builtin "str_eq" a b → dest`.
        let mut f = IIRFunction::new(
            "same",
            vec![("a".into(), "str".into()), ("b".into(), "str".into())],
            "i64",
            vec![
                IIRInstr::new("str_eq", Some("ok".into()), vec![
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("ok".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        assert!(
            f.instructions.iter().all(|i| i.op != "str_eq"),
            "str_eq on parameters should be removed by lowering: {:?}",
            f.instructions
        );
        let call = f.instructions.iter().find(|i| i.dest.as_deref() == Some("ok"));
        assert!(
            matches!(
                call,
                Some(i) if i.op == "call_builtin"
                    && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "str_eq")
                    && matches!(i.srcs.get(1), Some(Operand::Var(v)) if v == "a")
                    && matches!(i.srcs.get(2), Some(Operand::Var(v)) if v == "b")
            ),
            "runtime str_eq should lower to call_builtin str_eq: {:?}",
            f.instructions
        );
    }

    #[test]
    fn runtime_str_concat_lowers_to_call_builtin_str_concat() {
        // `str_concat` where at least one operand is a runtime string (a function
        // parameter here — not in the `strings` literal map) must lower to
        // `call_builtin "str_concat" a b → dest`, delegating to `__twig_str_concat`.
        let mut f = IIRFunction::new(
            "join",
            vec![("a".into(), "str".into()), ("b".into(), "str".into())],
            "str",
            vec![
                IIRInstr::new("str_concat", Some("s".into()), vec![
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ], "str"),
                IIRInstr::new("ret", None, vec![Operand::Var("s".into())], "str"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        assert!(
            f.instructions.iter().all(|i| i.op != "str_concat"),
            "runtime str_concat should be removed by lowering: {:?}",
            f.instructions
        );
        let call = f.instructions.iter().find(|i| i.dest.as_deref() == Some("s"));
        assert!(
            matches!(
                call,
                Some(i) if i.op == "call_builtin"
                    && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "str_concat")
                    && matches!(i.srcs.get(1), Some(Operand::Var(v)) if v == "a")
                    && matches!(i.srcs.get(2), Some(Operand::Var(v)) if v == "b")
            ),
            "runtime str_concat should lower to call_builtin str_concat: {:?}",
            f.instructions
        );
    }

    #[test]
    fn literal_str_concat_still_folds_to_data_segment() {
        // Regression guard for the fold FAST path: when BOTH operands are known
        // printable literals, `str_concat` must still fold to a baked data-segment
        // buffer (a `mov` from the literal buffer), NOT a runtime call_builtin.
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "str",
            vec![
                IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("OK".into())], "str"),
                IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("!".into())], "str"),
                IIRInstr::new("str_concat", Some("s".into()), vec![
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ], "str"),
                IIRInstr::new("ret", None, vec![Operand::Var("s".into())], "str"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        assert!(
            f.instructions.iter().all(|i| i.op != "str_concat"),
            "literal str_concat should fold away, not survive: {:?}",
            f.instructions
        );
        assert!(
            f.instructions.iter().all(|i| i.op != "call_builtin"),
            "literal str_concat must NOT lower to a runtime call_builtin: {:?}",
            f.instructions
        );
    }

    #[test]
    fn string_literal_buffer_has_length_header() {
        // After lowering `str_const "HI"`, the emitted instructions must include a
        // `field_store buf, 0, len_var` to write the 8-byte LANG-STR-RT header,
        // allocate 10 bytes (8 + 2), and write the 2 bytes at offsets 8 and 9.
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HI".into())], "str"),
                IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        // Must allocate 2 + 8 = 10 bytes.
        let alloc = f.instructions.iter().find(|i| i.op == "alloc_bytes").expect("alloc_bytes");
        let alloc_src = alloc.srcs.first().expect("alloc size operand");
        // alloc_bytes takes a Var pointing to the tlen const, not a literal Int.
        // Verify the total-len const feeds 10.
        if let Operand::Var(tlen_var) = alloc_src {
            let tlen_const = f.instructions.iter().find(|i| i.dest.as_deref() == Some(tlen_var.as_str()));
            assert!(
                matches!(tlen_const, Some(i) if i.srcs == vec![Operand::Int(10)]),
                "alloc should request 10 bytes (8 header + 2 data): {:?}",
                f.instructions
            );
        } else {
            panic!("alloc_bytes arg should be a Var, got {:?}", alloc_src);
        }

        // Must have a field_store for the length header.
        assert!(
            f.instructions.iter().any(|i| i.op == "field_store"),
            "buf must have a field_store for the length header: {:?}",
            f.instructions
        );

        // Byte stores should be at offsets 8 and 9, not 0 and 1.
        let store_offsets: Vec<i64> = f.instructions.iter()
            .filter(|i| i.op == "store_byte")
            .filter_map(|i| {
                if let Some(Operand::Var(off_var)) = i.srcs.get(1) {
                    f.instructions.iter().find(|c| c.dest.as_deref() == Some(off_var))
                        .and_then(|c| c.srcs.first())
                        .and_then(|op| if let Operand::Int(v) = op { Some(*v) } else { None })
                } else { None }
            })
            .collect();
        assert_eq!(store_offsets, vec![8, 9], "byte stores must be at offset 8..9: {:?}", f.instructions);
    }

    // ── E4-dyn (E4d-4): runtime (branch-selected) strings ────────────────────

    /// A string variable assigned by `str_const` in two different basic blocks
    /// is chosen by control flow, so `print_str` on it must read the length from
    /// the buffer header at run time (`field_load` index 0) rather than folding a
    /// single branch's static length. The two branches here differ in length
    /// ("LONGER" = 6 vs "HI" = 2), so a static-length fold would be observably
    /// wrong. This is the native sibling of iir-to-llvm's E4d-2 and iir-to-wasm's
    /// E4d-3 runtime paths.
    #[test]
    fn branch_selected_string_reads_length_at_runtime() {
        // cond = 1; if !cond goto Lelse
        // Lthen: A = "LONGER"; goto Ldone
        // Lelse: A = "HI"
        // Ldone: print_str A; ret 0
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "i64"),
                IIRInstr::new("jmp_if_false", None,
                    vec![Operand::Var("cond".into()), Operand::Var("Lelse".into())], "void"),
                IIRInstr::new("label", None, vec![Operand::Var("Lthen".into())], "void"),
                IIRInstr::new("str_const", Some("A".into()), vec![Operand::Str("LONGER".into())], "str"),
                IIRInstr::new("jmp", None, vec![Operand::Var("Ldone".into())], "void"),
                IIRInstr::new("label", None, vec![Operand::Var("Lelse".into())], "void"),
                IIRInstr::new("str_const", Some("A".into()), vec![Operand::Str("HI".into())], "str"),
                IIRInstr::new("label", None, vec![Operand::Var("Ldone".into())], "void"),
                IIRInstr::new("print_str", None, vec![Operand::Var("A".into())], "void"),
                IIRInstr::new("const", Some("r".into()), vec![Operand::Int(0)], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        // Both branches still build their `[i64 len][bytes]` heap buffers.
        assert_eq!(
            f.instructions.iter().filter(|i| i.op == "alloc_bytes").count(),
            2,
            "each branch's string must still allocate its own buffer: {:?}",
            f.instructions
        );
        // The runtime length read: `field_load` the header (index 0) from `A`.
        assert!(
            f.instructions.iter().any(|i| {
                i.op == "field_load"
                    && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == "A")
                    && matches!(i.srcs.get(1), Some(Operand::Int(0)))
            }),
            "print_str of a branch-selected string must read the length header at \
             run time (field_load A[0]), not fold a static length: {:?}",
            f.instructions
        );
        // And it still calls the print runtime.
        assert!(
            f.instructions.iter().any(|i| {
                i.op == "call_builtin"
                    && matches!(i.srcs.first(), Some(Operand::Var(name)) if name == "print_string")
            }),
            "print_str should still lower to call_builtin print_string: {:?}",
            f.instructions
        );
    }

    /// A string reassigned twice *straight-line* (one basic block) is NOT
    /// promoted: the last-writer-wins literal tracking is exactly right, so
    /// `print_str` keeps the static-length fast path and emits no runtime
    /// `field_load`. Mirrors the E4d-2/E4d-3 "straight-line not promoted" rule.
    #[test]
    fn straight_line_reassignment_keeps_static_length() {
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("OK".into())], "str"),
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("NO".into())], "str"),
                IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
                IIRInstr::new("const", Some("r".into()), vec![Operand::Int(0)], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);

        assert!(
            !f.instructions.iter().any(|i| i.op == "field_load"),
            "a straight-line reassignment keeps the static-length fast path (no \
             runtime field_load): {:?}",
            f.instructions
        );
    }

    /// Regression: a branch-selected string (E4d-4 promoted var) has ONE alias
    /// name (`s`) fed by TWO buffers — one per branch. `strip_dead_aot_string_allocs`
    /// must keep BOTH buffers alive; a map keyed by alias would drop all but the
    /// last, so the not-last branch would read a freed/empty buffer at run time
    /// (the E4-dyn foothold only dodged this by printing the last-defined branch).
    #[test]
    fn multi_block_string_keeps_every_branch_buffer() {
        // if cond goto Lelse; Lthen: s = "LONGER"; goto Ldone; Lelse: s = "HI";
        // Ldone: print s
        let mut f = IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "i64"),
                IIRInstr::new("jmp_if_false", None,
                    vec![Operand::Var("cond".into()), Operand::Var("Lelse".into())], "void"),
                IIRInstr::new("label", None, vec![Operand::Var("Lthen".into())], "void"),
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("LONGER".into())], "str"),
                IIRInstr::new("jmp", None, vec![Operand::Var("Ldone".into())], "void"),
                IIRInstr::new("label", None, vec![Operand::Var("Lelse".into())], "void"),
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HI".into())], "str"),
                IIRInstr::new("label", None, vec![Operand::Var("Ldone".into())], "void"),
                IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
                IIRInstr::new("const", Some("r".into()), vec![Operand::Int(0)], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
            ],
        );

        lower_string_literals_for_aot(&mut f);
        strip_dead_aot_string_allocs(&mut f);

        // Both branch buffers must survive — neither is dead when `s` is live.
        assert_eq!(
            f.instructions.iter().filter(|i| i.op == "alloc_bytes").count(),
            2,
            "both branch buffers must survive strip_dead_aot_string_allocs: {:?}",
            f.instructions
        );
        // And both alias-movs (`mov s = buf`) survive so the taken branch's buffer
        // reaches `s`.
        assert_eq!(
            f.instructions
                .iter()
                .filter(|i| i.op == "mov" && i.dest.as_deref() == Some("s"))
                .count(),
            2,
            "both alias-movs feeding `s` must survive: {:?}",
            f.instructions
        );
    }
}
