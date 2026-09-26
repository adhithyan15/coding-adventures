### Added — `--runtime-library` accepts an `.xcframework` for SwiftUI (UI89 §2.1)

- A SwiftUI package can link its Rust runtime statically: the directory (with
  its `Info.plist`) is copied to `Runtime/MosaicAppRuntime.xcframework`,
  `Package.swift` links it, and the app passes no bundled path. Only
  directories and regular files are copied; a symbolic link inside is refused.
  Other backends refuse an `.xcframework`.
- Proven end to end: Trestle built with `code/scripts/build-mosaic-xcframework.sh`
  compiles and links for the iOS Simulator (arm64 + x86_64) and iOS devices,
  and `nm` shows the engine in both binaries. CI does the same.

