# Changelog — image-convert

All notable changes to this program are documented here.

---

## [0.1.0] — 2026-05-30

### Added

- **`image-convert` CLI** — universal image format converter routing through
  RGBA8 `PixelContainer` as the intermediate representation.

- **Format detection** (`detect.rs`):
  - Magic byte detection for: PNG, JPEG, GIF (87a/89a), WebP (RIFF+WEBP),
    JPEG XL (naked `\xFF\x0A` and ISOBMFF `ftypJXL`), QOI (`qoif`),
    Fujifilm RAF (`FUJIFILMCCD-RAW `), Panasonic RW2 (`II\x55\x00`),
    ICO (`\x00\x00\x01\x00`), BMP (`BM`), PPM (`P2`/`P3`/`P5`/`P6`),
    Canon CR2 (TIFF + `CR\x02` at offset 8), Olympus ORF (`IIRO`).
  - Extension fallback for TIFF-family formats that share magic bytes:
    `.tiff` → TIFF, `.dng` → DNG, `.nef` → NEF, `.arw` → ARW, `.orf` → ORF.
  - Magic bytes override incorrect extensions.

- **Decode dispatch** (`codecs.rs`):
  - 17 input formats: PNG, BMP, PPM, QOI, JPEG, WebP, JXL, GIF, ICO, TIFF,
    DNG, CR2, NEF, ARW, RAF, ORF, RW2.

- **Encode dispatch** (`codecs.rs`):
  - 10 output formats: PNG, BMP, PPM, QOI, JPEG, WebP, JXL, GIF, ICO, TIFF.
  - RAW formats rejected with a clear error message.
  - Alpha compositing over white for JPEG/BMP/PPM output.

- **CLI** (`main.rs`):
  - Positional `<INPUT>` and `<OUTPUT>` arguments.
  - `-q / --quality <N>` for lossy encode quality (default: 85).
  - `--from <FORMAT>` / `--to <FORMAT>` to override auto-detection.
  - `--list-formats` to display all supported formats.
  - Atomic output: writes to `<output>.tmp` then renames on success.
  - Informative exit codes (0–6) and `stderr` progress messages.
  - 512 MB input size guard.

- **45 unit tests**: magic byte detection for 13 formats, extension detection,
  combined detection (magic beats extension), round-trips for PNG/BMP/TIFF/
  QOI/PPM/ICO, alpha compositing (opaque/transparent/half), RAW encode rejection,
  `pixels_to_rgba` ordering, `list_formats` content.

[0.1.0]: https://github.com/adhithyan15/coding-adventures/tree/feat/ic17-image-convert
