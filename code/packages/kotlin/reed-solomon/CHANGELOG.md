# Changelog — reed-solomon (Kotlin)

## [0.1.0] — 2026-04-24

### Added

- `buildGenerator(nCheck)` — builds the monic RS generator polynomial `g(x) = (x+α)(x+α²)…(x+α^nCheck)` in little-endian form
- `encode(message, nCheck)` — systematic RS encoding via `M(x)·x^{nCheck} mod g(x)`; output is `message || parity`
- `syndromes(received, nCheck)` — evaluates codeword at α^1…α^{nCheck} (big-endian Horner)
- `decode(received, nCheck)` — full Berlekamp-Massey / Chien / Forney pipeline; corrects up to `t = nCheck/2` errors
- `errorLocator(synds)` — exposes Berlekamp-Massey result for diagnostics/QR decoders
- `TooManyErrorsException` — thrown when codeword has more than `t` errors
- `InvalidInputException` — thrown for bad parameters (nCheck=0/odd, total length >255, etc.)
- `VERSION = "0.1.0"` constant
- 35 unit tests covering: generator construction, syndrome computation, encoding/decoding with 0/1/2/4 errors,
  single-byte round-trips for all 256 values, too-many-errors case, parameter validation

### Implementation notes

- Uses b=1 convention: generator roots are α^1, α^2, …, α^{nCheck} (α=2)
- Codeword bytes treated as big-endian polynomial: `codeword[0]·x^{n-1} + … + codeword[n-1]`
- `polynomial` package dependency is declared but internal helpers use direct GF256 operations
  for big-endian byte arrays (avoiding LE/BE conversion overhead in the hot path)
- Dependencies on `gf256` and `polynomial` resolved via Gradle composite builds (`includeBuild`)
- BUILD uses a shared mutex lock (`kotlin-foundation.lock`) to prevent CI race conditions
  when building alongside the `polynomial` package which includes the same `gf256` composite build
