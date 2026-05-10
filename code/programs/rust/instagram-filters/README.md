# instagram-filters

End-to-end demo program for the matrix execution layer.  Takes a PPM
image, applies an Instagram-style filter expressed as a MatrixIR
graph, writes a PPM image.

## What this proves

This program runs your image through the entire matrix execution
layer:

```
PPM bytes
   ↓
PixelContainer
   ↓
image-gpu-core (builds matrix_ir::Graph)
   ↓
matrix-runtime planner (lowers to compute-ir ComputeGraph)
   ↓
matrix-cpu executor (evaluates)
   ↓
PixelContainer
   ↓
PPM bytes
```

If the matrix execution layer is broken, the output image is *visibly*
wrong — colours shifted, gamma off, channels swapped, alpha lost.
That makes this a much stronger smoke test than asserting `[1, 2, 3]`
round-trips through a graph.

## Filters

```
invert                              Invert RGB channels.  Alpha unchanged.
greyscale | grayscale               Rec.709 luminance, linear light.
sepia                               Classic Microsoft 3×3 sepia matrix.
brightness   --amount N             Add N ∈ [-255, 255] to each channel,
                                    clamped to [0, 255].
gamma        --gamma G              Power-law gamma in linear light.
                                    γ < 1 brightens midtones, γ > 1 darkens.
contrast     --scale S              Stretch around mid-grey 128.
                                    S > 1 increases contrast,
                                    0 < S < 1 lowers it.
posterize    --levels L             Reduce to L distinct values per channel.
```

Every filter decomposes into MatrixIR primitives:

| Filter | Primitives used |
|--------|-----------------|
| invert | `Const`, `Sub`, `Where` |
| greyscale | `Cast`, `Reshape`, `MatMul` |
| sepia | same as greyscale + custom matrix constant |
| brightness | `Cast`, `Add`, `Min`, `Max`, `Where`, `Broadcast` |
| gamma | `Cast`, `Pow`, `Broadcast` |
| contrast | `Cast`, `Sub`, `Mul`, `Add`, `Min`, `Max`, `Where`, `Broadcast` |
| posterize | `Cast`, `Div`, `Mul`, `Where`, `Broadcast` |

If you pick the right filter you'll exercise nearly every primitive
the matrix execution layer supports.

## Usage

```sh
# Build it
$ cd code/programs/rust/instagram-filters
$ cargo build --release

# Apply filters
$ ./target/release/instagram-filters --input photo.ppm --output sepia.ppm --filter sepia
$ ./target/release/instagram-filters --input photo.ppm --output b.ppm    --filter brightness --amount 30
$ ./target/release/instagram-filters --input photo.ppm --output g.ppm    --filter gamma --gamma 0.7
$ ./target/release/instagram-filters --input photo.ppm --output h.ppm    --filter contrast --scale 1.5
$ ./target/release/instagram-filters --input photo.ppm --output p.ppm    --filter posterize --levels 4
$ ./target/release/instagram-filters --input photo.ppm --output gr.ppm   --filter greyscale
$ ./target/release/instagram-filters --input photo.ppm --output i.ppm    --filter invert
```

## File format

Input and output are **PPM (P6)** files — the simple binary format
that `image-codec-ppm` supports.  PNG read isn't yet implemented in
this repo's PNG codec; once it is, this program will gain a
`--format` flag and accept either.

You can convert images to/from PPM with ImageMagick:

```sh
$ convert photo.jpg photo.ppm           # → P6 RGB binary
$ convert photo.ppm photo.png           # PPM → PNG
```

## Architecture

```
code/programs/rust/instagram-filters/
├── src/lib.rs   — Filter enum, parameter validation, apply_filter
└── src/main.rs  — CLI argument parsing, file I/O, dispatch
```

The library half is fully unit-testable without touching the
filesystem; the binary half handles only argument parsing and I/O.

## Trust model

`--input` and `--output` are treated as literal filesystem paths,
same as `cp`.  Path traversal is allowed at the user's discretion.
The program protects against runaway memory usage by capping input
file size at 64 MiB before reading.

## Testing

```
cargo test
```

22 tests pass:
- 13 library tests (parameter validation, end-to-end filter
  dispatches on synthetic 2×2 images)
- 9 binary tests (argument parser corner cases)

## Ideas for follow-up

- Add `--format png` support once PNG decode lands
- Add filter chains (`--filter "sepia | contrast --scale 1.2"`)
- Wire to `cli-builder` for richer help generation
- GPU executor support — when matrix-metal or matrix-cuda lands,
  this program automatically gets faster on capable hosts because
  the matrix execution layer planner routes large workloads to the
  GPU
