# Changelog — dsp-filters

## 0.1.0 — 2026-05-15

### Added — DSP03 Phase 1 + 2 (crate skeleton + scalar FIR direct convolution)

Initial release.  The pure-Rust scalar oracle that Phase 3
(FIR via FFT) and Phase 6 (matrix-ir lowered) will test against.

#### Public API

```rust
pub fn fir(signal: &[f32], kernel: &[f32])
    -> Result<Vec<f32>, FilterError>;

pub enum FilterError {
    EmptySignal,
    EmptyKernel,
    InvalidCoefficient(String),
    Fft(String),
}
```

#### Algorithm

Direct linear convolution:

```text
    y[n] = Σ_{k=0..K-1}  kernel[k] · signal[n - k]
```

Output length is `N + K - 1` (the full convolution, matching
`numpy.convolve(signal, kernel, mode='full')`).  Boundary
handling: input is implicitly zero-padded outside `[0, N)`.
`O(N · K)` time, `O(N + K)` memory.

#### Tests (10)

Error paths:
- `fir_rejects_empty_signal`
- `fir_rejects_empty_kernel`

Closed-form known vectors:
- `fir_with_identity_kernel_returns_signal` — `[1.0]` is identity.
- `fir_with_delay_kernel_shifts_by_one` — `[0.0, 1.0, 0.0]` shifts.
- `fir_with_uniform_kernel_preserves_total_sum`
- `fir_with_box_kernel_3tap` — sums neighborhood of three.

Length contract:
- `fir_output_length_is_n_plus_k_minus_1` for several N, K.

Naive cross-check:
- `fir_matches_naive_reference_n5_k3`
- `fir_matches_naive_reference_n8_k4`
- `fir_matches_naive_reference_n100_k15`

#### Dependencies

- `dsp-fft` — declared but not used in Phase 2.  Reserved for
  Phase 3 (`fir_fft` overlap-add) and Phase 6 (matrix-ir
  graph builder).

No FFI, no `unsafe`, no external crates.

#### What this phase does NOT include

- Phase 3: `fir_fft` (FFT-based overlap-add for long kernels).
- Phase 4: `iir` (direct-form-II Transposed).
- Phase 5: filter design helpers (Butterworth, Chebyshev,
  windowed-sinc).
- Phase 6: matrix-ir-lowered `fir_via_runtime`.
- 2-D filters (`filter2D`).  Lives in DSP04.
