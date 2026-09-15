### Added — IC18: PNG Encoder and Decoder
- `image-codec-png` turns a `PixelContainer` into a real `.png` and back:
  chunk framing with CRC-32, the RFC 1950 zlib wrapper with Adler-32, and all
  five RFC 2083 scanline filters with per-row selection by the PNG spec's own
  minimum-sum-of-signed-bytes heuristic.
- The hard layer is not in the package. RFC 1951 DEFLATE and PNG's CRC-32 both
  come from `zip`, because the bit stream inside `IDAT` is the one inside a ZIP
  entry and the polynomial is the same. A second copy would be a second place
  for the same class of bug to hide.
- Encodes 8-bit truecolour with alpha, which is exactly what a `PixelContainer`
  holds, so the round trip is lossless by construction. Decodes 8-bit colour
  types 0, 2, 4 and 6, any number of `IDAT` chunks, skipping unknown ancillary
  chunks and refusing unknown critical ones as the spec requires. Palette
  images, 16-bit depths and Adam7 interlacing are refused by name.
- Conformance is tested against foreign implementations, not just against
  itself: Node's zlib inflates our `IDAT`, and the decoder reads PNGs assembled
  by hand from RFC 2083. The output was confirmed readable by `file`, macOS
  `sips` and Python's `zlib`.
- Refuses four shapes of file that decode to exactly the right image while
  carrying bytes the image does not need: a payload inside `IEND`, anything
  after `IEND`, a chunk wedged between two `IDAT`s, and the `IDAT` cavity —
  the dead space between where the DEFLATE stream announces its own end and
  where the Adler-32 begins. The picture is identical either way, which is
  exactly why tolerating them makes a valid-looking PNG into free carriage.
- Caps the total pixel count, not just each edge: 16384 x 16384 passes an edge
  cap and is 268 million pixels, roughly 3 GiB of peak allocation for about a
  megabyte of input. BMP survives on an edge cap because its pixels have to be
  in the file; PNG amplifies, and that is the whole difference.
- `zip` gains `rawInflateCounted`, which reports how many input bytes a stream
  actually used. Without it the `IDAT` cavity is undetectable.
- Adds `IC18-image-codec-png.md`. PNG had fallen out of the IC series
  entirely: IC00's roadmap reserved IC04 for it, that number went to JPEG, and
  the table was never corrected — while IC08 (ICO) still names PNG as a
  dependency for its 256x256 frames. IC00's roadmap now matches the specs that
  exist.

