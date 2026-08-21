# IC18 portable PNG v1 fixtures

This directory is the language-neutral contract for `image-codec-png` under
IC18. It separates PNG interoperability evidence from any one implementation's
encoder/decoder round trip.

## Files

- `schema.json` closes the profile, limits, operations, expected results, and
  29 stable error identifiers.
- `cases.json` contains 85 deterministic vectors: valid decoding, foreign
  encoder interoperability, Adler-32, malformed input, and resource limits.
- `generate_cases.py` constructs stable stored/fixed streams, wraps a checked
  dynamic-Huffman vector, and reproduces `cases.json` exactly across zlib
  versions.

## Operations

- `decode` supplies complete PNG bytes and exact RGBA8 output.
- `decode-error` requires the exact portable error identifier and no partial
  image.
- `encode` supplies RGBA8 pixels. Compressed bytes are deliberately not pinned;
  a foreign PNG decoder must recover the pixels, the structural expectations
  must hold, and each normative row-filter choice is pinned.
- `encode-error` requires the exact portable encoder-input error identifier.
- `adler32` pins the zlib checksum independently.

Hex is lowercase and byte-aligned. Valid vectors cover colour types 0, 2, 4,
and 6; all five filters; stored, fixed, and dynamic DEFLATE blocks; split IDAT;
Paeth predictor branches and ties; suggested `PLTE`; `tRNS` transparency; and
an unknown ancillary chunk. Valid-CRC rejection vectors name APNG's `acTL`,
`fcTL`, and `fdAT` chunks. Rejection vectors cover every IC18 framing,
checksum, zlib, filter, exact-consumption, and caller-lowerable allocation
boundary.

## Oracle and security boundary

Python zlib independently decodes the success corpus but never chooses its
canonical compressed bytes. TypeScript
uses `pngjs` only in tests to prove that encoded files are accepted by a foreign
PNG implementation. Production consumers remain pure in-memory transforms and
must declare an empty capability profile.

The corpus caps each embedded PNG at 1 MiB and the complete JSON document at
256 KiB. `maxPixels` is a positive safe integer no larger than 33,554,432; a
caller may lower but never raise that ceiling. Error identifiers are stable and
do not contain attacker-controlled payload bytes.

Validate schema, regeneration, independent decoding, filter coverage, DEFLATE
block coverage, error coverage, and size bounds from the repository root:

```bash
python -m unittest discover -s code/scripts/tests -p test_image_codec_png_fixtures.py -v
```

Every future lane must consume `cases.json` through its public codec API. A
15-lane consumer registry belongs to the final PNG completion umbrella, after
the toolchain-shaped lane children merge.
