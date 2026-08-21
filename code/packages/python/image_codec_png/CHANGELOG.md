# Changelog

## 0.1.0 - 2026-08-21

### Added

- Native Python IC18 PNG encoder, decoder, Adler-32 helper, `PngCodec`, fixed
  resource limits, and the ordered 29-code payload-blind `PngError` taxonomy.
- Exact chunk type, CRC, first-IHDR, APNG, PLTE/tRNS, consecutive-IDAT, IEND,
  zlib, counted-inflate, cavity, Adler, and filter validation precedence.
- Deterministic all-five-filter RGBA encoding through repository ZIP raw
  DEFLATE and CRC-32 primitives.
- Public consumption of all 85 language-neutral PNG cases, test-only zlib and
  Pillow interoperability, focused allocation and precedence regressions, and
  portable BUILD front doors with branch coverage, Ruff, and strict MyPy.
- Dependency-shaped Python ZIP hardening for large raw-DEFLATE input: bounded
  blocks, constant-size streaming match state, stored incompressible blocks,
  and no boxed per-byte LZSS token list.
- Empty production capability manifest for the pure in-memory transform.
