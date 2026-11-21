# Changelog

## 0.1.0 - 2026-11-21

### Added

- Add pure Dart grammar-driven lexical analyzer for ANSI SQL (`coding_adventures_sql_lexer`).
- Provide uppercase `KEYWORD` normalization (`token.value.toUpperCase()`) while preserving exact casing of `NAME` identifiers.
- Support comparison operators with aliasing (`!=` and `<>` to `NOT_EQUALS`, `<=` and `>=`), string literals, quoted identifiers (backticks and double quotes), and comment skipping.
- Enforce `maxSourceLength = 1048576` ceiling and reject lone/reversed UTF-16 surrogates atomically.
- Include 5-Class SQL Lexer Security Resilience Showcase, conformance tests against `language-frontends-v1`, deterministic property tests, fuzz loop, and >= 95% coverage.
