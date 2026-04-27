# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- `Symbol` with readable string and repr forms
- `SymbolTable` with canonical interning by `(namespace, name)`
- `sym()` and `is_symbol()` convenience helpers
- validation rules for empty and whitespace-padded symbol parts
- pytest coverage for interning, namespaces, helper APIs, and validation
