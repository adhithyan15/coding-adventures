# Changelog — image-codec-ppm (Go)

## 0.1.0 — 2026-04-05

Initial release.

- `PpmCodec` struct implementing `pixelcontainer.ImageCodec`
- `EncodePpm` — P6 binary PPM encoder; alpha channel dropped
- `DecodePpm` — P6 binary PPM decoder; comment lines skipped; A set to 255
- `IsPpm` — magic-number detection
- Byte-cursor parsing approach (no bufio dependency)
- 19 unit tests, >95% coverage
