# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-06

### Added

- 6LoWPAN dispatch classification for IPv6, HC1, IPHC, mesh, broadcast, and
  fragmentation headers.
- Mesh header parse/encode helpers for hop limits and short/extended
  originator/final addresses.
- LOWPAN_IPHC first/second byte parsing plus fragment first/next header
  parse/encode helpers.
- LOWPAN_NHC UDP compressed port and checksum-elision parse/encode helpers.
- Fragment payload parsing plus deterministic tag/size-keyed reassembly
  buffers and tables with overlap and bounds validation.
