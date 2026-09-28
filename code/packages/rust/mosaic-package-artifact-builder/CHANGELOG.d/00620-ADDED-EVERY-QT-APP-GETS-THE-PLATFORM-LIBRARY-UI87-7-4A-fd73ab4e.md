### Added — every Qt app gets the platform library (UI87 §7.4a)

- `MosaicPlatformEffects.{h,cpp}` are written beside `MosaicHost.{h,cpp}` and
  added to the same `target_sources` line, so they compile under the host's
  guard (`qt_cmake_with_platform_effects` refuses a CMakeLists without exactly
  one such line).
- `main.cpp` installs `installMosaicPlatformEffects(mosaicHost, <kinds or
  std::nullopt>)` after the package's handler (or after the host when there is
  none), and includes the header inside `#if MOSAIC_HAS_HOST` when that shape
  is emitted.
- A package whose `[host_assets]` replaces `MosaicHost.h/.cpp` (Venture) keeps
  its own host and gets no platform library.

