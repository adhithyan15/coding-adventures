# coding-adventures-image-point-ops (Elixir)

**IMG03** — Per-pixel point operations for the `PixelContainer` image type.

## Stack position

```
IMG03 CodingAdventures.ImagePointOps
 └── IC00 CodingAdventures.PixelContainer
```

## Operations

| Category     | Functions |
|--------------|-----------|
| u8-domain    | `invert/1`, `threshold/2`, `threshold_luminance/2`, `posterize/2`, `swap_rgb_bgr/1`, `extract_channel/2`, `brightness/2` |
| Linear-light | `contrast/2`, `gamma/2`, `exposure/2`, `greyscale/2`, `sepia/1`, `colour_matrix/2`, `saturate/2`, `hue_rotate/2` |
| Colorspace   | `srgb_to_linear_image/1`, `linear_to_srgb_image/1` |
| LUT          | `apply_lut1d_u8/4`, `build_lut1d_u8/1`, `build_gamma_lut/1` |

## Usage

```elixir
alias CodingAdventures.PixelContainer, as: PC
alias CodingAdventures.ImagePointOps, as: Ops

img = PC.new(640, 480)

inverted = Ops.invert(img)
darkened  = Ops.gamma(img, 2.0)
bw        = Ops.greyscale(img, :rec709)
```

## Testing

```
mix test
```
