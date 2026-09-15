### Added — Raw RFC 1951 DEFLATE, Exported
- `zip` now exports `rawDeflate` / `rawInflate`: the DEFLATE codec with no ZIP
  framing. The same bit stream sits inside `zlib`, `gzip`, and PNG's `IDAT`, so
  exporting it keeps those formats from each carrying a second copy of the same
  bit-packing code. The encoder is unchanged and byte-stable; only the reader grew.
- CMP09 records the export as optional-per-port, and states the asymmetry as a
  rule: an encoder may emit fixed blocks only, but a decoder must read all three.

