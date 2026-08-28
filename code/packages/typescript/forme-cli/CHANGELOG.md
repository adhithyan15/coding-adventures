# Changelog — @coding-adventures/forme-cli

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
