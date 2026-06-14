# IC17 — image-convert: Universal Image Format Converter

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00–IC16 (all image codecs)  
**Implements**: A pandoc-style command-line image format converter

---

## 1. Overview

`image-convert` is a command-line program that converts images between any two
supported formats by routing through the shared `PixelContainer` (RGBA8)
intermediate representation. It is the capstone application of the IC series:
every image codec implemented in IC01–IC16 is wired into a single tool.

```
input file → detect format → decode → PixelContainer → encode → output file
```

The design principle is **simplicity over fidelity**: the converter is not a
professional image processing pipeline. It is a teaching tool that shows how a
unified pixel abstraction enables format interoperability with minimal glue code.

---

## 2. Supported Formats

### 2.1 Input formats (decode)

| Extension(s)    | Format          | Magic bytes                        | Crate               |
|-----------------|-----------------|------------------------------------|---------------------|
| .png            | PNG             | `\x89PNG\r\n\x1a\n`               | `png`               |
| .bmp            | BMP             | `BM`                               | `image-codec-bmp`   |
| .ppm/.pgm       | PPM/PGM         | `P6`/`P5`/`P3`/`P2`               | `image-codec-ppm`   |
| .qoi            | QOI             | `qoif`                             | `image-codec-qoi`   |
| .jpg/.jpeg      | JPEG            | `\xFF\xD8\xFF`                     | `image-codec-jpeg`  |
| .webp           | WebP            | `RIFF????WEBP`                     | `image-codec-webp`  |
| .jxl            | JPEG XL         | `\xFF\x0A` or ISOBMFF box         | `image-codec-jxl`   |
| .gif            | GIF             | `GIF87a`/`GIF89a`                  | `image-codec-gif`   |
| .ico/.cur       | ICO/CUR         | `\x00\x00\x01\x00`                | `image-codec-ico`   |
| .tif/.tiff      | TIFF            | `II\x2A\x00`/`MM\x00\x2A`         | `image-codec-tiff`  |
| .dng            | Adobe DNG       | TIFF + DNG tags                    | `image-codec-dng`   |
| .cr2            | Canon CR2       | TIFF + `CR\x02` at offset 8       | `image-codec-cr2`   |
| .nef            | Nikon NEF       | TIFF + Make=NIKON                  | `image-codec-nef`   |
| .arw            | Sony ARW        | TIFF + Make=SONY                   | `image-codec-arw`   |
| .raf            | Fujifilm RAF    | `FUJIFILMCCD-RAW `                 | `image-codec-raf`   |
| .orf            | Olympus ORF     | `II` + TIFF or `IIRO`             | `image-codec-orf`   |
| .rw2            | Panasonic RW2   | `II\x55\x00`                       | `image-codec-rw2`   |

### 2.2 Output formats (encode)

| Extension(s)    | Format   | Notes                                     |
|-----------------|----------|-------------------------------------------|
| .png            | PNG      | Lossless, best general-purpose output     |
| .bmp            | BMP      | Uncompressed, large files                 |
| .ppm            | PPM      | Plain RGB, no compression, for debugging  |
| .qoi            | QOI      | Fast lossless, smaller than BMP           |
| .jpg/.jpeg      | JPEG     | Lossy, quality 1–100 (default: 85)        |
| .webp           | WebP     | Lossless VP8L; lossy VP8 with quality     |
| .jxl            | JPEG XL  | Lossless modular                          |
| .gif            | GIF      | 256-colour palette, no animation          |
| .ico            | ICO      | 32bpp BGRA, single frame                  |
| .tif/.tiff      | TIFF     | Uncompressed RGB                          |

RAW formats (DNG/CR2/NEF/ARW/RAF/ORF/RW2) are **input-only** — re-encoding to
a proprietary RAW container is not meaningful.

---

## 3. CLI Interface

```
image-convert [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>    Path to input image file ('-' for stdin)
  <OUTPUT>   Path to output image file ('-' for stdout)

Options:
  -q, --quality <N>     Encode quality 1–100 (JPEG/WebP lossy; default: 85)
  --from <FORMAT>       Force input format (skip auto-detection)
  --to <FORMAT>         Force output format (skip extension detection)
  --list-formats        Print all supported input and output formats and exit
  -h, --help            Print help
  -V, --version         Print version (0.1.0)
```

### 3.1 Examples

```bash
# Develop a camera RAW to PNG
image-convert photo.nef photo.png
image-convert photo.cr2 photo.tiff
image-convert photo.raf photo.jpg --quality 90

# Convert between standard formats
image-convert logo.bmp logo.png
image-convert banner.gif banner.webp
image-convert icon.ico icon.png

# Force format (useful when extension is wrong)
image-convert data.bin output.png --from jpeg --to png

# List all supported formats
image-convert --list-formats
```

### 3.2 Exit codes

| Code | Meaning                                          |
|------|--------------------------------------------------|
| 0    | Success                                          |
| 1    | Input file not found or unreadable               |
| 2    | Format detection failed (unknown input format)   |
| 3    | Output format not supported for encoding         |
| 4    | Decode error (corrupted / unsupported variant)   |
| 5    | Encode error                                     |
| 6    | Output file write error                          |

---

## 4. Format Detection

Detection priority:

1. **`--from` flag**: use the specified format name, no magic check
2. **Magic bytes**: read the first 16 bytes of the file and match against the
   magic byte table (§2.1). This handles files with wrong extensions.
3. **Extension fallback**: if magic bytes are ambiguous or the file is shorter
   than 16 bytes, fall back to the file extension.

### 4.1 Magic byte lookup order

Some formats share prefixes (TIFF/DNG/CR2/NEF/ARW/ORF). For TIFF-family files,
read deeper into the file to distinguish them:

```
bytes[0..2] == "II" or "MM", bytes[2..4] == 42 → TIFF family
  then check:
    bytes[8..10] == "CR" AND bytes[10] == 2 → CR2
    Make tag contains "NIKON"               → NEF
    Make tag contains "SONY"                → ARW
    Make tag contains "OLYMPUS"             → ORF
    DNG version tag (50706) present         → DNG
    else                                    → TIFF (generic)

For simplicity, if the TIFF-family discrimination is slow or the file lacks
a Make tag (synthetic files), prefer the extension.
```

For most use cases, magic bytes alone are sufficient (JPEG, PNG, WebP, GIF,
QOI, RAF, RW2, ICO all have unique magic).

---

## 5. Conversion Pipeline

```
┌───────────┐     detect      ┌──────────────────┐     decode     ┌─────────────────┐
│ input file │ ─────────────▶ │   ImageFormat    │ ─────────────▶ │ PixelContainer  │
└───────────┘                 └──────────────────┘                │  (RGBA8 canvas) │
                                                                   └────────┬────────┘
                                                                            │ encode
                                                                            ▼
                                                                   ┌─────────────────┐
                                                                   │  output format  │
                                                                   └────────┬────────┘
                                                                            │ write
                                                                            ▼
                                                                   ┌───────────┐
                                                                   │output file│
                                                                   └───────────┘
```

Alpha handling: all codecs produce RGBA8. Formats that don't support alpha
(JPEG, PPM, BMP) will silently discard the alpha channel (composite over
white background).

---

## 6. Program Layout

```
code/programs/rust/image-convert/
  Cargo.toml    (standalone [workspace]; deps on all IC codecs)
  BUILD         (cargo test -p image-convert -- --nocapture)
  README.md
  CHANGELOG.md
  src/
    lib.rs      (detect, decode, encode — all testable without filesystem)
    main.rs     (CLI: argument parsing, file I/O, exit codes)
    detect.rs   (ImageFormat enum + format detection logic)
    codecs.rs   (decode/encode dispatch to the right crate)
```

---

## 7. API (library surface for tests)

```rust
pub enum ImageFormat { Png, Bmp, Ppm, Qoi, Jpeg, WebP, Jxl, Gif, Ico, Tiff,
                       Dng, Cr2, Nef, Arw, Raf, Orf, Rw2 }

/// Detect the format of an image from its bytes and/or file extension.
/// `ext`: file extension without dot (e.g. "jpg"), case-insensitive.
pub fn detect_format(bytes: &[u8], ext: Option<&str>) -> Option<ImageFormat>;

/// Decode image bytes in the given format to an RGBA8 PixelContainer.
pub fn decode_image(bytes: &[u8], fmt: ImageFormat) -> Result<PixelContainer, String>;

/// Encode a PixelContainer to the given format.
/// `quality`: used for lossy formats (JPEG, WebP); ignored for lossless.
pub fn encode_image(pixels: &PixelContainer, fmt: ImageFormat, quality: u8)
    -> Result<Vec<u8>, String>;

/// Detect output format from a file path's extension.
pub fn format_from_path(path: &str) -> Option<ImageFormat>;

/// Whether a format supports encoding (output).
pub fn is_encodable(fmt: &ImageFormat) -> bool;

/// Human-readable format name.
pub fn format_name(fmt: &ImageFormat) -> &'static str;
```

---

## 8. Test Strategy (≥20 tests)

| Category                      | Tests |
|-------------------------------|-------|
| Magic byte detection          | 8     |
| Extension fallback detection  | 4     |
| Round-trip lossless (PNG/TIFF/BMP/QOI) | 4 |
| Decode + re-encode (JPEG output) | 2  |
| Error: unknown format         | 2     |
| Error: unrecognised extension | 1     |
| is_encodable (RAW = false)    | 2     |
| format_name                   | 1     |
| **Total**                     | **24**|

---

## 9. Security Constraints

- Maximum input file size: 512 MB (reject before decode)
- All decode errors are propagated as exit code 4 with a human-readable message
- Output files are written atomically (write to `.tmp`, rename on success) to
  avoid leaving corrupt partial outputs

---

## 10. References

- All IC00–IC16 codec specs in `code/specs/`
- pandoc (https://pandoc.org) — inspiration for the universal-converter design
