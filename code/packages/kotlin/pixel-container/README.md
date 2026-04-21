# pixel-container (Kotlin)

**Spec:** IC00 — Universal RGBA8 Pixel Buffer

A zero-dependency Kotlin port of `PixelContainer`, the narrow-waist data type
that every image codec and processing stage in the coding-adventures stack
shares. If you can turn your bytes into a `PixelContainer`, every downstream
library can read them; if your library produces a `PixelContainer`, every
encoder can write them out.

## Layout

```
offset(x, y) = (y * width + x) * 4
data[offset + 0] = R   (0..255)
data[offset + 1] = G   (0..255)
data[offset + 2] = B   (0..255)
data[offset + 3] = A   (0..255)
```

Row-major, top-left origin, 4 channels x 8 bits each, no padding or stride.

## Components

- `PixelContainer(width, height, data = ZeroedBuffer)` — data holder.
- `PixelOps` — `pixelAt`, `setPixel`, `fillPixels`.
- `ImageCodec` — abstract `encode`/`decode`/`mimeType` for codecs to extend.

## Usage

```kotlin
import com.codingadventures.pixelcontainer.PixelContainer
import com.codingadventures.pixelcontainer.PixelOps

val c = PixelContainer(64, 64)
PixelOps.fillPixels(c, 255, 255, 255, 255)           // white canvas
PixelOps.setPixel(c, 10, 10, 255, 0, 0, 255)         // one red pixel
val rgba = PixelOps.pixelAt(c, 10, 10)               // -> [255, 0, 0, 255]
```

Out-of-bounds reads return `(0, 0, 0, 0)`; out-of-bounds writes are silently
ignored. This keeps the read path safe inside geometric transforms whose
sample coordinates can legitimately fall off the image.

## Where it fits

`pixel-container` has no dependencies. Libraries that depend on it include
`image-point-ops` (IMG03) and `image-geometric-transforms` (IMG04).

## Tests

```
gradle test
```
