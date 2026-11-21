# Changelog

All notable changes to `coding_adventures_excel_lexer` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Grammar-driven lexical analyzer for Excel formulas using the pre-compiled `excel.tokens` grammar.
- Context-sensitive `NAME` promotion to `FUNCTION_NAME` / `TABLE_NAME` matching the TypeScript `excelOnToken` hook.
- Bounded input validation, surrogate defense, and 100% centralized, payload-blind diagnostics per Standing Rule #20.
