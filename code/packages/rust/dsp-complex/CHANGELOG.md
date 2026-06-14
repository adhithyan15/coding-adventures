# Changelog — dsp-complex

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
