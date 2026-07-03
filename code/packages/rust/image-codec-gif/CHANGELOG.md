# Changelog — image-codec-gif

All notable changes to this crate follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format.
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-05-26

### Added

- **`decode_gif(bytes)`** — Decodes GIF87a and GIF89a byte streams into RGBA8
  `PixelContainer` buffers.  Handles:
  - Global and Local Color Tables
  - Graphic Control Extension (transparency flag + transparent colour index)
  - 4-pass de-interlacing (pass order: rows 0/8/…, 4/12/…, 2/6/…, 1/3/…)
  - Unknown extension blocks (skipped gracefully)
  - Animated GIF detection (second Image Descriptor → `Err`)

- **`encode_gif(pixels)`** — Encodes a `PixelContainer` as a GIF byte stream.
  - Outputs GIF87a for fully opaque images; GIF89a for images with transparency.
  - Exact palette when ≤ 256 distinct opaque colours are present.
  - Median-cut quantisation (256 buckets) when > 256 colours.
  - One palette slot is reserved for the transparent colour (GIF89a only).
  - Non-interlaced progressive scan.

- **`GifCodec`** — Zero-field struct implementing `paint_instructions::ImageCodec`
  (`mime_type() → "image/gif"`, `encode`, `decode`).

- **`lzw` module** — GIF-variant LZW encoder + decoder.
  - Configurable `lzw_minimum_code_size` (2–8 bits).
  - LSB-first bit packing (`BitWriter`, `BitReader`).
  - Sub-block framing (`to_sub_blocks`, `read_sub_blocks`).
  - Code-width growth: decoder grows one step before the encoder (at
    `next_code >= 2^code_size − 1`) to compensate for the one-entry lag
    inherent in LZW decoding.
  - Table-full (`next_code ≥ 4096`) CLEAR + reset in encoder.

- **42 unit tests + 1 doc-test** covering:
  - LZW round-trips at all `min_code_size` values (2–8)
  - RLE and gradient patterns; 256-colour full palette
  - Sub-block framing invariants
  - Solid, gradient, transparent, and mixed-alpha images
  - File structure (header, trailer, canvas dimensions in LSD)
  - Error paths: bad magic, unknown version, truncated data

### Implementation notes

- Palette quantisation uses squared Euclidean distance for nearest-colour
  assignment (not perceptually weighted) — sufficient for the lossless
  indexed-colour domain.
- Median-cut splits on the channel with the greatest min/max range; bucket
  representative is the arithmetic mean.
- The LZW decoder uses a parent-pointer table (no per-entry `Vec<u8>`) to
  reconstruct code strings; worst-case stack depth is 4096 entries.
