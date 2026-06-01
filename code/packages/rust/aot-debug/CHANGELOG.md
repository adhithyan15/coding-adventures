# Changelog — aot-debug

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-06-01 (AOT-DBG-01 — IIR ↔ native-debug-info bridge)

### Added — sidecar builder + embed orchestrator + ArtifactInfo helpers

First release.  Implements item AOT-DBG-01 of the
[multi-language backend plan][plan].  Provides the bridge layer that
connects two existing pieces:

* `IIRModule.source_map: Vec<SourceLoc>` (populated by every front-end
  per tasks #33 / #34 / #35), and
* `native-debug-info`'s `embed_debug_info` (DWARF 4 + CodeView 4
  embedding into ELF / Mach-O / PE binaries).

#### Public surface

```rust
pub fn build_sidecar_from_iir(
    module: &IIRModule,
    source_file_path: &str,
) -> Vec<u8>;

pub fn embed_iir_debug_info(
    binary: &[u8],
    artifact: &ArtifactInfo,
    module: &IIRModule,
    source_file_path: &str,
) -> Result<Vec<u8>, String>;

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

#### Design choices

* **Synthetic `SourceLoc`s are filtered.**  `SourceLoc::SYNTHETIC` means
  "no real source counterpart" — emitting those rows would produce
  `???:???` entries in gdb/lldb that are strictly worse than no entry.
* **Length-mismatch tolerance.**  If a front-end produces a `source_map`
  shorter (or longer) than `instructions`, we walk the shorter prefix
  instead of panicking.  A future invariant check could harden this; for
  now it lets buggy front-ends fail gracefully.
* **`artifact_info_pe` rejects oversized offsets.**  CodeView uses 32-bit
  relative offsets.  Any `fn_offsets[name] > u32::MAX` produces a
  human-readable error rather than a silent truncation.

#### What this release does NOT do

* Does NOT modify `twig-aot`'s `compile_module_to_*_executable` paths.
  That wiring (AOT-DBG-02) follows in a separate PR — splitting keeps
  the binary-emitter changes isolated for review.
* Does NOT compute function byte-offsets — those come from the AOT
  backend's link step (twig-aot already returns them internally).
* Does NOT emit `.debug_frame` CFI for unwinding yet — `native-debug-info`'s
  DWARF emitter already handles that when the sidecar has the right
  data, but we don't synthesize frame metadata from IIR here.

#### Tests added (7 total)

* `builds_sidecar_with_one_function_and_one_real_location`
* `synthetic_locs_are_skipped`
* `param_count_propagates_to_function_entry`
* `tolerates_shorter_source_map`
* `artifact_info_elf_or_macho_smoke`
* `artifact_info_pe_happy`
* `artifact_info_pe_rejects_oversized_offset`

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md
