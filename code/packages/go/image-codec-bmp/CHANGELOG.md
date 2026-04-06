# Changelog — image-codec-bmp (Go)

## 0.1.0 — 2026-04-05

Initial release.

- `BmpCodec` struct implementing `pixelcontainer.ImageCodec`
- `EncodeBmp` — 32-bit BGRA BMP encoder with top-down layout (negative biHeight)
- `DecodeBmp` — 32-bit BI_RGB decoder; handles top-down and bottom-up files
- `IsBmp` — magic-number detection
- `LookupByMime` — codec registry keyed by MIME type
- 23 unit tests, >95% coverage
