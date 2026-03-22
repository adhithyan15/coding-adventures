# Changelog

All notable changes to the TOML Parser package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the TOML parser.
- `parseTOML()` function that parses TOML text into ASTs using the grammar-driven parser engine.
- Loads `toml.grammar` file defining all TOML v1.0.0 syntax rules.
- Supports all TOML value types: strings (4 types), integers, floats, booleans, date/time (4 types), arrays, inline tables.
- Supports table headers ([table]) and array-of-tables headers ([[array]]).
- Supports dotted keys (a.b.c = 1) and quoted keys ("127.0.0.1" = value).
- Multi-line array parsing with optional trailing commas.
- Comprehensive test suite covering primitive values, key-value pairs, table headers, array-of-tables, arrays, inline tables, complete documents, and error cases.
