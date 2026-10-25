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
- A per-render aggregate URL-input work budget that charges repeated aliases
  and rejected destinations across links, images, and autolinks.
- A per-render aggregate table-alignment budget that stops oversized lists and
  charges repeated immutable aliases before array allocation.
- Native xUnit coverage plus Unix and Windows repository build fronts.
- Clean-checkout build fronts prebuild the full project graph before Coverlet
  instrumentation so project-reference copy ordering cannot report false 0%
  coverage.
