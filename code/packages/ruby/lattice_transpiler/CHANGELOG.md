# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- `CodingAdventures::LatticeTranspiler.transpile(source, minified: false, indent: "  ")`
  — full Lattice-to-CSS pipeline: parse, transform, emit. Returns a CSS
  string (pretty-printed or minified depending on options).
- `minified: true` produces compact CSS with no whitespace between tokens
  and no newlines between rules.
- `indent:` accepts any string for indentation (default `"  "`, two spaces).
