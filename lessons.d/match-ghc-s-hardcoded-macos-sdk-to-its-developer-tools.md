---
category: Haskell
---

# Match GHC's hardcoded macOS SDK to its developer tools

GHC 9.4.8 on the macOS 26 arm64 runner retained a Command Line Tools SDK
sysroot even when its parent build step exported an Xcode developer directory
and `SDKROOT`. That mixed the CLT 27 stubs with an older Xcode linker, which
rejected the stubs' `arm64e.x1` targets. Selecting an older SDK in the parent
environment therefore did not change GHC's actual link command.

For each independently selected Cabal root, scope
`DEVELOPER_DIR=/Library/Developer/CommandLineTools` to its Darwin Cabal process.
This pairs GHC's configured CLT sysroot with the matching CLT linker while
leaving other platforms and later Xcode builds unchanged. Test the routing and
failure propagation locally; only native macOS CI proves the linker pairing.
