### Added — CI runs the Flutter platform library on macOS, Windows and Linux arm64

- CI gains a `Flutter platform library` job that runs the Flutter platform
  library's headless harness (`tests/flutter_platform_effects.rs`) on
  macOS arm64, Windows x64 and Linux arm64. Until now the harness ran on
  Linux x64 only, so the macOS and Linux arm64 `open(2)` flags, the macOS
  `stat` layout and the Windows save (`CreateFileW`, `FlushFileBuffers`,
  `MoveFileExW`) were pinned by tests but never executed. UI87 §7.7's
  known-gaps note becomes "Where the libc calls run".
- `MOSAIC_REQUIRE_DART`: when set, the harness test fails if `dart` is not
  on PATH instead of skipping, so a broken runner cannot pass silently.
