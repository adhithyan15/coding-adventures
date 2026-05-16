# dsp-conv

Scalar reference 1-D and 2-D convolution for the DSP layer.

As of **0.2.0 (DSP04 Phase 3)** this crate ships:

- **1-D** (`conv1d`) — same-size signal convolution.
- **2-D** (`conv2d`) — same-size image convolution on
  row-major `[H, W]` real `f32` buffers.

Complements `dsp-filters::fir` (which produces the full
`N + K - 1` linear-convolution output) by providing the
**same-size** output that image processing and filter chains
usually want, plus explicit control over the boundary
extension at the edges (`Zero` / `Replicate` / `Reflect` /
`Wrap`).

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
| 1+2    | Crate skeleton + scalar `conv1d` with 4 boundary modes | landed (0.1.0) |
| **3**  | **Scalar `conv2d` for row-major `[H, W]` images**    | **this PR (0.2.0)** |
| 4      | `sep_conv2d` for separable kernels                   | pending |
| 5      | Image filter design helpers (Gaussian / Sobel / box / Laplacian / sharpen) | pending |
| 6      | Matrix-ir-lowered `conv1d` / `conv2d`                | pending |

## 2-D conv example (Phase 3)

```rust
use dsp_conv::{conv2d, BoundaryMode};

// 3×3 box-blur kernel (normalised).
let kernel = vec![1.0_f32 / 9.0; 9];

// 5×5 row-major image.
let image: Vec<f32> = (0..25).map(|i| i as f32).collect();

let blurred = conv2d(&image, &kernel, 5, 5, 3, 3, BoundaryMode::Replicate)
    .unwrap();
assert_eq!(blurred.len(), 25);
```

## Tests

`cargo test -p dsp-conv` — 26 unit tests + 1 doctest:

- 15 from Phase 1+2 (1-D): error paths, identity / centred-delta,
  output length contract, all 4 boundary modes with handwritten
  expected outputs, fir cross-check, periodicity / replicate /
  symmetry / integral invariants.
- 11 from Phase 3 (2-D, this release): error paths (zero dims,
  size mismatch, kernel-too-large), 1×1 + 3×3 identity, constant
  image + box kernel = constant, 3×3 outer-product matches
  sequential conv1d (separability cross-check), boundary modes
  produce distinct corner values for a 3×3 image / 3×3 uniform
  kernel.
