## 2026-11-02 — Haskell Darwin BUILDs must select the CLT linker

- New `validateHaskellDarwinLinker` (`internal/validator/haskell_darwin_linker.go`),
  run by `--validate-build-files` alongside the other BUILD rules. Every line of
  the BUILD file a `haskell` package resolves on macOS (`BUILD_mac`, then
  `BUILD_mac_and_linux`, then `BUILD`) must select
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` behind a
  `[ "$(uname -s)" = Darwin ]` check, either as an export prefix
  (`if ...; then export DEVELOPER_DIR=...; fi; <command>`) or as the inline
  assignment on the Darwin branch's Cabal command. GHC 9.4.8's macOS bindist
  hardcodes a CLT SDK sysroot; on runners with CLT 27 beside an older default
  Xcode, an unselected Cabal process links with the Xcode linker, which rejects
  the SDK's `arm64e.x1` stubs. Cabal builds the whole local project in the
  selected root's process, so every root needs the selection, and every line
  needs it because each line is its own `sh -c`. The unsupported-protocol
  record and bare `echo` lines are exempt; comments never satisfy the rule.
  The error names the file and
  `lessons.d/ghc-must-select-the-linker-matching-its-hardcoded-macos-sdk.md`.
