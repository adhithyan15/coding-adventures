# Changelog

All notable changes to `vscode-lang-extension-generator` are documented
in this file.

## 0.1.0 - Initial release

- Generates VS Code extensions wrapping LSP and/or DAP servers for
  any language built on the LANG-VM pipeline.
- Both capabilities ship in one extension (the rust-analyzer / Pylance
  pattern) so users install one thing.
- LSP-only, DAP-only, and combined configurations all supported.
- Optional minimal TextMate grammar from a `--keywords` list.
- Deterministic output: identical inputs produce byte-identical files.
- 68 unit + smoke tests; ~98% coverage.
- Spec: `code/specs/LS04-vscode-extension-generator.md`.
