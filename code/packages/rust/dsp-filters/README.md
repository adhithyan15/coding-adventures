# dsp-filters

**DSP03 Phase 1+2** — scalar reference FIR (finite impulse
response) filter for the DSP layer.

FIR filters are the workhorse of DSP: image blurring,
sharpening, edge detection, audio EQ pre-shaping, sensor
smoothing — all of these reduce to convolution against a
small real-valued kernel.

```rust
use dsp_filters::fir;

let signal = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
let kernel = vec![0.25_f32, 0.5, 0.25];   // 3-tap low-pass
let smoothed = fir(&signal, &kernel).unwrap();
// smoothed.len() == 5 + 3 - 1 == 7  (full linear convolution)
```

## Algorithm

Direct linear convolution:

```text
    y[n] = Σ_{k=0..K-1}  kernel[k] · signal[n - k]
```

Output length is `N + K - 1` (the full convolution, no
truncation).  `O(N · K)` time, `O(N + K)` memory.  For long
kernels (`K > ~64`), Phase 3 will add an FFT-based overlap-add
implementation via `dsp-fft` that's `O((N + K) · log(N + K))`.

Boundary handling: the input is implicitly zero-padded outside
`[0, N)`.  This matches `numpy.convolve(signal, kernel,
mode='full')` and `scipy.signal.convolve(signal, kernel,
mode='full')`.

## Public API

```rust
pub fn fir(signal: &[f32], kernel: &[f32])
    -> Result<Vec<f32>, FilterError>;

pub enum FilterError {
    EmptySignal,
    EmptyKernel,
    InvalidCoefficient(String),  // for IIR (Phase 4)
    Fft(String),                  // for FIR-via-FFT (Phase 3)
}
```

## Phase scope

| Phase  | Lands                                                | Status |
| ------ | ---------------------------------------------------- | ------ |
| 0      | Spec (`code/specs/DSP03-filters.md`)                 | landed |
| **1+2** | **Crate skeleton + scalar FIR direct convolution** | **this PR (0.1.0)** |
| 3      | FIR via FFT (overlap-add) using `dsp-fft`            | pending |
| 4      | Scalar IIR direct-form-II Transposed                 | pending |
| 5      | Butterworth / Chebyshev / windowed-sinc design helpers | pending |
| 6      | Matrix-ir-lowered FIR                                | pending |

## Tests

`cargo test -p dsp-filters` exercises:

- Error paths: empty signal, empty kernel.
- Closed-form: identity kernel `[1.0]`, delay-by-1 kernel
  `[0.0, 1.0, 0.0]`, uniform K-tap moving average.
- Naive O(N·K) cross-check for several `(N, K)` combinations.
- Length contract: `output.len() == signal.len() + kernel.len() - 1`.
