# Changelog

All notable changes to `markdown-docx` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-08

### Added

- Initial release — native **Markdown → `.docx`** (spec
  `code/specs/MD02-markdown-to-docx.md`).
- `markdown_to_docx(&str) -> Vec<u8>` (CommonMark) and `gfm_to_docx(&str) -> Vec<u8>`
  (GitHub-Flavored) — compose `commonmark-parser` / `gfm-parser` with
  `document-ast-to-docx::to_docx_bytes`, so Markdown flows through the shared
  Document AST into real WordprocessingML bytes with no external dependency.
- 8 end-to-end tests + a doctest: a rich Markdown document round-trips (text +
  block structure) through the `wordprocessingml` reader; GFM tables and task
  lists round-trip; fenced code blocks and blockquotes render; raw HTML embedded
  in Markdown is NOT injected into the `.docx` (XSS-safe); and a 20 000-level
  nested input converts without overflowing the stack (the depth guard holds
  end-to-end). `#![forbid(unsafe_code)]`, clippy-clean.
