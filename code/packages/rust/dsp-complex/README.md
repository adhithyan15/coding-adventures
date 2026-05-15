# dsp-complex

**DSP01 Phase 2** — complex-number helper for the DSP layer.

Provides [`ComplexTensor`](src/lib.rs), the type the
[DSP00](../../../specs/DSP00-signal-processing-overview.md) spec
calls for.  V1 storage is interleaved `[re, im, re, im, …]` in a
`Vec<f32>`, matching the DSP00 complex-number convention.

```rust
use dsp_complex::ComplexTensor;

let signal = ComplexTensor::from_real(&[1.0, 0.0, -1.0, 0.0]);
let mag    = signal.magnitude();          // [1.0, 0.0, 1.0, 0.0]
let phase  = signal.phase();              // angles in radians
let conj   = signal.conjugate();          // imag → -imag
```

## V1 scope

- Constructors: `from_real`, `from_real_imag`, `from_interleaved`.
- Accessors: `real`, `imag`, `magnitude`, `phase`, `conjugate`.
- Matrix-IR shape / dtype helpers: `matrix_shape`, `matrix_dtype`.

## Out of scope (V1)

- **Graph-builder accessors.**  Phase 3 of DSP01 will introduce a
  matrix-ir-backed `ComplexTensor` variant whose `real()` /
  `imag()` / `magnitude()` / `phase()` return `Tensor` handles
  built by emitting Slice/Mul/Sqrt/Atan2 ops.  That depends on a
  Slice op being added to `matrix-ir`, which lands at the same
  time as Phase 3.  The public method names will not change.
- **Full Complex32 algebra.**  No `std::ops::*` impls,
  transcendentals, or trigonometric helpers.  When user code
  needs complex arithmetic it operates on the interleaved layout
  directly.

## Roadmap

| Phase | Lands                                                | Status |
| ----- | ---------------------------------------------------- | ------ |
| 2     | Scalar `ComplexTensor` (this crate)                   | **this PR** |
| 3     | Matrix-IR-backed variant + Slice op for matrix-ir     | pending |

## Tests

`cargo test -p dsp-complex` — 11 unit tests covering constructors,
accessors, error paths, and the Debug truncation.
