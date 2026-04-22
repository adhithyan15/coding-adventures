# ImageGeometricTransforms (Swift)

**IMG04** — Spatial / geometric image transforms for the `PixelContainer` image type.

## Stack position

```
IMG04 ImageGeometricTransforms
 └── IC00 PixelContainer   (RGBA8 raster store)
```

## What it does

ImageGeometricTransforms provides operations that rearrange pixels spatially —
flipping, rotating, cropping, padding, scaling, and warping — as opposed to
point operations (IMG03) that change pixel values in place.

## Operations

| Category          | Functions |
|-------------------|-----------|
| Lossless flips    | `flipHorizontal`, `flipVertical` |
| Lossless rotation | `rotate90CW`, `rotate90CCW`, `rotate180` |
| Lossless crop/pad | `crop`, `pad` |
| Resampling scale  | `scale` |
| Resampling rotate | `rotate` |
| Affine warp       | `affine` |
| Perspective warp  | `perspectiveWarp` |

## Types

```swift
public enum Interpolation { case nearest, bilinear, bicubic }
public enum RotateBounds  { case fit, crop }
public enum OutOfBounds   { case zero, replicate, reflect, wrap }
public typealias Rgba8 = (UInt8, UInt8, UInt8, UInt8)
```

## Interpolation

All resampling operations accept an `Interpolation` mode:

- **`.nearest`** — round the source coordinate to the nearest integer pixel.
  Fastest; produces a "pixelated" look when upscaling.
- **`.bilinear`** — blend the 2×2 neighbourhood with linear weights.
  Smooth at moderate magnification.
- **`.bicubic`** — blend the 4×4 neighbourhood with Catmull-Rom cubic weights.
  Sharpest; may show slight ringing at high-contrast edges.

All blending is performed in **linear light** (sRGB decoded via the EOTF)
and re-encoded to sRGB on output, which is the physically correct approach.

## Out-of-bounds handling

When a sampler requests a coordinate outside the source image, the `OutOfBounds`
parameter controls the response:

| Mode         | Behaviour |
|--------------|-----------|
| `.zero`      | Return transparent black `(0,0,0,0)` |
| `.replicate` | Clamp to the nearest border pixel |
| `.reflect`   | Mirror-reflect across the border |
| `.wrap`      | Tile the image periodically |

## Usage

```swift
import PixelContainer
import ImageGeometricTransforms

// Load an image into src: PixelContainer ...

// Lossless flips and rotations
let hFlipped  = flipHorizontal(src)
let rotated90 = rotate90CW(src)
let flipped   = rotate180(src)

// Crop a 100×100 region from (50, 50)
let thumb = crop(src, x: 50, y: 50, w: 100, h: 100)

// Add a 10-pixel white border
let framed = pad(src, top: 10, right: 10, bottom: 10, left: 10,
                 fill: (255, 255, 255, 255))

// Scale to half size with bilinear interpolation (default)
let half = scale(src, width: src.width / 2, height: src.height / 2)

// Rotate 30° counter-clockwise, expanding canvas to fit
let tilted = rotate(src, radians: Float.pi / 6, mode: .bicubic, bounds: .fit)

// Affine shear (2×3 matrix)
let shear: [[Float]] = [[1, 0.3, 0], [0, 1, 0]]
let sheared = affine(src, matrix: shear, width: src.width, height: src.height)

// Perspective warp (3×3 homogeneous matrix)
let h: [[Float]] = [[1, 0.001, 0], [0, 1, 0], [0, 0.001, 1]]
let warped = perspectiveWarp(src, matrix: h, width: src.width, height: src.height)
```

## Testing

```
swift test
```

All 28 tests should pass.  Coverage targets every public function, including
round-trip identities, dimension assertions, exact pixel-value checks for
lossless operations, and near-identity checks (±2 per channel) for resampling
operations.
