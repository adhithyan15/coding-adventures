# dsp-filters

Scalar reference FIR + IIR filters for the DSP layer.

As of **0.2.0 (DSP03 Phase 4)** this crate ships the two
workhorse 1-D filter primitives:

- **FIR** (`fir(signal, kernel)`) via direct linear convolution.
- **IIR** (`iir(signal, b, a)`) via direct-form-II Transposed
  — matches `scipy.signal.lfilter(b, a, x)` exactly.

```rust
use dsp_filters::{fir, iir};

let signal = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];

// FIR: 3-tap low-pass kernel.
let kernel = vec![0.25_f32, 0.5, 0.25];
let smoothed = fir(&signal, &kernel).unwrap();

// IIR: single-pole low-pass.  y[n] = x[n] + 0.9 · y[n-1].
let filtered = iir(&signal, &[1.0], &[1.0, -0.9]).unwrap();
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
| 1+2    | Crate skeleton + scalar FIR direct convolution       | landed (0.1.0) |
| 3      | FIR via FFT (overlap-add) using `dsp-fft`            | deferred |
| **4**  | **Scalar IIR direct-form-II Transposed**             | **this PR (0.2.0)** |
| 5      | Butterworth / Chebyshev / windowed-sinc design helpers | pending |
| 6      | Matrix-ir-lowered FIR                                | pending |

## IIR algorithm (Phase 4)

Direct-Form-II Transposed: one state vector `z` of length
`order = max(len(b), len(a)) - 1`, initialised to zero.  Per
sample `x`:

```text
    y = (b[0] · x + z[0]) / a[0]
    for k in 0..order-1:  z[k] = b[k+1] · x - a[k+1] · y + z[k+1]
    z[order-1] = b[order] · x - a[order] · y
```

The implementation pre-scales `b` and `a` by `1/a[0]` so the
inner loop avoids a divide per sample.

Phase 5 will produce stable `(b, a)` pairs via Butterworth /
Chebyshev / windowed-sinc design helpers.

## Tests

`cargo test -p dsp-filters` — 25 unit tests + 1 doctest:

- 10 from Phase 1+2 (FIR): error paths, identity / delay / box /
  uniform kernels, length contract, naive O(N·K) cross-check.
- 15 from Phase 4 (IIR, this release): error paths (empty, a[0] = 0/NaN/∞),
  identity, pure gain, `a[0]` normalisation, single-pole low-pass
  step response, geometric impulse response, two-pole DC gain
  verification, FIR cross-check (`a = [1.0]`), output length
  contract.
