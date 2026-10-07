# Changelog

## [1.0.0] - 2026-10-06

### Added

- Closed v1 case and grammar-manifest schema with 132 SHA-256-pinned canonical
  grammar files across 12 language families and 66 editions.
- One small, test-backed successful source per family, with exact ordered
  tokens and complete generic AST expectations.
- Bounded corpus and grammar-drift validator, negative contract tests, and a
  Python 3.12+ canonical-engine replay gate for every expected token and AST.
