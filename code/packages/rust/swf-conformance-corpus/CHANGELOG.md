# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `swf-conformance-corpus` (`SWF08`).
- Clean-room bounded synthetic SWF binary encoder (`SwfBitWriter` with `MAX_BIT_WIDTH = 31` + `SwfBuilder` supporting `FWS`, `CWS` via `deflate`, `RECT`, 8.8 FPS, short/long tag framing, `DefineShape1..4` straight/curved quadratic Bézier + linear/radial gradients + hole cutouts, `DefineSprite`, `PlaceObject2`/`PlaceObject3`, `RemoveObject2`, `DoAction` AVM1, `DoABC` AVM2, `DefineSound`/`StartSound`/`SoundStreamHead2`/`SoundStreamBlock` with 4-bit ADPCM, `ShowFrame`, and `End`).
- All 6 mandatory positive conformance corpus cases (`corpus-01-shapes` through `corpus-06-streaming-audio`) with pinned canonical SHA-256 scene and PCM audio frame digests.
- Adversarial security corpus covering CWE-409, CWE-400, CWE-125, CWE-1321, CWE-78, and CWE-94.
- Conformance, `CountingAllocator` zero-alloc microbenchmark, and runtime capability test suites.
