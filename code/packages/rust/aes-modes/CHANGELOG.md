# Changelog

## Unreleased

### Added
- A caller-owned fixed-memory GHASH boundary and GF(2^128) multiplication
  primitive with fixed loop schedules and masked secret-derived decisions.
- Wipe-on-drop ownership for GHASH products, multipliers, accumulators, and
  padded blocks, plus NIST and block-edge regression evidence.

### Security
- Retained the strict optimized-IR branch gate after LLVM transformed a known
  zero/all-ones mask into secret-dependent control flow. The mask now uses
  wipe-on-drop storage behind a best-effort optimizer barrier; compiler/target
  IR evidence remains required and is not a general constant-time guarantee.
- GHASH length validation now occurs before the fixed boundary changes caller
  output. This primitive is not a combined AES-GCM, TLS, HTTP, or OAuth
  transport boundary.

### Fixed

- Fixed the optimized-IR gate (`tests/optimized_ir.rs`) on Rust/Cargo 1.100:
  Cargo's new build-dir layout emits `--emit=llvm-ir` output under
  `release/build/<crate>/<hash>/out/` instead of `release/deps/`, so the gate
  failed with "release dependency directory must exist" before inspecting any
  IR. The gate now locates the `.ll` file by searching the release tree. No
  assertion was changed or weakened; the IR shape checks pass unmodified on
  both 1.97 and 1.100.


## 0.1.0 — 2026-04-12

### Added
- ECB mode encryption/decryption (INSECURE — educational only)
- CBC mode encryption/decryption with PKCS#7 padding
- CTR mode encryption/decryption (stream cipher, no padding)
- GCM mode authenticated encryption/decryption with GHASH
- PKCS#7 padding and unpadding utilities
- GF(2^128) multiplication for GHASH
- Comprehensive tests using NIST SP 800-38A and GCM test vectors
- Literate programming style with educational explanations
