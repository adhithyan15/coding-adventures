---
category: CI & GitHub Actions
---

# Pin the macOS SDK selected for older Haskell toolchains instead of inheriting Command Line Tools from the moving runner image.

The `macos-latest` arm64 image moved its Command Line Tools SDK to macOS 27 while the repository still intentionally uses GHC 9.4.8. GHC's bundled linker could not parse the SDK's new `arm64e.x1` target entries, so every selected Haskell package failed before its own code linked. When a language toolchain is older than the runner image, select an installed reviewed Xcode developer directory and assert the exact SDK basename. Publish the paths as step outputs and export `DEVELOPER_DIR` plus `SDKROOT` only inside the build steps that compile Haskell; a job-wide `GITHUB_ENV` override can hide mobile platforms installed only in the runner's default Xcode from later `xcodebuild` steps. Do not rely on the runner's mutable `xcode-select` or Command Line Tools default.
