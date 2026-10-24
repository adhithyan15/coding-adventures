# Changelog

All notable changes to `vscode-lang-extension-generator` are documented
in this file.

## Unreleased

- Rewrite `BUILD_windows` for `cmd.exe`. The build tool runs every line
  through `cmd /C`, but the file was written as a PowerShell script, so its
  first line (`$ErrorActionPreference = 'Stop'`) failed and the package could
  never build on Windows. It now installs its `file:` deps with
  `npm install --prefix`, the same pattern as `language-ladder`.

## 0.2.0 - Spec-driven flow

- Added `--language-spec <path>` flag that consumes the JSON document
  produced by each language's `<lang>-spec-dump` binary (twig-spec-dump,
  etc.).  The generator now reads `languageId`, `languageName`,
  `fileExtensions`, `keywords`, `lineComment`, and `blockComment`
  straight from the compiled lexer/parser rlibs — no manual flag-passing
  needed when a spec is available.
- All other CLI flags became optional overrides when `--language-spec`
  is set.
- Hardened security: validators reject quotes, backticks, backslashes,
  newlines, `${}` substitution markers in `languageName`/`description`;
  validators restrict `lspBinary`/`dapBinary` to alphanumerics + `_./-`.
  All user-controlled values flow through `JSON.stringify` before
  embedding in generated TypeScript source (defense-in-depth).  TOCTOU
  on writes converted to noisy errors via `flag: "wx"`.
- Generated BUILD scripts are POSIX-compatible (no `set -euo pipefail`,
  no bash shebang) so they run cleanly under dash on Ubuntu CI.
- 94 unit + smoke + integration tests; ~98% coverage.

## 0.1.0 - Initial release

- Generates VS Code extensions wrapping LSP and/or DAP servers for
  any language built on the LANG-VM pipeline.
- Both capabilities ship in one extension (the rust-analyzer / Pylance
  pattern) so users install one thing.
- LSP-only, DAP-only, and combined configurations all supported.
- Optional minimal TextMate grammar from a `--keywords` list.
- Deterministic output: identical inputs produce byte-identical files.
- Spec: `code/specs/LS04-vscode-extension-generator.md`.
