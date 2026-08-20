# Changelog — image-codec-png (Go)

## 0.1.0 — 2026-08-20

- Added the bounded IC18 PNG encoder and decoder for 8-bit colour types 0, 2,
  4, and 6.
- Reused the repository ZIP package for CRC-32, raw RFC 1951 encoding, counted
  decoding, exact-consumption checks, and the 256 MiB hard inflate ceiling.
- Added all five PNG filters with the normative signed encoder heuristic and
  Paeth tie order.
- Added suggested-palette and transparency handling, split-IDAT support, exact
  chunk/zlib/Adler validation, APNG refusal, and stable payload-blind errors.
- Added caller-lowerable pixel limits and independent edge, product, and exact
  filtered-output allocation bounds.
- Added consumption of the complete 82-case language-neutral corpus, direct
  APNG refusal regressions, standard-library foreign decoding and zlib filter
  inspection, race testing, vet, build, and coverage gates.
- Added matching Unix and Windows build metadata plus an explicit empty
  capability declaration.
