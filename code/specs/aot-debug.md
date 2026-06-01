# aot-debug — IIR ↔ native-debug-info bridge

**Status:** v0.1.0 (AOT-DBG-01 — bridge layer + tests).
**Plan:** [`MULTILANG-BACKEND-PLAN.md`](MULTILANG-BACKEND-PLAN.md) §AOT-DBG

## What problem this crate solves

AOT-compiled native binaries from `twig-aot` don't currently carry source
location data, so `gdb` / `lldb` / WinDbg can't single-step through the
user's `.bas` / `.nib` / `.twig` source files.  The pieces to fix that
already exist:

1. **Source locations** are threaded through IIR by every front-end
   (tasks #33, #34, #35).
2. **DWARF 4 / CodeView 4 embedding** is implemented in the
   `native-debug-info` crate, which exposes `embed_debug_info(binary,
   artifact, sidecar_bytes)`.
3. **Function byte-offsets** are returned internally by twig-aot's
   `compile_module_x86_64_to_text` and AArch64 sibling.

The only missing layer was a bridge that:

- Walks an `IIRModule.source_map` and builds a `debug-sidecar` blob.
- Packages function-offset maps into the shape `native-debug-info`
  wants (`ArtifactInfo` with `symbol_table_u64` for DWARF / `_u32` for
  CodeView).

That's what `aot-debug` provides.

## Scope of v0.1.0 (AOT-DBG-01)

| Component | Status |
|-----------|--------|
| `build_sidecar_from_iir(module, src_path)` | done |
| `embed_iir_debug_info(binary, artifact, module, src_path)` orchestrator | done |
| `artifact_info_elf_or_macho(...)` helper | done |
| `artifact_info_pe(...)` helper with u32 overflow guard | done |
| Bridge tests (7) | done |
| twig-aot wiring (`compile_module_to_*_executable` carries DWARF) | **deferred to v0.2.0 (AOT-DBG-02)** |
| `lang-aot --debug` flag | **deferred to v0.3.0 (AOT-DBG-03)** |
| End-to-end gdb scripted run on a compiled binary | **deferred to v0.4.0 (AOT-DBG-04)** |

Splitting v0.1.0 off keeps the binary-emitter changes isolated for
review.  All v0.1.0 work is pure data plumbing — no changes to
twig-aot, x86_64-encoder, aarch64-encoder, or any link step.

## Public surface

```rust
// Build sidecar bytes from IIR source-map data.
pub fn build_sidecar_from_iir(
    module: &IIRModule,
    source_file_path: &str,
) -> Vec<u8>;

// One-shot: build sidecar + embed DWARF/CodeView into the binary.
pub fn embed_iir_debug_info(
    binary: &[u8],
    artifact: &ArtifactInfo,
    module: &IIRModule,
    source_file_path: &str,
) -> Result<Vec<u8>, String>;

// ArtifactInfo constructors — convert twig-aot's `HashMap<String, usize>`
// offset map into the DWARF (u64) or CodeView (u32) shape.
pub fn artifact_info_elf_or_macho(
    target: &str,
    load_address: u64,
    fn_offsets: &HashMap<String, usize>,
    code_size: usize,
) -> ArtifactInfo;

pub fn artifact_info_pe(
    image_base: u64,
    fn_offsets: &HashMap<String, usize>,
    code_rva: u32,
    code_section_index: u16,
) -> Result<ArtifactInfo, String>;
```

## Design choices

### Filter synthetic `SourceLoc`s

`SourceLoc::SYNTHETIC` (`line == 0 && column == 0`) means "no real source
counterpart" — emitting those rows would produce `???:???` entries in
gdb/lldb, strictly worse than no entry at all.  The bridge filters them.

### Tolerate length mismatch

IIR guarantees `source_map` is parallel to `instructions`.  If a
front-end bug breaks that invariant, we walk
`min(source_map.len(), instructions.len())` rather than panic.  A future
hardening pass could promote this to an `Err`, but for now graceful
degradation is the right default for an unblocked debugger pipeline.

### Reject oversized PE offsets

CodeView uses 32-bit relative offsets.  Any `fn_offsets[name] > u32::MAX`
in `artifact_info_pe` produces a human-readable error rather than silent
truncation — a >4 GiB code section is unrealistic but we'd rather refuse
than mislead the debugger.

## Roadmap

| Version | Scope |
|---------|-------|
| v0.1.0 (AOT-DBG-01) | bridge crate + tests (this PR). |
| v0.2.0 (AOT-DBG-02) | wire `embed_iir_debug_info` into `twig-aot::compile_module_to_*_executable`. |
| v0.3.0 (AOT-DBG-03) | `lang-aot --debug` CLI flag. |
| v0.4.0 (AOT-DBG-04) | scripted end-to-end test: compile, run under gdb, assert breakpoint hits the right source line. |
| (later) | DWARF variable bindings (use `DebugSidecarWriter::declare_variable`), `.debug_frame` CFI for unwinding. |

## Why a new crate (vs. expanding `native-debug-info`)

`native-debug-info` is intentionally IIR-agnostic — it deals in raw
sidecar bytes + binary metadata.  Pushing IIR knowledge into it would
flip the layering (lower-level crate dependent on higher-level type).
The bridge crate keeps the abstraction boundary clean: `IIRModule` →
sidecar → DWARF/CodeView.
