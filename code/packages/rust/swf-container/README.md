# swf-container

Deterministic, zero-dependency binary SWF container reader, anti-bomb decompressor, and streaming tag parser for Rust.

Part of the **Coding Adventures** Flash Player suite (`SWF01`, see `code/specs/SWF00-flash-runtime-roadmap.md`).

---

## Overview

`swf-container` implements safe, bounded parsing and decompression of Adobe Shockwave Flash (`.swf`) files:

1. **Header Parsing**:
   - Multi-format signatures: `FWS` (uncompressed), `CWS` (RFC 1950/1951 ZLIB), and `ZWS` (LZMA1, version $\ge 13$).
   - Header parameters: Version byte ($1 \le \text{version} \le 50$), untrusted 32-bit `FileLength` hint, `RECT` stage bounds in twips ($1\text{ twip} = \frac{1}{20}\text{ pixel}$), 8.8 fixed-point frame rate, and frame count.
2. **Anti-Bomb & Security Defenses**:
   - Untrusted `FileLength` handling with bounded progressive allocation.
   - Hard allocation cap: `max_uncompressed_bytes` defaults to 256 MB (configurable).
   - Expansion ratio enforcement: streaming decompression strictly bounds $\text{bytes}_{\text{out}} / \text{bytes}_{\text{in}} \le 50:1$ once output exceeds 1 MB; returns `SwfError::DecompressionBombDetected` on violation.
   - Bounded sub-slices: tag payloads and bitstream readers operate on bounded byte slices preventing buffer over-reads.
3. **Bitstream Primitives (`BitReader`)**:
   - Zero-allocation MSB-first bit reader supporting unsigned/signed integers (`read_ub`, `read_sb`), 16.16 fixed-point numbers (`read_fb`), stage dimensions (`SwfRect`), 2D affine transforms (`SwfMatrix`), and color transforms (`SwfColorTransform` / `CXFORM` / `CXFORMWITHALPHA`).
4. **Streaming Tag Iterator (`SwfTagIter`)**:
   - Zero-allocation streaming iterator over uncompressed SWF body tags.
   - Handles short (< 63 bytes) and long (`0x3F` + 32-bit length) headers.
   - Complete `SwfTagKind` vocabulary covering definition, control, script, display, and sound tags.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![deny(unsafe_code)]`.
- **Zero Allocation Hot Paths (`VER07`):** `BitReader` and `SwfTagIter` perform zero heap allocations, verified with `CountingAllocator` in `tests/microbenchmarks.rs`.
- **CWE Weakness Closures (`VER15`):** Complete 26-CWE matrix in `assurance_report.json` with machine-checked defenses against CWE-20, CWE-125, CWE-190, CWE-400, CWE-409, and CWE-787.

---

## Usage

```rust
use swf_container::{SwfDecompressorConfig, SwfMovie, SwfTagKind};

let config = SwfDecompressorConfig::default_decompressor_config();
let movie = SwfMovie::from_swf_bytes(&swf_bytes, &config)?;

println!("SWF Version: {}", movie.movie_header().header_version());
println!("Stage size: {}x{} px",
    movie.movie_header().header_stage_rect().rect_width_pixels(),
    movie.movie_header().header_stage_rect().rect_height_pixels(),
);
println!("Frame rate: {} fps", movie.movie_header().header_frame_rate());

for tag in movie.movie_tags() {
    let tag = tag?;
    println!("Tag {:?} (length: {} bytes)", tag.header.kind, tag.header.length);
}
```
