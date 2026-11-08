# Changelog

All notable changes to `@coding-adventures/fuzz` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-08

### Added
- Standalone extraction of the TypeScript Level 2 Coverage-Guided Fuzzing engine (VER06).
- Zero runtime dependencies (`"dependencies": {}`).
- Deterministic mutator algebra: `bitFlip`, `byteFlip`, `byteArithmetic`, `boundaryInsert`, `blockDelete`, `blockInsert`, `blockDuplicate`, `splice`, `mutateSequence`.
- 65,536-bit execution coverage bitmap (`CoverageBitmap`) with trace admission, edge recording, and cumulative merging.
- Energy power scheduling via `calculateEnergy`.
- Normalized crash signatures and deduplication via FNV-1a 64-bit hashing (`crashSignature`, `isDuplicateCrash`).
- Multi-phase crash input minimization (`minimizeCrashingInput`).
- Closed failure vocabulary (`FID_FUZZ_CRASH`, `FID_FUZZ_TIMEOUT`, `FID_FUZZ_OOM`, `FID_FUZZ_LEAK`, `FID_FUZZ_PANIC`).
- Privacy-preserving TAP reporting (`formatTapFailure`).
- Full conformance to all 25 canonical test vectors in `code/specs/fixtures/fuzz-v1/cases.json`.
