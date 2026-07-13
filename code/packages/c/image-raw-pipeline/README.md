# image-raw-pipeline (C)

The shared RAW colour-development pipeline, in **pure ISO C17**. A faithful port
of the Rust [`image-raw-pipeline`](../../rust/image-raw-pipeline) crate.

## What it does

Camera RAW formats (TIFF, DNG, CR2, NEF, ARW, RAF, ORF, RW2) all turn raw sensor
values into a displayable sRGB image with the same four-stage pipeline:

1. **Normalize** — subtract the black-level pedestal, scale to `[0, 1]` by the
   white level (saturating, so sub-black values clamp to 0).
2. **White balance** — per-channel multipliers correct the scene illuminant.
3. **Colour matrix** — a 3×3 camera→sRGB matrix, then clamp to `[0, 1]`.
4. **sRGB gamma** — the IEC 61966-2-1 transfer function, scaled to 8-bit.

## API

- `irp_srgb_gamma` / `irp_srgb_decode` — the sRGB EOTF and its inverse
  (piecewise linear segment + power law).
- `irp_mat3x3_mul` — 3×3 matrix × column vector (no allocation).
- `irp_invert_3x3` — analytic 3×3 inversion via Cramer's rule (returns `0` for a
  singular matrix).
- `irp_apply_color_pipeline` — the full development; returns a malloc'd
  `IrpRgb8 *` (caller frees) with an `IrpStatus`.

## Design notes

- **No libm.** The sRGB gamma's fractional `pow` is computed from a from-scratch
  `exp`/`ln`, so the package needs no `<math.h>` and no `-lm`. It reproduces the
  Rust f64 `powf` to ~1e-12 relative — the round-trip `decode(gamma(x)) ≈ x`
  holds to 1e-10.
- **Faithful divergences.** Rust `Option`→a `1/0` flag + out-param; Rust `Vec`→a
  malloc'd buffer + `IrpStatus`. The output allocation is guarded against
  `size_t` overflow.

## Usage

```c
#include "image_raw_pipeline.h"

IrpRaw16 pixels[1] = {{65535, 65535, 65535}};       /* pure white */
double wb[3] = {1.0, 1.0, 1.0};
double id[3][3] = {{1,0,0},{0,1,0},{0,0,1}};
IrpRgb8 *out = NULL;
if (irp_apply_color_pipeline(pixels, 1, 0, 65535, wb, id, &out) == IRP_OK) {
    /* out[0] == {255, 255, 255} */
    free(out);
}
```

## Building

```sh
sh BUILD           # POSIX: GCC and/or Clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
