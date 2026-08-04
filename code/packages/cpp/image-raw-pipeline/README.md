# image-raw-pipeline (C++)

The shared RAW colour-development pipeline, **header-only** in pure ISO C++17
(namespace `ca::image_raw_pipeline`). A faithful port of the Rust
[`image-raw-pipeline`](../../rust/image-raw-pipeline) crate.

## What it does

Camera RAW formats all turn raw sensor values into a displayable sRGB image with
the same four-stage pipeline:

1. **Normalize** — subtract the black-level pedestal, scale to `[0, 1]`.
2. **White balance** — per-channel multipliers.
3. **Colour matrix** — a 3×3 camera→sRGB matrix, then clamp to `[0, 1]`.
4. **sRGB gamma** — IEC 61966-2-1 transfer function, scaled to 8-bit.

## API

- `srgb_gamma` / `srgb_decode` — the sRGB EOTF and its inverse.
- `mat3x3_mul` — 3×3 matrix × column vector (`Mat3` = `array<array<double,3>,3>`,
  `Vec3` = `array<double,3>`).
- `invert_3x3` — analytic inversion returning `std::optional<Mat3>`.
- `apply_color_pipeline` — the full development, returning `std::vector<Rgb8>`.

## Design notes

- **Exceptions/optionals, not `Option`/`Vec`.** `invert_3x3` returns
  `std::optional` (Rust `Option`); `apply_color_pipeline` returns
  `std::vector<Rgb8>` (Rust `Vec`).
- **No `<cmath>`.** The sRGB gamma's fractional `pow` is computed from scratch,
  matching the Rust f64 `powf` to ~1e-12 relative.
- **Header-only.** `#include "image_raw_pipeline.hpp"` and go.

## Usage

```cpp
#include "image_raw_pipeline.hpp"
using namespace ca::image_raw_pipeline;

Mat3 id = {{{1,0,0},{0,1,0},{0,0,1}}};
auto out = apply_color_pipeline({{65535, 65535, 65535}}, 0, 65535,
                                {1.0, 1.0, 1.0}, id);
// out[0] == Rgb8{255, 255, 255}
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
