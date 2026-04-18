# pixel-container

A pure F# RGBA8 pixel buffer package for the .NET paint stack.

`pixel-container` is the lowest layer in the paint pipeline. Paint backends,
image codecs, and higher-level renderers can all agree on one simple in-memory
format: width, height, and row-major RGBA bytes.

## What It Provides

- Bounds-safe pixel reads and writes
- Whole-buffer fill helpers
- Construction from dimensions or existing RGBA data
- A tiny `IImageCodec` contract for packages that add concrete encoders later

## Usage

```fsharp
open CodingAdventures.PixelContainer

let pixels = PixelContainers.create 2 2
pixels.Fill(255uy, 255uy, 255uy, 255uy)
pixels.SetPixel(1, 0, 37uy, 99uy, 235uy, 255uy)

let sample = pixels.GetPixel(1, 0)
// sample = { R = 37uy; G = 99uy; B = 235uy; A = 255uy }
```

## Development

```bash
bash BUILD
```
