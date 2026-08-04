# Changelog

All notable changes to the `image-codec-ppm` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — Netpbm PPM (P6) encoder/decoder** (CCPP02 port campaign,
  bucket A / pure-ISO, port #4). The C port of the Rust `image-codec-ppm` crate:
  the simplest real image format — an ASCII header (`P6\n<w> <h>\n255\n`) then raw
  RGB. A pure-ISO crate (no OS), so it rides the `iso-harness` (links nothing,
  `-pedantic-errors` / `/permissive-`).
  - **API.** `ppm_mime_type` (`image/x-portable-pixmap`); `ppm_encode` (container
    → fresh PPM byte buffer, released with `ppm_free`); `ppm_decode` (PPM bytes →
    fresh `PixelContainer`, released with `pixel_free`); `ppm_free`. `ppm_status`
    error enum replaces the Rust `Result<_, String>`.
  - **Composes `c/pixel-container`.** Pixels are an RGBA8 `PixelContainer`; PPM
    has no alpha, so encode drops the alpha byte and decode restores it to 255.
    `run.sh` compiles pixel-container's source in; nothing is linked. `BUILD`
    declares `deps=c/iso-harness c/pixel-container`.
  - **Faithfulness.** Header parser is whitespace/`#`-comment-aware with the same
    whitespace set as Rust's `is_ascii_whitespace` (space/`\t`/`\n`/`\r`/`\f`, not
    `\v`), consumes exactly one whitespace byte before the raster, and accepts
    only maxval 255. `read_int` fails on non-digits and on `size_t` overflow (the
    Rust `parse::<usize>()` → None).
  - **Safety (untrusted decoder input).** Every pixel read is bounds-checked
    (`len - pos >= width*height*3`, no unsigned underflow), every
    `width*height*{3,4}` size is `size_t`-overflow guarded (Rust's `checked_mul`)
    → `PPM_ERR_OVERFLOW`, and dimensions beyond `UINT32_MAX` are rejected before
    the cast into the container. An adversarial security review confirmed no
    out-of-bounds access, overflow, or error-path leak.
  - **Test (`tests/ppm_codec_test.c`).** The Rust tests (P6 header, dimensions in
    header, exact encoded size, alpha dropped, solid- and multi-colour
    round-trips, comment in header, wrong magic, unsupported maxval, truncated
    data, MIME type) plus an extra-whitespace header and the NULL-argument paths.
    121 checks, verified under gcc + clang with `-pedantic-errors`, clean under
    ASan+UBSan, 0 leaks.
