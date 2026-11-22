# Changelog — dsp-complex

## Unreleased

### Fixed & Hardened — VER15 Deep Audit

- Replaced `(re*re + im*im).sqrt()` in `ComplexTensor::magnitude` with `re.hypot(im)` to prevent spurious `Inf` overflow on large finite magnitudes (e.g. `1e20`) and underflow on subnormal pairs (**CWE-20**).
- Added explicit `checked_mul(2)` / `u32::try_from` bounds in buffer capacity and `matrix_shape` conversions (**CWE-190**).
- Added dedicated integration test suites `tests/test_hostile_inputs.rs` and `tests/test_pbt.rs` (driven by `coding_adventures_test_bootstrap::prng::SplitMix64`) covering `NaN`/`±Inf`, subnormals, overflow-safe magnitude, `MAX_SAMPLES` cap enforcement, and polar/conjugation invariants.

## 0.2.0 — 2026-11-15

### Changed — Security Hardening and CWE Mitigations

- Enforced `#![forbid(unsafe_code)]` unconditionally.
- Added `MAX_SAMPLES = 1 << 24` resource bound (CWE-400 / CWE-190).
- Added `try_from_real` checked constructor with clean `Result::Err` handling.
- Added finite-input checks (`!is_finite()` rejected cleanly with `ComplexError::InvalidInput`) across all constructors, preventing arithmetic poisoning and NaN propagation.
- Added Knuth-style literate comments detailing algorithmic structure and invariants.
- Added property-based testing suite driven by `coding_adventures_test_bootstrap::prng::SplitMix64` and hostile tests for `NaN`, `Inf`, and oversized lengths.
- Added `dsp-complex.warrant` contract specification.

## 0.1.0 — 2026-05-13

### Added — DSP01 Phase 2 (scalar `ComplexTensor`)

Initial release.  First concrete piece of the DSP layer (built on
top of the matrix execution layer MX01–MX06).

- `pub struct ComplexTensor` — interleaved `[re, im]` f32 layout
  matching DSP00's complex-number convention.
- Constructors: `from_real`, `from_real_imag`, `from_interleaved`.
  All return typed `ComplexError`s on length mismatches or
  malformed input; no panics.
- Accessors: `real`, `imag`, `magnitude`, `phase`, `conjugate`,
  `len`, `is_empty`, `as_interleaved`, `into_interleaved`.
- Matrix-IR interop: `matrix_shape()` returns
  `Shape::from(&[N, 2])`; `matrix_dtype()` returns `DType::F32`.
- `Debug` impl truncates large signals so test logs stay readable.

### Tests

11 unit tests covering constructors, accessors, error paths, known
values for magnitude / phase / conjugate, and the Debug truncation.

### What this crate does NOT do (V1)

- No graph-builder accessors.  Phase 3 of DSP01 will introduce a
  matrix-ir-backed `ComplexTensor` variant whose accessors return
  `Tensor` handles built by emitting Slice / Mul / Sqrt / Atan2
  ops.  Requires a Slice op being added to `matrix-ir` first.
- No std::ops::* impls or transcendentals.  Add a separate
  `dsp-complex-math` crate if those are needed.

### Dependencies

- `matrix-ir` — for `Shape` / `DType` used by the interop helpers.

No `unsafe`, no FFI.
