# Changelog

All notable changes to `swf-container` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Validated `ZwsConfig` bounds (`lc <= 8`, `lp <= 4`, `pb <= 4`, `lc + lp <= 12`) and used `rep0.checked_add(1)` in `lzma_decompress_bounded` (CWE-191 / CWE-125 / CWE-400).
- Enforced `max_uncompressed_bytes` and `max_ratio` decompression bomb checks on every output byte including `ShortRep` in `lzma_decompress_bounded` (CWE-409).
- Added `tests/hostile_inputs.rs` (`#![forbid(unsafe_code)]`) covering crafted `ZwsConfig`, LZMA `ShortRep` bomb, malformed zlib blocks, truncated/negative long tags, and a 1,000-case hostile byte fuzz campaign.

## [0.1.0] - 2026-11-16

### Added
- Initial implementation of `SWF01: swf-container`.
- Safe SWF header parser supporting `FWS` (uncompressed), `CWS` (RFC 1950/1951 ZLIB), and `ZWS` (LZMA1) container signatures.
- Anti-bomb decompression security controls: 256 MB default allocation cap and 50:1 expansion ratio ceiling beyond 1 MB (`SwfError::DecompressionBombDetected`).
- Zero-allocation MSB-first `BitReader` supporting `read_ub`, `read_sb`, `read_fb`, `SwfRect` (twips/pixels), `SwfMatrix` (affine transform), and `SwfColorTransform` (`CXFORM`/`CXFORMWITHALPHA`).
- Zero-allocation streaming `SwfTagIter` over uncompressed tag streams with short and long tag framing.
- Full `SwfTagKind` vocabulary with integer tag code mapping.
- Bounded RFC 1950/1951 zlib decompressor and bounded LZMA decompressor with Adler-32 verification.
- Conformance test suite (`tests/conformance.rs`), zero-allocation microbenchmarks with `CountingAllocator` (`tests/microbenchmarks.rs`), and runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification and security assurance files: `swf-container.warrant`, `required_capabilities.json`, `assurance-profile.json`, `effect_certificate.json`, and 26-CWE `assurance_report.json`.
