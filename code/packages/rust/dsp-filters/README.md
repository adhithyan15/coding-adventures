# dsp-filters

Scalar reference FIR + IIR filters and checked Biquad/SOS primitives for the
DSP layer, plus filter design helpers.

As of **0.4.0 (DSP03 Phase 7)** this crate ships:

- **FIR** (`fir(signal, kernel)`) via direct linear convolution.
- **IIR** (`iir(signal, b, a)`) via direct-form-II Transposed
  — matches `scipy.signal.lfilter(b, a, x)` exactly.
- **Design helpers** (`design_low_pass`, `design_high_pass`,
  `butterworth_lowpass`) — windowed-sinc FIR and Butterworth
  IIR.
- **Checked Biquad/SOS** (`Biquad`, `sosfilt`) — stateful `f32` DFII-T
  processing and one-shot second-order-section cascades.

```rust
use dsp_filters::{fir, iir, design_low_pass, butterworth_lowpass, sosfilt, Biquad, WindowType};

let signal: Vec<f32> = (0..200).map(|i| ((i as f32) * 0.1).sin()).collect();

// 33-tap windowed-sinc low-pass with Hamming window, cutoff at 0.2·Nyquist.
let kernel = design_low_pass(0.2, 33, WindowType::Hamming);
let smoothed_fir = fir(&signal, &kernel).unwrap();

// 2nd-order Butterworth low-pass, cutoff at 0.1·Nyquist.
let (b, a) = butterworth_lowpass(2, 0.1);
let smoothed_iir = iir(&signal, &b, &a).unwrap();

// Checked single-precision section; state continues across blocks.
let coefficients = [0.5, 0.25, 0.5, 2.0, -2.0, 0.5];
let mut section = Biquad::new(coefficients)?;
let first = section.process(&signal[..100])?;
let second = section.process(&signal[100..])?;
section.reset();

// One-shot cascade: each section starts at zero; no streaming state is exposed.
let cascade = sosfilt(&[coefficients, [1.0, -1.0, 0.0, 1.0, 0.0, 0.0]], &signal)?;
let _ = (first, second, cascade);
Ok::<(), dsp_filters::FilterError>(())
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

pub struct Biquad; // constructor: new([b0, b1, b2, a0, a1, a2])
impl Biquad {
    pub fn new(coefficients: [f32; 6]) -> Result<Self, FilterError>;
    pub fn process_sample(&mut self, input: f32) -> Result<f32, FilterError>;
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, FilterError>;
    pub fn reset(&mut self);
}
pub fn sosfilt(sections: &[[f32; 6]], signal: &[f32])
    -> Result<Vec<f32>, FilterError>;

pub enum FilterError {
    EmptySignal,
    EmptyKernel,
    InvalidCoefficient(String),  // coefficients, non-finite values, or caps
    Fft(String),                  // for FIR-via-FFT (Phase 3)
}
```

`Biquad::new` accepts six raw `f32` coefficients in `[b0, b1, b2, a0, a1, a2]`
order. It requires finite raw coefficients and finite, nonzero `a0`, divides
five coefficients directly by `a0`, and rejects non-finite normalized values.
Processing accepts finite samples and rejects non-finite output or candidate
state delays. A sample commits delays only on `Ok`; a block commits state only
if every sample succeeds. `reset` clears both delays. `sosfilt` is a one-shot
cascade: every section starts at zero state, and no state is returned to the
caller. Empty signals return `FilterError::EmptySignal`; block/signal length is
capped at `MAX_SAMPLES` (2^24), and cascades at `MAX_ORDER` (1024 sections).
These checked APIs use single-precision arithmetic. The shared fixture verifies
selected exact cases; it does not establish general Python/TypeScript bit or
error parity, pole stability, native floating-point proofs, or Warrant/SAT
verification.

## Phase scope

| Phase  | Lands                                                | Status |
| ------ | ---------------------------------------------------- | ------ |
| 0      | Spec (`code/specs/DSP03-filters.md`)                 | landed |
| 1+2    | Crate skeleton + scalar FIR direct convolution       | landed (0.1.0) |
| 3      | FIR via FFT (overlap-add) using `dsp-fft`            | deferred |
| 4      | Scalar IIR direct-form-II Transposed                 | landed (0.2.0) |
| 5      | Windowed-sinc + Butterworth design helpers            | landed (0.3.0) |
| 6      | Matrix-ir-lowered FIR                                | pending |
| **7**  | **Checked f32 Biquad and one-shot SOS primitives**    | **this release (0.4.0)** |

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

`cargo test -p dsp-filters` currently runs 43 unit tests, 10 checked Biquad/SOS
integration tests, 1 public design API integration test, and 0 doctests. The
Biquad/SOS integration suite consumes the shared fixed JSON fixture through
crate-root exports, checks all 32,768 nonempty partitions of its 16-sample
input, reset/samplewise behavior, one-shot SOS, caps, and failure atomicity.
It also runs bounded seeded exact-oracle properties and a structured raw-bit
fuzz campaign; these tests are not native/source floating-point proofs.
