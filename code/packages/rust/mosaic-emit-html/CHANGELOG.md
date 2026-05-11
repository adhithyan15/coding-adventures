# Changelog — mosaic-emit-html

## 0.1.0 — 2026-05-11

Initial release.

- Implement `HtmlRenderer` — pure HTML static snapshot backend for the Mosaic compiler.
- Implements `MosaicRenderer` trait; driven by `MosaicVM`.
- Fixture JSON support: slot values resolved from `serde_json::Map` at compile time.
- All Mosaic primitives mapped to HTML elements with inline `style=""` attributes:
  Box, Column, Row, Text, Image, Spacer, Scroll, Divider, Stack, Icon, Grid.
- `when` blocks: compile-time suppression based on fixture boolean value.
  Missing fixture defaults to `true` (show content for design review).
- `each` blocks: first fixture array element substituted for loop variable (v1).
- Grid → static `<table>` with `<thead>`/`<tbody>` populated from fixture arrays.
- HTML escaping via `html_escape()` for all user-provided slot values.
- CSS inlining: optional CSS string placed in `<style>` block; falls back to
  a minimal reset (`box-sizing: border-box; body margin: 0; font-family: sans-serif`).
- Full `<!DOCTYPE html>` document output with `<html lang="en">`, `<head>`, `<title>`, `<body>`.
- 17 unit tests, all passing.
