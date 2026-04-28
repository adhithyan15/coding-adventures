# Changelog — coding_adventures_reed_solomon

## 0.1.0 — 2026-04-24

### Added

- Initial release: systematic RS encoding and decoding over GF(256), per spec MA02.
- `rsBuildGenerator` — build the RS generator polynomial g(x) = (x+α¹)...(x+α^{nCheck}).
- `rsEncode` — systematic encoding: `message || check_bytes` via polynomial mod.
- `rsSyndromes` — compute syndrome values S_j = received(α^j) for j=1..nCheck.
- `rsDecode` — full decoding pipeline:
    1. Syndrome computation
    2. Berlekamp-Massey → error locator Λ(x)
    3. Chien search → error positions
    4. Forney's algorithm → error magnitudes
    5. XOR correction
- `rsErrorLocator` — expose Berlekamp-Massey for advanced callers.
- `TooManyErrorsException` — thrown when > t errors detected.
- `InvalidInputException` — thrown for invalid parameters (odd nCheck, length > 255, etc.).
- 44 unit tests covering: generator properties, encode properties, syndromes,
  round-trip (no errors), error correction up to capacity, failure beyond capacity,
  and spec property verification.
