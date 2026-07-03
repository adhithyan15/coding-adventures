# Changelog — conduit-hello (F#)

## 0.1.0 — 2026-06-14

Initial release (WEB16).

- Demo program for CodingAdventures.Conduit.FSharp.
- Routes: `/`, `/health`, `/api/greet/:name`, `/api/search`, `/api/echo`, `/old-home`, `/tpot`.
- Before-filter: API-key guard (opt-in bypass for development env — secure by default).
- After-hook: stamps `x-served-by` and `x-env` on every response.
- HTML-encode all server-controlled values embedded in the home-page template.
- Validates PORT env var; logs and defaults to 3000 on parse failure.
- 9 smoke tests verifying all routes and middleware.
