---
category: CI & GitHub Actions
---

# Pin the macOS SDK selected for older Haskell toolchains instead of inheriting Command Line Tools from the moving runner image.

The `macos-latest` arm64 image moved its Command Line Tools SDK to macOS 27 while the repository still intentionally uses GHC 9.4.8. GHC's bundled linker could not parse the SDK's new `arm64e.x1` target entries, so every selected Haskell package failed before its own code linked. When a language toolchain is older than the runner image, select an installed reviewed Xcode developer directory and assert the exact SDK basename before setup. Export both `DEVELOPER_DIR` and `SDKROOT` through `GITHUB_ENV`; do not rely on the runner's mutable `xcode-select` or Command Line Tools default.
