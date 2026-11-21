# Changelog

## 0.1.0 - 2026-11-21

### Added

- Add pure Dart grammar-driven parser for ANSI SQL (`coding_adventures_sql_parser`).
- Expose `loadSqlParserGrammar`, `SqlParser`, `createSqlParser`/`createSQLParser`, `parseSql`/`parseSQL`, and `parseSqlTokens`.
- Enforce `maxTokenCount = 65536` ceiling and `maxNestingDepth = 256` defense before grammar execution.
- Include 5-Class SQL Parser Security Resilience Showcase, conformance test against `language-frontends-v1` recursive AST structure, property tests, and >= 95% coverage.
