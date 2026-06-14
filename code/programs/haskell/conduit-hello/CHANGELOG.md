# Changelog — conduit-hello (Haskell)

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
