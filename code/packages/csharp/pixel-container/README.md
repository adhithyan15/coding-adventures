# pixel-container

A pure C# RGBA8 pixel buffer package for the .NET paint stack.

`pixel-container` is the lowest layer in the paint pipeline. Paint backends,
image codecs, and higher-level renderers can all agree on one simple in-memory
format: width, height, and row-major RGBA bytes.

## What It Provides

- Bounds-safe pixel reads and writes
- Whole-buffer fill helpers
- Construction from dimensions or existing RGBA data
- A tiny `IImageCodec` contract for packages that add concrete encoders later

## Usage

```csharp
using CodingAdventures.PixelContainer;

var pixels = PixelContainers.Create(2, 2);
pixels.Fill(255, 255, 255, 255);
pixels.SetPixel(1, 0, 37, 99, 235, 255);

var sample = pixels.GetPixel(1, 0);
// sample == new Rgba(37, 99, 235, 255)
```

## Development

```bash
bash BUILD
```
