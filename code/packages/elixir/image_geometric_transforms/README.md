# image_geometric_transforms

IMG04 — Geometric transforms on `PixelContainer` images.

Part of the **coding-adventures** image-processing pipeline
(IC00 → IMG00 → IMG01 → IMG02 → IMG03 → **IMG04**).

## What it does

Remaps pixel *locations* rather than pixel *values*.  Every output pixel is
computed by mapping its coordinate backwards into the source image (the
**inverse warp** convention) and sampling there:

```
(u, v) = T⁻¹(x′, y′)       — inverse warp
O(x′, y′) = sample(I, u, v) — sampling with interpolation
```

Two families of transforms are provided:

| Family             | Functions                                  | Pixel quality? |
|--------------------|--------------------------------------------|----------------|
| Integer (lossless) | `flip_horizontal`, `flip_vertical`,        | Exact copy     |
|                    | `rotate_90_cw`, `rotate_90_ccw`,           |                |
|                    | `rotate_180`, `crop`, `pad`                |                |
| Continuous (lossy) | `scale`, `rotate`, `affine`,               | Interpolated   |
|                    | `perspective_warp`                         |                |

Lossless transforms copy raw RGBA8 bytes with no arithmetic.
Continuous transforms blend in **linear light** — averaging in sRGB is
physically incorrect because sRGB is a perceptual (gamma-encoded) space.

## How it fits in the stack

```
PixelContainer (IC00)
    └── image_point_ops  (IMG03) — per-pixel value transforms
    └── image_geometric_transforms (IMG04) — location remapping  ← you are here
```

## Installation

Add to your `mix.exs`:

```elixir
defp deps do
  [
    {:coding_adventures_image_geometric_transforms,
     path: "../image_geometric_transforms"}
  ]
end
```

## Usage

```elixir
alias CodingAdventures.PixelContainer, as: PC
alias CodingAdventures.ImageGeometricTransforms, as: GT

img = PC.new(400, 300)
# ... fill pixels ...

# Lossless
flipped  = GT.flip_horizontal(img)
cropped  = GT.crop(img, 10, 10, 200, 150)
padded   = GT.pad(img, 10, 10, 10, 10, {0, 0, 0, 255})

# Scale up 2×  (bilinear, default)
big      = GT.scale(img, 800, 600)

# Rotate 45° counter-clockwise, expand canvas to fit
rotated  = GT.rotate(img, :math.pi() / 4)

# Arbitrary affine (identity)
id       = {{1, 0, 0}, {0, 1, 0}}
same     = GT.affine(img, id, 400, 300)

# Perspective de-warp
h        = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}  # identity homography
rectified = GT.perspective_warp(img, h, 400, 300)
```

## Interpolation modes

| Atom         | Quality | Notes                           |
|--------------|---------|---------------------------------------|
| `:nearest`   | Low     | Pixel-art look, fastest               |
| `:bilinear`  | Good    | 2×2 linear-light blend (default)      |
| `:bicubic`   | Best    | 4×4 Catmull-Rom kernel, sharpest      |

## Out-of-bounds modes

| Atom          | Behaviour                                     |
|---------------|-----------------------------------------------|
| `:zero`       | Transparent black `{0, 0, 0, 0}`              |
| `:replicate`  | Clamp to nearest edge pixel (default)         |
| `:reflect`    | Mirror at boundary                            |
| `:wrap`       | Tile the image (modular arithmetic)           |

## Running tests

```bash
mix test --cover
```

Coverage target: ≥ 90% (currently ~99%).

## References

- IMG04 spec: `code/specs/IMG04-geometric-transforms.md`
- IMG00: pixel container and colour science foundations
- IMG03: per-pixel point operations (sRGB ↔ linear helpers shared here)
