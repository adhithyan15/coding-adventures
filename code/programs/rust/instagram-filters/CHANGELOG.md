# Changelog — instagram-filters

## 0.1.0 — 2026-05-04

Initial release.  End-to-end demo program proving the matrix execution
layer works on a real workload — PPM image in, MatrixIR-described
filter, PPM image out.

### Added

- CLI: `--input PATH --output PATH --filter NAME [filter args]`
- Seven filters wired up:
  - `invert` — RGB inversion
  - `greyscale` (alias `grayscale`) — Rec.709 luminance
  - `sepia` — classic 3×3 sepia matrix
  - `brightness --amount N` — additive shift, clamped
  - `gamma --gamma G` — power-law in linear light
  - `contrast --scale S` — stretch around mid-grey
  - `posterize --levels L` — reduce to L distinct values per channel
- All filters delegate to `image-gpu-core`, which builds a MatrixIR
  graph and dispatches through `matrix-runtime` + `matrix-cpu`.
- I/O via PPM (`image-codec-ppm`) — PNG decode isn't yet implemented
  in this repo's PNG codec, so PPM is the V1 transport.
- Library half (`src/lib.rs`) separates filter selection / parameter
  validation from the I/O concerns in `src/main.rs`, so unit tests
  exercise the filter dispatch without touching the filesystem.

### Tests: 22 passing

- 13 library tests (parser validation per filter, dispatch round-trips
  for each filter at the API level)
- 9 main-binary CLI argument-parsing tests

### Constraints

- Input file size capped at 64 MiB to bound OOM impact.
- Path arguments are treated literally — same trust model as `cp`.
- Zero external Cargo dependencies (only path deps to image-gpu-core,
  image-codec-ppm, pixel-container).
