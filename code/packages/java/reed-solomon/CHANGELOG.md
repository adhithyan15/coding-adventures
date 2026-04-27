# Changelog — reed-solomon (Java)

## 0.1.0 — 2026-04-24

### Added

- `ReedSolomon.java` — main class with encoding and decoding.
  - `buildGenerator(nCheck)` — builds the RS generator polynomial over GF(256).
  - `encode(message, nCheck)` — systematic encoding: `message || checkBytes`.
  - `syndromes(received, nCheck)` — evaluates the codeword at each generator root.
  - `errorLocator(synds)` — exposes the Berlekamp-Massey output for diagnostics.
  - `decode(received, nCheck)` — full 5-step decoder pipeline:
    syndromes → Berlekamp-Massey → Chien search → Forney → correction.
- `RsTooManyErrorsException.java` — thrown when more than `t` errors are present.
- `RsInvalidInputException.java` — thrown for malformed parameters.
- Full JUnit Jupiter test suite (`ReedSolomonTest.java`) covering:
  - Generator polynomial: structure (monic, correct length), roots, error cases.
  - Encoding: systematic structure, length constraints, error cases.
  - Syndromes: zero for valid codeword, non-zero for corrupted.
  - Decoding: no errors, single error, two errors, at capacity (t=4), random fuzz test.
  - Too-many-errors path: t+1 errors must throw, not return wrong data.
  - QR-like standard vector: 19-byte message, nCheck=8, up to 4 errors corrected.
- `BUILD` and `BUILD_windows` scripts for the monorepo build tool.
- Composite builds for gf256 and polynomial dependencies.
