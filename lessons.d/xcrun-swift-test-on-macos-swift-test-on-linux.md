---
category: Cross-platform & Windows BUILD_windows
---

# `xcrun swift test` on macOS, `swift test` on Linux

Bare `swift test` on macOS CI fails to find XCTest framework (lives in Xcode bundle). Make BUILD platform-aware: `if command -v xcrun >/dev/null 2>&1; then xcrun swift test; else swift test; fi`. Swift on Windows requires `winget install Swift.Toolchain` in the workflow.
