# Changelog — barcode-1d (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for the sibling libraries) instead of an `echo ... skipping` recipe. The native Paint VM (Metal) and PNG codec stay macOS/Linux-only; on Windows `render_png` reports the documented "no native Paint VM is available for this host" error.

### Fixed

- `render_png` now renders first and loads `CodingAdventures::PaintCodecPngNative` only afterwards, so a host with no native backend reports the clean "no native Paint VM is available for this host" error instead of a DynaLoader "Can't locate loadable object" failure from the codec.

## 0.01 — 2026-04-13

Initial release.

- `build_scene` — dispatches Code 39, Codabar, Code 128, EAN-13, ITF, and UPC-A
- `render_pixels` — executes the native Paint VM for the current host
- `render_png` — wraps native PNG encoding
