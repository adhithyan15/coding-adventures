# coding-adventures-image-point-ops (Ruby)

**IMG03** — Per-pixel point operations for the `PixelContainer` image type.

## Stack position

```
IMG03 CodingAdventures::ImagePointOps
 └── IC00 CodingAdventures::PixelContainer
```

## Operations

| Category     | Methods |
|--------------|---------|
| u8-domain    | `invert`, `threshold`, `threshold_luminance`, `posterize`, `swap_rgb_bgr`, `extract_channel`, `brightness` |
| Linear-light | `contrast`, `gamma`, `exposure`, `greyscale`, `sepia`, `colour_matrix`, `saturate`, `hue_rotate` |
| Colorspace   | `srgb_to_linear_image`, `linear_to_srgb_image` |
| LUT          | `apply_lut1d_u8`, `build_lut1d_u8`, `build_gamma_lut` |

## Usage

```ruby
require "coding_adventures/image_point_ops"

ops = CodingAdventures::ImagePointOps
pc  = CodingAdventures::PixelContainer

img = pc.create(640, 480)

inverted = ops.invert(img)
darkened  = ops.gamma(img, 2.0)
bw        = ops.greyscale(img, :rec709)
```

## Testing

```
ruby -I lib -I ../pixel_container/lib test/test_image_point_ops.rb
```
