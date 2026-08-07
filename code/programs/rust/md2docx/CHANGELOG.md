# Changelog

All notable changes to `md2docx` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-08

### Added

- Initial release — the CLI end-goal of the Markdown → `.docx` pipeline (MD02).
- `md2docx <in.md> [out.docx]` converts a Markdown file to a real `.docx`
  (default output: the input with a `.docx` extension); `--gfm` selects the
  GitHub-Flavored parser (pipe tables, task lists, strikethrough); `--demo`
  converts a bundled sample; `--help` prints usage.
- A testable `convert(&str, Dialect) -> Vec<u8>` library core over the
  `markdown-docx` crate, with tests asserting both dialects produce a valid
  `.docx` and that GFM recognizes a pipe table CommonMark leaves as text.
  `#![forbid(unsafe_code)]`, clippy-clean.
