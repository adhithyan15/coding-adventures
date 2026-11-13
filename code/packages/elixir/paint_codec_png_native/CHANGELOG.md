# Changelog - CodingAdventures.PaintCodecPngNative

## [Unreleased]

### Changed

- `BUILD_windows` now builds the NIF and runs the Mix tests on Windows instead of skipping with `ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE`. The Windows entry point takes the BEAM's callback table and hands it to `erl-nif-bridge`; no `enif_*` import is needed. `available?/0` and `encode/1` now look for `paint_codec_png_native.dll` on Windows (they hardcoded `.so`, which would have reported the NIF as missing); behaviour on Linux and macOS is unchanged.
