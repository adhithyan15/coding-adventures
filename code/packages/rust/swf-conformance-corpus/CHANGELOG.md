# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Rejected embedded NUL (`\0`) bytes in `Avm1ActionBuilder::avm1_push_str`, `avm1_define_function`, and `avm1_define_function2` (`s`, `name`, `params`) and enforced `MAX_CORPUS_SWF_BYTES` atomically before mutating `self.bytes` (CWE-20).
- Made `SwfBitWriter` (`bit_writer_write_ub`, `bit_writer_write_rect`, `bit_writer_write_matrix`, `bit_writer_write_cxform_alpha`) atomic on `Err` via upfront capacity validation and snapshot rollback.
- Added `tests/hostile_inputs.rs` (`#![forbid(unsafe_code)]`) covering `NaN`/`±Inf`/`1e20` `SwfMatrix` & `fb` values, embedded `\0` across builders, `SwfBitWriter` atomic rollback at `MAX_CORPUS_SWF_BYTES`, malformed digest hex strings, hostile `FlashPlayerConfig`, and a seeded 1,000-case builder/bitwriter fuzz campaign.

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `swf-conformance-corpus` (`SWF08`).
- Clean-room bounded synthetic SWF binary encoder (`SwfBitWriter` with `MAX_BIT_WIDTH = 31` + `SwfBuilder` supporting `FWS`, `CWS` via `deflate`, `RECT`, 8.8 FPS, short/long tag framing, `DefineShape1..4` straight/curved quadratic Bézier + linear/radial gradients + hole cutouts, `DefineSprite`, `PlaceObject2`/`PlaceObject3`, `RemoveObject2`, `DoAction` AVM1, `DoABC` AVM2, `DefineSound`/`StartSound`/`SoundStreamHead2`/`SoundStreamBlock` with 4-bit ADPCM, `ShowFrame`, and `End`).
- All 6 mandatory positive conformance corpus cases (`corpus-01-shapes` through `corpus-06-streaming-audio`) with pinned canonical SHA-256 scene and PCM audio frame digests.
- Adversarial security corpus covering CWE-409, CWE-400, CWE-125, CWE-1321, CWE-78, and CWE-94.
- Conformance, `CountingAllocator` zero-alloc microbenchmark, and runtime capability test suites.
