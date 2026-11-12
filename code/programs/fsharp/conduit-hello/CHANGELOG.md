# Changelog — conduit-hello (F#)

## Unreleased

- Windows now builds conduit-capi with the shared helper (`code/scripts/stage-conduit-capi-windows.ps1`), points `CONDUIT_CAPI_PATH` at the staged `conduit_capi.dll` and runs the same `dotnet test` as the Unix recipe (new `tools/run-tests.ps1`), instead of skipping.

## 0.1.0 — 2026-06-14

Initial release (WEB16).

- Demo program for CodingAdventures.Conduit.FSharp.
- Routes: `/`, `/health`, `/api/greet/:name`, `/api/search`, `/api/echo`, `/old-home`, `/tpot`.
- Before-filter: API-key guard (opt-in bypass for development env — secure by default).
- After-hook: stamps `x-served-by` and `x-env` on every response.
- HTML-encode all server-controlled values embedded in the home-page template.
- Validates PORT env var; logs and defaults to 3000 on parse failure.
- 9 smoke tests verifying all routes and middleware.
