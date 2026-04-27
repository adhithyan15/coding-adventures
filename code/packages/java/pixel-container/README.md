# Java Pixel Container

IC00 — the zero-dependency foundation of the coding-adventures image
processing stack. Every image codec, point operation, geometric transform,
and compositing stage builds on this one class.

## What it provides

- `PixelContainer` — a flat RGBA8 pixel buffer with `width`, `height`, and
  a `byte[]` of pixels laid out row-major from the top-left.
- `PixelOps` — static helpers to read, write, and fill pixels, plus a
  convenience `create(width, height)` factory.
- `ImageCodec` — the abstract interface every image codec (PNG, JPEG,
  BMP, …) implements: `mimeType()`, `encode`, and `decode`.

## Layout

```
offset = (y * width + x) * 4
data[offset + 0] = R
data[offset + 1] = G
data[offset + 2] = B
data[offset + 3] = A
```

All channels are unsigned 8-bit. The format is fixed by design — higher
layers decode/encode to this universal representation.

## Usage

```java
import com.codingadventures.pixelcontainer.PixelContainer;
import com.codingadventures.pixelcontainer.PixelOps;

PixelContainer img = PixelOps.create(16, 16);
PixelOps.fillPixels(img, 255, 0, 0, 255);   // solid red
PixelOps.setPixel(img, 0, 0, 0, 255, 0, 255); // one green pixel
int[] rgba = PixelOps.pixelAt(img, 0, 0);
```

## Place in the stack

| Layer | Depends on                |
|-------|---------------------------|
| IC00  | nothing                   |
| IMG03 | IC00                      |
| IMG04 | IC00                      |
| PNG, JPEG, BMP codecs | IC00 (via ImageCodec) |
