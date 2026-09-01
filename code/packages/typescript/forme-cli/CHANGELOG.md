# Changelog — @coding-adventures/forme-cli

## 0.3.0 — 2026-09-01

- Wired `settings.cacheDir` to the filesystem cache backend so safe pure-stage
  outputs are reused across separate CLI processes.
- Refused project-root and outside-project cache paths before any cache access,
  matching the containment contract already enforced by `forme clean`.
- Added deterministic per-stage cache statistics to `--report` output for
  product-level incremental-build verification.

## 0.2.0 — 2026-08-31

- Added `forme watch` with declarative `--port` and `--debounce` options.
- Added recursive project watching with generated output, cache, dependency,
  and VCS trees excluded from rebuild notifications.
- Wired the orchestrator watch session to the in-memory Forme dev server so
  successful builds reload browsers and failed builds retain the last good site.
- Added CLI and project-watcher tests plus live landing/blog dogfood coverage.

## 0.1.0 — 2026-08-28

- Added `forme build` (`forme run` alias), `forme check`, and
  containment-checked `forme clean`.
- Added `--config`, automatic config discovery, and `--reproducible`.
- Added `--report` with deterministic output manifests and per-file hashes for
  post-build acceptance without duplicating artifact bytes.
- Added stable diagnostic formatting and documented exit codes.
- Declared the command surface in `forme.cli.json` and delegated routing,
  flag validation, fuzzy flag suggestions, help, and version output to the shared
  TypeScript `@coding-adventures/cli-builder` package.
- Added cooperative SIGINT cancellation with exit code 130.
- Added a portable Node/tsx npm launcher for TypeScript-first Forme packages.
- Added a centralized, deterministic local `file:` dependency bootstrap helper.
- Declared each dogfood site's complete local Forme dependency set to the
  monorepo scheduler so bootstrap installs cannot race dependency test runs.
- Kept recursive bootstrap in the standalone site scripts while making their
  scheduler recipes install only the site itself, so the sites remain safe to
  build in parallel without writing the same local dependency tree.
- Proved config loading and execution from a temporary project outside the
  repository tree.
