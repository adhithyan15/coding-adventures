# dsp-conv

**DSP04 Phase 1+2** — scalar reference same-size 1-D
convolution for the DSP layer.

`dsp-conv` complements `dsp-filters::fir` (which produces the
full `N + K - 1` linear-convolution output) by providing the
**same-size** `N`-length output that image processing and
filter chains usually want, plus explicit control over the
boundary extension at the edges.

```rust
use dsp_conv::{conv1d, BoundaryMode};

let signal = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
let kernel = vec![0.25_f32, 0.5, 0.25];  // 3-tap low-pass

// Same-size output (length 5) with zero-padded boundaries.
let out = conv1d(&signal, &kernel, BoundaryMode::Zero).unwrap();
assert_eq!(out.len(), signal.len());
```

## Algorithm

For each output index `i ∈ 0..N`:

```text
    out[i] = Σ_{k=0..K-1}  kernel[k] · signal_ext[i + (centre - k)]
```

where:

- `centre = K / 2` (integer division — for even `K`, picks the
  upper centre)
- `signal_ext[j]` is the boundary-extended signal:

| Mode        | Extension                                              |
| ----------- | ------------------------------------------------------ |
| `Zero`      | `0` outside `[0, N)`                                   |
| `Replicate` | clamp index to `[0, N - 1]`                            |
| `Reflect`   | mirror about the boundary                              |
| `Wrap`      | modular: `signal_ext[j] = signal[j mod N]` (signed)    |

`O(N · K)` time, `O(N)` memory.

## Boundary modes

| Mode        | scipy.ndimage equivalent | When to use                              |
| ----------- | ------------------------ | ---------------------------------------- |
| `Zero`      | `mode='constant'`        | Signals with zero context outside.       |
| `Replicate` | `mode='nearest'`         | Image edges (avoids dark fringes).       |
| `Reflect`   | `mode='reflect'`         | Image denoising (smoothest extension).   |
| `Wrap`      | `mode='wrap'`            | Periodic signals, textures.              |

## Public API

```rust
pub enum BoundaryMode { Zero, Replicate, Reflect, Wrap }

pub fn conv1d(signal: &[f32], kernel: &[f32], mode: BoundaryMode)
    -> Result<Vec<f32>, ConvError>;

pub enum ConvError {
    EmptySignal,
    EmptyKernel,
    ImageSizeMismatch(String),  // for conv2d in Phase 3
    KernelTooLarge(String),      // for conv2d in Phase 3
}
```

## Phase scope

| Phase  | Lands                                                | Status |
| ------ | ---------------------------------------------------- | ------ |
| 0      | Spec (`code/specs/DSP04-convolution.md`)             | landed |
| **1+2** | **Crate skeleton + scalar `conv1d` with 4 boundary modes** | **this PR (0.1.0)** |
| 3      | Scalar `conv2d` for row-major `[H, W]` images        | pending |
| 4      | `sep_conv2d` for separable kernels                   | pending |
| 5      | Image filter design helpers (Gaussian / Sobel / box / Laplacian / sharpen) | pending |
| 6      | Matrix-ir-lowered `conv1d` / `conv2d`                | pending |

## Tests

`cargo test -p dsp-conv` exercises:

- Error paths: empty signal, empty kernel.
- Closed-form: identity kernel `[1.0]` is identity; centred
  delta kernel preserves the signal.
- Output length contract: `output.len() == signal.len()`.
- All 4 boundary modes verified with handwritten expected
  outputs.
- `conv1d(_, _, Zero)` matches the centre slice of
  `dsp_filters::fir(_, _)`.
