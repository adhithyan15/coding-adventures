//! # aot-debug — IIR ↔ native-debug-info bridge.
//!
//! Translates an [`interpreter_ir::IIRModule`]'s source-location data
//! (`IIRFunction::source_map`) into a debug-sidecar blob, then hands that
//! blob to [`native_debug_info::embed_debug_info`] to inject DWARF 4 or
//! CodeView 4 sections into an AOT-compiled binary.
//!
//! ## Pipeline position
//!
//! ```text
//! IIRModule ──┐
//!             │ build_sidecar_from_iir() ─► sidecar bytes ─┐
//!             │                                            │
//!             └───────────────────────────►─────────────────┤
//!                                                           ▼
//! AOT-compiled binary  ────► embed_iir_debug_info() ──► .debug_* augmented binary
//!  (twig-aot output)                                  (gdb/lldb/WinDbg-friendly)
//! ```
//!
//! ## Why a separate crate
//!
//! 1. **No twig-aot churn yet.**  This first slice (AOT-DBG-01) lands the
//!    sidecar-builder + embed orchestrator, with tests against synthetic
//!    fixtures.  A follow-up PR (AOT-DBG-02) wires it into
//!    `twig-aot::compile_module_to_*_executable`.  Splitting the work
//!    means twig-aot's binary emitter stays untouched in this round.
//! 2. **Reusable across backends.**  `iir-to-llvm` (LLVM IR → `llc` →
//!    native) can also call `build_sidecar_from_iir` if it ever wants to
//!    emit `.dwo` files directly, without duplicating IIR → sidecar logic.
//! 3. **Testable in isolation.**  Sidecar correctness checks don't require
//!    a real ELF/Mach-O/PE binary on hand — see
//!    `tests/bridge.rs`.
//!
//! ## What this crate does NOT do
//!
//! * Does NOT compute function byte-offsets — those come from the AOT
//!   backend (twig-aot already returns them from
//!   `compile_module_x86_64_to_text`'s return tuple).
//! * Does NOT pick a binary format — that's `native_debug_info`'s
//!   `ArtifactInfo.target` field.
//! * Does NOT understand the source text itself — line/column come from
//!   the `SourceLoc` already threaded through IIR by the frontend.

use std::collections::HashMap;

use debug_sidecar::DebugSidecarWriter;
use interpreter_ir::{IIRModule, SourceLoc};
use native_debug_info::{embed_debug_info, ArtifactInfo};

// ===========================================================================
// build_sidecar_from_iir
// ===========================================================================

/// Build a debug-sidecar blob from an IIR module's source-location data.
///
/// Walks every function in the module, registering it with the writer and
/// emitting one line-table row per non-synthetic
/// [`SourceLoc`](interpreter_ir::SourceLoc).  Synthetic source locations
/// (`SourceLoc::SYNTHETIC`, i.e. line `0`/column `0`) are **skipped** —
/// the contract for synthetics is "no real source counterpart", and
/// emitting them would produce confusing 0-line/0-column entries in the
/// debugger.
///
/// # Parameters
///
/// * `module` — the IIR module to walk.
/// * `source_file_path` — file path the debugger should display in the
///   `Frame.source.path` slot.  Pass the user's original source path
///   (e.g. `"hello.bas"`).  When the path is empty, we still register a
///   single file with an empty path so file_ids stay valid.
///
/// # Returns
///
/// Sidecar bytes ready to hand to [`embed_iir_debug_info`] or directly to
/// [`native_debug_info::embed_debug_info`].
pub fn build_sidecar_from_iir(module: &IIRModule, source_file_path: &str) -> Vec<u8> {
    let mut writer = DebugSidecarWriter::new();
    // We currently support a single source file per module — the front-end
    // gives us one path.  Future versions can register multi-file modules
    // by walking the `SourceLoc`s for distinct file_ids first.
    let file_id = writer.add_source_file(source_file_path, b"");

    for func in &module.functions {
        // Match the convention used by twig-vm's debugger: param_count
        // here is the IIR-declared param count, which matches what
        // gdb/lldb shows as the function's "arity" in the frame info.
        writer.begin_function(&func.name, 0, func.params.len());

        // `source_map[i]` corresponds to `instructions[i]`.  IIR
        // guarantees they're parallel arrays — we trust that contract and
        // walk by index.  Mismatched lengths (a frontend bug) are
        // tolerated: we walk `min(map_len, instr_len)` to avoid panics.
        let n = func
            .source_map
            .len()
            .min(func.instructions.len());
        for i in 0..n {
            let loc: SourceLoc = func.source_map[i];
            if loc.is_synthetic() {
                continue; // skip the "no source" sentinel
            }
            writer.record(&func.name, i, file_id, loc.line, loc.column);
        }

        writer.end_function(&func.name, func.instructions.len());
    }

    writer.finish()
}

// ===========================================================================
// embed_iir_debug_info
// ===========================================================================

/// One-shot orchestrator: build a sidecar from IIR, then embed DWARF/CodeView.
///
/// Hand this the raw output of twig-aot (the linked binary), the platform
/// metadata in [`ArtifactInfo`], the IIR module that produced it, and the
/// user's source path.  We do the sidecar build for you and call into
/// `native-debug-info`.
///
/// # Why a separate `build_sidecar_from_iir`?
///
/// Two reasons:
///
/// 1. **Testability.**  A unit test that exercises just the IIR-walking
///    logic doesn't need to construct a valid ELF/Mach-O/PE binary.
/// 2. **Future composability.**  A caller might want to merge sidecars
///    from multiple modules before embedding (e.g. for a multi-file
///    project).  Exposing the sidecar build separately lets them do so
///    without re-implementing the IIR walk.
///
/// # Errors
///
/// Returns whatever `native_debug_info::embed_debug_info` returns:
///
/// * `Err(msg)` if `artifact.target` is unrecognised.
/// * `Err(msg)` if the underlying emitter (DWARF or CodeView) fails.
///
/// The sidecar build itself cannot fail — `DebugSidecarWriter`'s
/// operations are infallible.
pub fn embed_iir_debug_info(
    binary: &[u8],
    artifact: &ArtifactInfo,
    module: &IIRModule,
    source_file_path: &str,
) -> Result<Vec<u8>, String> {
    let sidecar = build_sidecar_from_iir(module, source_file_path);
    embed_debug_info(binary, artifact, &sidecar)
}

// ===========================================================================
// Convenience constructors for ArtifactInfo
// ===========================================================================

/// Build an [`ArtifactInfo`] for an ELF / Mach-O binary (DWARF targets).
///
/// Common case: the caller already has the function-name → offset map
/// from `twig-aot::compile_module_x86_64_to_text` (or the AArch64 sibling)
/// in `HashMap<String, usize>` form.  Converts the offsets to `u64` and
/// stuffs everything into an `ArtifactInfo` for DWARF embedding.
///
/// CodeView (PE) targets should use [`artifact_info_pe`] instead.
pub fn artifact_info_elf_or_macho(
    target: &str,
    load_address: u64,
    fn_offsets: &HashMap<String, usize>,
    code_size: usize,
) -> ArtifactInfo {
    let symbol_table_u64: HashMap<String, u64> = fn_offsets
        .iter()
        .map(|(k, &v)| (k.clone(), v as u64))
        .collect();
    ArtifactInfo {
        target: target.to_string(),
        load_address,
        image_base: 0,
        symbol_table_u64,
        symbol_table_u32: HashMap::new(),
        code_size: code_size as u64,
        code_rva: 0,
        code_section_index: 1,
    }
}

/// Build an [`ArtifactInfo`] for a PE binary (CodeView target).
///
/// CodeView uses 32-bit relative offsets, so we convert `fn_offsets`
/// (`usize`) down to `u32` and bail if any value exceeds `u32::MAX` —
/// a >4 GiB code section is not a realistic input but we'd rather
/// return an error than silently truncate.
pub fn artifact_info_pe(
    image_base: u64,
    fn_offsets: &HashMap<String, usize>,
    code_rva: u32,
    code_section_index: u16,
) -> Result<ArtifactInfo, String> {
    let mut symbol_table_u32 = HashMap::new();
    for (name, &off) in fn_offsets {
        let off32: u32 = u32::try_from(off).map_err(|_| {
            format!(
                "aot-debug: symbol {name:?} offset {off} exceeds u32::MAX \
                 (PE/CodeView uses 32-bit relative offsets)"
            )
        })?;
        symbol_table_u32.insert(name.clone(), off32);
    }
    Ok(ArtifactInfo {
        target: "windows".to_string(),
        load_address: 0,
        image_base,
        symbol_table_u64: HashMap::new(),
        symbol_table_u32,
        code_size: 0,
        code_rva,
        code_section_index,
    })
}
