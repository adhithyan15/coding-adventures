# ARCH01 — Image ↔ DSP Routing Rule

## Why this spec exists

The image layer (`image-*` crates) and the DSP layer (`dsp-*` crates) both
sit on top of the matrix execution layer (`matrix-ir`, `matrix-runtime`,
`matrix-cpu`, `matrix-metal`, `matrix-cuda`). They have **overlapping
operations** — convolution is the obvious one; FFT, DCT, and wavelets for
images are all DSP primitives applied to 2-D pixel data.

Without a written rule, whoever implements an image-side convolution (or
image FFT, or image DCT, or image wavelet) first will reach for the easiest
path — copy the algorithm, the kernel zoo, the padding modes, the
separable optimisation, and the matrix-IR lowering into the image layer.

We then end up maintaining the same Gaussian blur in two places, with two
test suites, two padding-mode enums, two matrix-IR lowerings, and inevitable
drift between them. This spec exists to **establish the rule once**, before
that happens, so the boundary is clear to everyone implementing IMG
operations.

This is a small cross-cutting spec — not an implementation spec. It
constrains how IMG specs are written and how `image-*` crates depend on
`dsp-*` crates. It does not change any existing code (no `image-*` crate
currently does convolution, FFT, DCT, or wavelets — the rule is being set
before any such code is written).

---

## The current state (2026-05-16)

### What ships in `image-*` today

| Crate                          | Version | What it does                                                 |
|--------------------------------|---------|--------------------------------------------------------------|
| `image-codec-bmp`              | 0.x     | BMP encode/decode                                            |
| `image-codec-ppm`              | 0.x     | PPM encode/decode                                            |
| `image-codec-qoi`              | 0.x     | QOI encode/decode                                            |
| `image-point-ops`              | 0.1.0   | Pure-Rust scalar reference: per-pixel point ops over `PixelContainer` (invert, threshold, gamma, brightness, contrast, sepia, colour_matrix, srgb↔linear, LUT helpers) |
| `image-geometric-transforms`   | 0.1.0   | Pure-Rust scalar reference: flip, rotate, crop, pad, scale, affine, perspective_warp |
| `image-gpu-core`               | 0.15.0  | The **matrix-IR-lowered** mirror of `image-point-ops` — `gpu_invert`, `gpu_greyscale`, `gpu_gamma`, `gpu_brightness`, `gpu_sepia`, `gpu_contrast`, `gpu_posterize`, `gpu_colour_matrix`. Builds graphs directly using `matrix-ir`, runs them through `matrix-runtime` / `matrix-cpu` / `matrix-metal` / `matrix-cuda`. |

Two tiers of image ops exist today:

1. **Pure-Rust scalar reference** (`image-point-ops`, `image-geometric-transforms`).
2. **Matrix-IR-lowered for GPU lift** (`image-gpu-core`).

Both depend on `pixel-container` for the image data type. Neither depends
on any `dsp-*` crate. This is **correct** for what they currently do —
none of those operations have DSP-layer counterparts (the DSP layer has no
"invert a per-pixel colour" or "rotate an image 90°" primitive, because
those aren't 1-D signal operations).

### What ships in `dsp-*` today

| Crate          | Version | What it does                                                            |
|----------------|---------|-------------------------------------------------------------------------|
| `dsp-complex`  | 0.1.0   | `ComplexTensor` view over interleaved `[re, im]` `[..., 2]` buffers     |
| `dsp-fft`      | 0.7.1   | Radix-2 + Bluestein FFT; scalar AND matrix-IR-lowered                   |
| `dsp-dct`      | 0.2.0   | DCT-II/III (1-D + 2-D); Makhoul reduction over `dsp-fft`                |
| `dsp-filters`  | 0.3.0   | FIR + IIR filters, window types (Hann/Hamming/Blackman/Rectangular)     |
| `dsp-conv`     | 0.3.0   | `conv1d`, `conv2d`, `sep_conv2d`, kernel zoo (Gaussian, box, Sobel x/y, Laplacian, sharpen); Phase 5 image filter design landed, Phase 6 matrix-IR-lowered pending |
| `dsp-stft`     | 0.4.0   | STFT/ISTFT, magnitude/log spectrogram, mel filterbank, mel-spectrogram, MFCC; scalar AND matrix-IR-lowered |

Every `dsp-*` crate operates on flat `&[f32]` slices (1-D) or row-major
`&[f32]` with explicit `(rows, cols)` (2-D). None of them know about
`PixelContainer` or any image-domain concept (multi-channel pixels, sRGB
encoding, alpha premultiplication, etc.).

### What's planned but not yet built

| Spec   | Subject                       | Implementation status                   |
|--------|-------------------------------|-----------------------------------------|
| IMG00  | Image data model              | Spec only; `pixel-container` crate ships |
| IMG01  | Convolution + spatial filters | **Spec only — no implementation yet.**  |
| IMG02  | Look-up tables (LUTs)         | Spec only; partly covered by `image-point-ops` |
| IMG03  | Point operations              | Implementation: `image-point-ops` + `image-gpu-core` |
| IMG04  | Geometric transforms          | Implementation: `image-geometric-transforms` |
| IMG05  | Compositing                   | Spec only                               |
| IMG06  | GPU bridge                    | Implementation: `image-gpu-core`        |
| DSP06  | Wavelets (next layer)         | Spec pending                            |

**IMG01-convolution.md is the immediate trigger** for this spec: it's the
first IMG spec whose subject overlaps entirely with an existing `dsp-*`
crate (`dsp-conv`). Whoever implements IMG01 next will reach for either
"copy dsp-conv into image-convolution" or "thin adapter over dsp-conv" —
this spec mandates the latter.

---

## The rule

**For any image-domain operation that has a meaningful DSP-layer
counterpart, the image-side implementation MUST be a thin adapter over the
DSP-layer implementation. The kernel math, the padding modes, the
optimisation passes (separable / sliding-window / FFT-based / matrix-IR
lowering), and the test oracles all live in the DSP layer exactly once.**

The image-side adapter is responsible for, and only for, the
image-shaped concerns:

1. **`PixelContainer` ↔ flat `&[f32]` conversion.** `PixelContainer` stores
   `u8` pixels in interleaved RGBA8 or grayscale layout; DSP operates on
   `f32` slices. The adapter pulls the right channel(s) out, converts
   `u8 → f32` (and back), and handles the row stride correctly.
2. **Multi-channel iteration.** DSP convolution is per-channel; the
   adapter loops over R, G, B (and optionally A) and forwards each one to
   the DSP routine.
3. **Colour-space conversions.** IMG01 §9 requires linear-light filtering
   for visually correct blurs. The adapter calls
   `image-point-ops::srgb_to_linear_image` before the DSP routine and
   `linear_to_srgb_image` after.
4. **Alpha-premultiplication handling.** IMG01 §8 requires special
   treatment for premultiplied-alpha images (blur RGB as-is, don't
   re-multiply). The adapter checks the `PixelContainer`'s alpha state
   and routes accordingly.
5. **Image-shaped error wrapping.** DSP errors (`StftError`,
   `ConvError`, …) get wrapped into an image-layer error type so callers
   don't have to know about DSP internals.

The image-side adapter is responsible for **nothing else**. In particular:

- The image adapter **does not** know which convolution algorithm to use
  (direct vs separable vs FFT-based). That's `dsp-conv`'s job.
- The image adapter **does not** know how a Gaussian kernel is computed.
  That's `dsp-conv::gaussian_blur_kernel`'s job.
- The image adapter **does not** lower convolution to matrix-IR. That's
  `dsp-conv` Phase 6's job (and the image adapter automatically benefits
  once Phase 6 lands).
- The image adapter **does not** maintain its own `PaddingMode` enum.
  It reuses `dsp_conv::PaddingMode` (or whatever the DSP-side type is
  called).

---

## How to decide which side an operation belongs on

The rule applies to operations with a **meaningful DSP counterpart**.
Concretely:

### Route through DSP (image adapter wraps `dsp-*`)

An operation belongs in the DSP layer with an image-side wrapper if:

- It generalises to non-image data (1-D signals, audio, scientific time
  series). Convolution, FFT, DCT, wavelets, STFT all qualify — they're
  mathematical operations that happen to be useful on images.
- Multiple DSP-layer optimisations apply (separable, FFT-based, matrix-IR
  lowering). The image layer should not have to know which path is
  fastest.
- The operation has well-known reference implementations (scipy.signal,
  scipy.fft, scipy.ndimage, pywavelets) that the DSP layer can be tested
  against, and the image layer can then inherit that correctness.

**Concrete list (today + planned):**

| Image op                              | DSP-layer home                                   |
|---------------------------------------|--------------------------------------------------|
| `convolve2d`, `convolve_separable`    | `dsp-conv::conv2d` / `sep_conv2d`                |
| `gaussian_blur`, `box_blur`           | `dsp-conv::sep_conv2d` + kernel zoo              |
| `sobel`, `prewitt`, `laplacian`       | `dsp-conv::conv2d` + kernel zoo (Sobel/Laplacian) |
| `sharpen`, `unsharp_mask`             | `dsp-conv` (sharpen kernel) + add-back composition |
| Image FFT / image DFT                 | `dsp-fft::rfft` / `fft_via_runtime`              |
| Image DCT (JPEG-style 8×8)            | `dsp-dct::dct_2d` / future Phase 5 Loeffler 8×8  |
| Image wavelet (JPEG 2000-style)       | `dsp-wavelets::dwt_2d` (DSP06, pending)          |
| Image spectrograms (rare, but possible) | `dsp-stft`                                      |

### Stay image-only (`image-*` builds directly on `matrix-ir` if matrix-lowered)

An operation belongs in the image layer if:

- It's inherently a pixel/colour-domain operation with no signal-processing
  analogue.
- The "1-D version" is either trivial (single-pixel) or meaningless.
- There's no DSP literature describing it — it's specific to image
  processing.

**Concrete list (today):**

| Image op                                   | Lives in                              |
|--------------------------------------------|---------------------------------------|
| Per-pixel colour ops (`invert`, `gamma`, `brightness`, `contrast`, `sepia`, `colour_matrix`, `posterize`, `threshold`, `greyscale`) | `image-point-ops` (scalar) + `image-gpu-core` (matrix-IR) |
| Channel extraction / swapping              | `image-point-ops`                     |
| sRGB ↔ linear conversion                   | `image-point-ops`                     |
| LUT application                            | `image-point-ops`                     |
| Geometric transforms (flip, rotate, crop, pad, scale, affine, perspective_warp) | `image-geometric-transforms` (today scalar only; future GPU mirror would build matrix-IR directly via the `image-gpu-core` pattern) |
| Compositing (Porter-Duff over/in/out)      | Future `image-compositing` (IMG05)    |
| Codec encode/decode                        | `image-codec-{bmp,ppm,qoi,...}`       |

These have no DSP-layer counterpart, so the `image-gpu-core` pattern
(build matrix-IR graphs directly, no DSP intermediate) is correct for
them.

### Edge cases

A few operations sit on the boundary. Resolutions:

- **Resize / scale.** Bicubic and Lanczos resampling can be expressed as
  separable 1-D convolutions. **Rule:** if the resize is implemented as
  "1-D conv per axis", route through `dsp-conv::sep_conv2d` with a
  Lanczos/cubic kernel. If it's implemented as direct bilinear sampling
  (the current `image-geometric-transforms::scale`), stay image-only —
  bilinear has no DSP analogue.
- **Morphological operations** (erode, dilate, open, close). These are
  min/max convolutions, not weighted-sum convolutions. **Rule:** stay
  image-only for now; if a DSP crate ever ships `min_conv` / `max_conv`,
  revisit.
- **Histogram operations** (equalisation, matching). These touch every
  pixel but aren't convolutions. **Rule:** image-only.
- **Bilateral filter** (edge-preserving blur). Pixel-value-dependent
  weights, not a fixed kernel. **Rule:** image-only for now; if DSP ever
  grows non-linear filters, revisit.

The decision criterion is always the same: **does the operation
generalise to non-image signal data?** If yes → DSP with image wrapper.
If no → image-only.

---

## The adapter recipe

This is the canonical pattern for an image-side wrapper. Example:
`image-convolution::gaussian_blur` wrapping `dsp-conv`.

```rust
// In a (future) `image-convolution` crate.

use dsp_conv::{sep_conv2d, gaussian_blur_kernel, PaddingMode as DspPad};
use image_point_ops::{srgb_to_linear_image, linear_to_srgb_image};
use pixel_container::PixelContainer;

/// Image-shaped error wrapping DSP errors plus image-specific ones.
#[derive(Debug)]
pub enum ConvolutionError {
    Dsp(String),                     // wrapping dsp_conv::ConvError
    UnsupportedPixelFormat(String),  // image-layer concern
    UnsupportedAlphaState(String),   // image-layer concern
}

/// Image-shaped padding mode, 1:1 with the DSP one.
/// We re-export the DSP enum rather than mirror it — single source of truth.
pub use dsp_conv::PaddingMode;

/// Gaussian blur with sigma in pixels.  Linear-light filtering (IMG01 §9).
pub fn gaussian_blur(
    src: &PixelContainer,
    sigma: f32,
    padding: PaddingMode,
) -> Result<PixelContainer, ConvolutionError> {
    // 1. Colour-space conversion: sRGB → linear.
    let linear = srgb_to_linear_image(src);

    // 2. Per-channel adapter loop.
    let radius = ((3.0 * sigma).ceil() as u32).max(1);  // ±3σ rule
    let kernel = gaussian_blur_kernel(radius, sigma);    // DSP owns the math

    let (h, w) = (linear.height(), linear.width());
    let mut out_channels: Vec<Vec<f32>> = Vec::with_capacity(3);
    for ch in [Channel::R, Channel::G, Channel::B] {
        let plane: Vec<f32> = extract_channel_to_f32(&linear, ch);
        let blurred = sep_conv2d(&plane, h, w, &kernel, &kernel, padding)
            .map_err(|e| ConvolutionError::Dsp(format!("{:?}", e)))?;
        out_channels.push(blurred);
    }
    // Alpha: pass through unchanged (or handle premultiplied case here).
    let alpha = extract_channel_to_f32(&linear, Channel::A);

    // 3. Recombine, convert back to u8, sRGB-encode.
    let out_linear = combine_channels_to_pixel_container(
        out_channels[0], out_channels[1], out_channels[2], alpha, h, w,
    );
    Ok(linear_to_srgb_image(&out_linear))
}
```

This is **the only** image-side code added. Everything else
(`gaussian_blur_kernel`, `sep_conv2d`, padding modes, the eventual
matrix-IR lowering, the `1e-5` accuracy guarantee from `dsp-conv`'s test
suite) is inherited from the DSP layer.

When `dsp-conv` Phase 6 ships its matrix-IR lowering, the adapter
optionally adds a `gaussian_blur_via_runtime` that builds the graph,
splices in the matrix-IR-lowered DSP convolution, and runs it through
`matrix-runtime` — **without touching the existing `gaussian_blur`
function**. Same pattern `dsp-stft` Phase 6 just established.

---

## What this means for the IMG specs

### IMG01 (convolution)

IMG01 currently defines `convolve2d`, `convolve_separable`,
`gaussian_blur`, `box_blur`, `sobel`, `laplacian`, `sharpen`, and
`unsharp_mask` as image-layer operations with no mention of `dsp-conv`.

**Action when IMG01 implementation starts:** the implementation crate
(provisionally `image-convolution`) takes a `dsp-conv` dependency and
follows the adapter recipe above. The spec text of IMG01 is updated with
a §13 reference to this spec.

IMG01's algorithm descriptions (§2–§7) remain accurate as exposition; the
implementation just doesn't reinvent them in image-side code.

### Future IMG-FFT / IMG-DCT / IMG-wavelets specs

When new IMG specs are written for FFT / DCT / wavelets / spectrograms
on images, they MUST cite this spec and route through `dsp-fft`,
`dsp-dct`, `dsp-wavelets` (DSP06, pending), and `dsp-stft` respectively.

The spec authors should not re-derive the algorithms in IMG specs — they
should reference the corresponding DSP spec and focus on the
image-specific concerns (colour space, multi-channel handling, alpha,
pixel-format edge cases).

### Existing IMG implementations that DON'T change

- `image-point-ops`, `image-gpu-core`, `image-geometric-transforms` —
  none of these implement DSP-overlapping ops. They stay as they are.
- `image-codec-*` — pure encode/decode, no DSP overlap. Stays as is.

The rule is **prospective only**: it constrains how new IMG ops are
implemented, not a retroactive refactor of working code.

---

## What this means for the DSP specs

Two implications:

1. **DSP-layer APIs MUST accept flat `&[f32]` slices, not image types.**
   `dsp-conv::sep_conv2d` takes `&[f32]` + `(rows, cols)` + kernels +
   padding; it does NOT take `&PixelContainer`. This keeps the DSP layer
   testable in isolation and reusable for non-image consumers (audio,
   scientific data, etc.). The image adapter does the conversion.

2. **DSP error types are DSP-shaped.** `ConvError`, `StftError`,
   `FftError`, `DctError`, `WaveletError` (future) are DSP-shaped and
   wrap into image-layer errors at the adapter boundary. No DSP error
   variant should mention `PixelContainer` or any image-domain term.

These are already the patterns the `dsp-*` crates follow. The rule
here is just to keep following them — don't add `PixelContainer`-aware
overloads in `dsp-*` to "save" the adapter conversion.

---

## What this does NOT mean

To pre-empt overreach:

- **`image-gpu-core`'s existing 0.15.0 ops are not affected.** They build
  matrix-IR graphs directly because their operations have no DSP
  counterpart. That is correct and stays.
- **`image-geometric-transforms`'s eventual GPU mirror is not forced
  through DSP.** Bilinear resampling, rotation, affine warp have no DSP
  analogue. A future `image-geometric-transforms-gpu` crate would mirror
  the `image-gpu-core` pattern: matrix-IR directly, no DSP intermediate.
- **DSP crates do not grow image-shaped overloads.** No
  `dsp_conv::gaussian_blur_pixel_container(...)`. That belongs on the
  image side.
- **This is not a "DSP knows about images" rule.** DSP remains
  domain-agnostic; the adapter is one-directional (image depends on DSP,
  not the reverse).

---

## Why this matters

When the architecture rule is implicit, each new implementer makes a
local decision that looks reasonable in isolation. After three or four
such decisions, you have:

- Two Gaussian-blur implementations (one in `image-convolution`, one in
  `dsp-conv`) that drift apart on edge cases (padding mode behaviour at
  zero-radius, Sobel sign conventions, rounding policy).
- Two padding-mode enums (`image_convolution::PaddingMode` and
  `dsp_conv::PaddingMode`) with subtly different semantics.
- Two matrix-IR lowerings of convolution, one in `dsp-conv` Phase 6 and
  one bespoke in `image-convolution` — and the image one never benefits
  from `dsp-conv`'s optimisation work.
- Test coverage that has to be replicated for both implementations, and
  bugs that get fixed in one and not the other.

The rule prevents all of this by making the boundary explicit before
the first IMG-convolution PR lands. The cost is one indirection
(`image_convolution::gaussian_blur` calls `dsp_conv::sep_conv2d`); the
benefit is a single source of truth for every DSP primitive used in
image processing.

---

## Phase / status

| Phase | Lands                                                  | Status |
|-------|--------------------------------------------------------|--------|
| 0     | This spec                                              | **this PR** |
| 1     | First IMG implementation that follows the rule (likely `image-convolution` wrapping `dsp-conv`) | pending |
| 2     | IMG01-convolution.md §13 reference back to this spec   | pending (small edit, can ride on the IMG01 implementation PR or land separately) |
| 3     | Each future IMG-FFT/DCT/wavelets/spectrogram spec cites this rule and routes through the corresponding `dsp-*` crate | pending — applies to specs not yet written |

---

## Out of scope

This spec does not address:

- **The internal architecture of any specific `image-*` crate.** Adapter
  layout is up to the implementing PR — this spec only mandates the
  routing, not the file structure.
- **The DSP layer's own internal architecture.** Whether `dsp-conv` does
  separable / FFT-based / matrix-IR-lowered convolution is `dsp-conv`'s
  problem; the image adapter just calls the public API.
- **Non-image consumers of DSP.** Audio (`paint-vm` audio pipeline,
  future `audio-*` crates) and scientific (`statistics-*`, `ml-*`) DSP
  consumers follow the same rule by default — depend on DSP directly —
  but they're not the subject of this spec.
- **GPU acceleration policy.** When DSP ships matrix-IR lowering for an
  op, the image adapter optionally adds a `*_via_runtime` variant; this
  spec does not mandate when.
