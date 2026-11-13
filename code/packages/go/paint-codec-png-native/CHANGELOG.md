# Changelog

## Unreleased

- Runs for real on Linux, Windows (amd64, MinGW-w64 cgo) and macOS amd64 as well
  as macOS arm64; it used to link and test only on macOS arm64 and echo-skipped
  everywhere else (BUILD-002 W3).
- One cgo file (`paint_codec_png_native_cgo.go`, tag
  `cgo && (linux || darwin || (windows && amd64))`) with per-OS
  `#cgo <os> LDFLAGS` lines replaces `paint_codec_png_native_darwin_arm64.go`.
  macOS keeps its existing `-L... -lpaint_codec_png_c` flags (now also used for
  amd64, unverified locally); Linux links `libpaint_codec_png_c.a` by path with
  the libraries `rustc --print native-static-libs` reports; Windows links the
  `x86_64-pc-windows-gnu` staticlib by path.
- `SupportedRuntime()` / `Available()` now report a single `nativeLinked` constant
  defined next to the build tags (true in the cgo file, false in the stub, which
  also covers `CGO_ENABLED=0`) instead of a duplicated `GOOS/GOARCH` check.
- `Encode` validates in Go (nil, inconsistent, empty) before the C call, so the
  stub and native builds reject bad input identically.
- Tests are no longer vacuous: where the codec is linked they assert the PNG
  signature, the IHDR dimensions, a pixel-exact round trip through `image/png`,
  and result ownership; the nil/inconsistent/empty error paths run on every host.
- `BUILD` no longer gates on `uname`; it declares `deps=rust/paint-codec-png-c`.
  `BUILD_windows` runs the new `tools\run-tests.ps1`.
- `code/scripts/build-conduit-capi-windows-gnu.ps1` gained `-CrateDir` and
  `-ArchiveName` (defaults keep go/conduit unchanged) so it can build this crate's
  GNU staticlib too.

## 0.1.0

- Initial Go wrapper over the Rust PNG paint codec C ABI
