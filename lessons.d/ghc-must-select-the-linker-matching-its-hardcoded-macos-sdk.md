---
category: Haskell
---

# GHC must select the linker matching its hardcoded macOS SDK

PR #18183's macOS job used runner image macos-26-arm64 20261013.0424.1.
GHC 9.4.8 linked dependencies against Command Line Tools 27 SDK stubs using
the default older Xcode linker, which rejected `arm64e.x1`. No provider tests
ran. The same regression and a successful CLT selection are documented in
[runner-images #14940](https://github.com/actions/runner-images/issues/14940)
and [Acton #3283](https://github.com/actonlang/acton/pull/3283).

Select `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for this package's
Darwin Cabal command and children. This matches the linker to GHC's SDK without
sudo, global xcode-select changes, SDK editing, dependency caps or reduced
coverage. Keep the other platforms' inherited environment and propagate Cabal
failure. Shell routing tests prove those boundaries; only fresh native macOS
CI can establish that linking and the actual package tests pass.
