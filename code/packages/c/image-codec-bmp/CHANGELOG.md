# Changelog

All notable changes to the `image-codec-bmp` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — 32-bit BGRA BMP encoder/decoder** (CCPP02 port campaign,
  bucket A / pure-ISO, port #5; the PPM codec's sibling). The C port of the Rust
  `image-codec-bmp` crate: a fixed 54-byte header (BITMAPFILEHEADER +
  BITMAPINFOHEADER) then a BGRA raster. A pure-ISO crate (no OS), so it rides the
  `iso-harness` (links nothing, `-pedantic-errors` / `/permissive-`).
  - **API.** `bmp_mime_type` (`image/bmp`); `bmp_encode` (container → fresh BMP
    byte buffer, released with `bmp_free`); `bmp_decode` (BMP bytes → fresh
    `PixelContainer`, released with `pixel_free`); `bmp_free`. A `bmp_status`
    error enum replaces the Rust `Result<_, String>`.
  - **Composes `c/pixel-container`.** Pixels are an RGBA8 `PixelContainer`; BMP is
    BGRA, so encode/decode swap R and B per pixel. `run.sh` compiles
    pixel-container's source in; nothing is linked. `BUILD` declares
    `deps=c/iso-harness c/pixel-container`.
  - **Format.** Encodes top-down (negative `biHeight`, no row reversal); decodes
    both top-down and bottom-up. 32-bit `BI_RGB` only (`biBitCount == 32`,
    `biCompression == 0`), matching the Rust.
  - **Faithfulness.** Portable little-endian reads; a dedicated `read_i32`
    reconstructs signed `biWidth`/`biHeight` by explicit two's-complement (no
    implementation-defined unsigned→signed cast) and rejects `i32::MIN` height
    exactly as the Rust does.
  - **Safety (untrusted decoder input).** Every header field lives in the
    validated 54-byte prefix; every raster byte is inside the checked
    `pixel_offset + width*height*4 <= len` window; every `width*height*4` /
    `offset+size` is `size_t`-overflow guarded → `BMP_ERR_OVERFLOW`. An
    adversarial security review confirmed no out-of-bounds access, overflow, or
    error-path leak.
  - **Test (`tests/bmp_codec_test.c`).** The Rust tests (magic, file size, pixel
    offset, negative biHeight, bit count, BGRA byte order, solid/checkerboard/
    transparency round-trips, too-short / wrong-magic / unsupported-bit-depth
    errors, MIME) plus a bottom-up-layout decode, more decode errors
    (compression / offset / width / truncation), and the NULL-argument paths. 64
    checks, verified under gcc + clang with `-pedantic-errors`, clean under
    ASan+UBSan, 0 leaks.
