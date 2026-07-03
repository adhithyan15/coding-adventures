# aot-debug

Bridge between IIR source-location data and `native-debug-info`'s DWARF /
CodeView emitters.  Builds a debug-sidecar blob from an `IIRModule`'s
`source_map`, then hands it to `native-debug-info` to inject debug
sections into AOT-compiled binaries (so `gdb` / `lldb` / WinDbg can
single-step through original source lines).

**Status: v0.1.0 (AOT-DBG-01).**  Sidecar builder + embed orchestrator +
`ArtifactInfo` convenience constructors.  twig-aot wiring follows in
AOT-DBG-02.

## Where it fits

```text
IIRModule ──┐
            │ build_sidecar_from_iir() ─► sidecar bytes ─┐
            │                                            │
            └───────────────────────────►─────────────────┤
                                                          ▼
AOT-compiled binary ────► embed_iir_debug_info() ──► .debug_* augmented binary
 (twig-aot output)                                 (gdb/lldb/WinDbg-friendly)
```

## Why a separate crate?

1. **No twig-aot churn yet.**  v0.1.0 lands the bridge + tests in
   isolation.  twig-aot's binary emitter stays untouched in this round;
   AOT-DBG-02 will pass the function-offset map + source path into a new
   debug-enabled emit path.
2. **Reusable across backends.**  `iir-to-llvm` (LLVM IR → `llc` →
   native) can also call `build_sidecar_from_iir` without duplicating
   IIR → sidecar logic.
3. **Testable in isolation.**  Sidecar correctness checks don't require
   a real ELF/Mach-O/PE on hand — see `tests/bridge.rs`.

## Quick start

```rust
use aot_debug::{build_sidecar_from_iir, embed_iir_debug_info, artifact_info_elf_or_macho};
use std::collections::HashMap;

// 1) IIR side: build the sidecar from your module + source path.
let sidecar_bytes = build_sidecar_from_iir(&module, "hello.bas");

// 2) Binary side: when you also have the linked-binary bytes + function
//    offsets from twig-aot, build an ArtifactInfo and embed:
let mut fn_offsets = HashMap::new();
fn_offsets.insert("main".to_string(), 0usize);
let artifact = artifact_info_elf_or_macho("linux", 0x400000, &fn_offsets, binary.len());
let augmented = embed_iir_debug_info(&binary, &artifact, &module, "hello.bas")?;
```

## Public surface (v0.1.0)

* `pub fn build_sidecar_from_iir(module: &IIRModule, source_path: &str) -> Vec<u8>`
* `pub fn embed_iir_debug_info(binary, artifact, module, source_path) -> Result<Vec<u8>, String>`
* `pub fn artifact_info_elf_or_macho(target, load_addr, fn_offsets, code_size) -> ArtifactInfo`
* `pub fn artifact_info_pe(image_base, fn_offsets, code_rva, code_section_index) -> Result<ArtifactInfo, String>`

## Why skip synthetic `SourceLoc`s?

The contract for `SourceLoc::SYNTHETIC` (`line == 0 && column == 0`) is
"no real source counterpart".  Surfacing those rows to the debugger would
produce a 0-line/0-column entry that gdb/lldb display as `???:???`,
worse than no entry at all.  The bridge filters them out.

## Tests

```sh
cargo test -p aot-debug
```

7 tests at v0.1.0 covering: minimal happy path, synthetic filtering,
`param_count` round-trip, length-mismatch tolerance, both
`ArtifactInfo` constructors (incl. PE u32 overflow rejection).
