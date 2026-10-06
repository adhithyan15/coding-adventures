### Fixed — Flutter projects carry the macOS file-dialog entitlement (UI87 §7.7)

- Every Flutter project now carries the macOS runner's entitlements,
  `macos/Runner/DebugProfile.entitlements` and `Release.entitlements`:
  Flutter's own keys (the sandbox; JIT and the VM service's socket in debug)
  plus `com.apple.security.files.user-selected.read-write`. Without that key
  NSOpenPanel and NSSavePanel fail inside the sandbox, so every `files.open`
  and `files.save` on a sandboxed macOS build answered
  `failed { "the file dialog failed" }` (UI87 §7.7 known gap).
- Written before `flutter create --platforms=macos` makes the runner, which
  writes only the files a project lacks, so the runner keeps them.
  `user-selected` is the narrowest file entitlement: the app gets the one
  file the person picked, nothing else on disk.
- The native-complete Flutter shell test pins both files' keys exactly.
