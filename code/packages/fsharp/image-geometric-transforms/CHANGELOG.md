# Changelog

All notable changes to `CodingAdventures.ImageGeometricTransforms` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-20

### Added

- **Types**
  - `Interpolation` discriminated union: `Nearest | Bilinear | Bicubic`
  - `RotateBounds` discriminated union: `Fit | Crop`
  - `OutOfBounds` discriminated union: `Zero | Replicate | Reflect | Wrap`
  - `Rgba8` type alias `byte * byte * byte * byte`

- **sRGB / linear-light LUT** — 256-entry decode table built once at module
  startup; `decode` and `encode` helpers for sRGB ↔ linear conversion used by
  bilinear and bicubic samplers.

- **Out-of-bounds resolver** — `resolveCoord` maps any integer coordinate to a
  valid array index (or `None` for `Zero` policy).

- **Catmull-Rom kernel** — `catmullRom` weight function used by the bicubic sampler.

- **Sampling pipeline**
  - `sampleNearest` — byte-exact nearest-neighbour lookup
  - `sampleBilinear` — 2×2 bilinear blend in linear light
  - `sampleBicubic`  — 4×4 Catmull-Rom blend in linear light
  - `doSample`       — dispatcher selecting the active sampler

- **Lossless transforms**
  - `flipHorizontal`  — mirror left-to-right
  - `flipVertical`    — mirror top-to-bottom
  - `rotate90CW`      — 90° clockwise rotation; W′ = H, H′ = W
  - `rotate90CCW`     — 90° counter-clockwise rotation; W′ = H, H′ = W
  - `rotate180`       — 180° rotation; dimensions unchanged
  - `crop`            — extract a rectangular region
  - `pad`             — add a filled border

- **Continuous transforms**
  - `scale`          — resize to arbitrary dimensions using pixel-centre model
  - `rotate`         — arbitrary-angle rotation with Fit/Crop canvas choice
  - `affine`         — 2×3 matrix inverse-mapped affine warp
  - `perspectiveWarp` — 3×3 homogeneous matrix perspective warp

- **Tests** — 35 xUnit `[<Fact>]` tests covering all public functions,
  all `OutOfBounds` modes, identity/double-application invariants, dimension
  checks, and pixel-value correctness.  Line coverage: 83.5% (threshold 80%).

- **BUILD** script for the repo build tool (dotnet test + coverage).
- **README.md** and **CHANGELOG.md**.
