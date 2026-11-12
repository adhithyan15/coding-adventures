# Changelog — conduit-hello (Haskell)

## Unreleased

### Changed

- Windows now builds conduit-capi with the shared
  `code/scripts/stage-conduit-capi-windows.ps1` helper and runs the Cabal tests
  through `tools\run-tests.ps1`, instead of skipping. `BUILD_windows` was
  replaced to call the script; `tools/run-tests.ps1` stages the DLL and import
  library in a conduit-hello-specific directory.

### Fixed

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 — 2026-06-14

Initial release (WEB18).

### Added
- `src/Main.hs` — 8-route demo: root, `/hello/:name`, POST `/echo`,
  `/search?q=`, `/redirect`, `/halt`, `/down` (blocked by before-filter),
  `/error` (routed through on_error), custom notFound + onError handlers,
  `X-Powered-By` after-hook.
- `test/SmokeSpec.hs` — 6 smoke tests exercising the routes over raw
  HTTP/1.0 sockets using the `hspec` + `network` libraries.
- `cabal.project` — references both this package and
  `../../packages/haskell/conduit` so cabal resolves the local library.
- `BUILD` / `BUILD_windows` / `required_capabilities.json`.
- `tools/run-tests.sh` — builds conduit-capi, then runs `cabal test`.
