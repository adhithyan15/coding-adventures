# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-24

### Added

- Pure F# TE00 Document AST to HTML rendering through public `render` and
  `renderWithOptions` APIs.
- Exact rendering for core block and inline nodes plus the table, task-item,
  and strikethrough node types already exposed by the F# AST package.
- HTML escaping, raw-format routing, dangerous URL-scheme rejection, and
  explicit raw-HTML suppression for untrusted input boundaries.
- Bounded depth, node count, URL normalization, and output size with no partial
  result returned after a resource-limit failure.
- Native xUnit coverage plus Unix and Windows repository build fronts.
