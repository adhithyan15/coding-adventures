# Changelog

All notable changes to `coding_adventures_typescript_lexer` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial pure Dart implementation of TypeScript lexical analyzer.
- Embedded precompiled token grammars for 7 editions: `typescript` (generic default), `ts1.0`, `ts2.0`, `ts3.0`, `ts4.0`, `ts5.0`, and `ts5.8`.
- Standing Rule #20 compliance: 100% centralized, localizable, payload-blind diagnostics in `lib/src/messages.dart` with closed `sealed class TypescriptLexerException` hierarchy.
- CWE-400 defense with 1 MB source ceiling and UTF-16 surrogate checks validated before version resolution.
- Bounded neutral fixture conformance replay (`typescript-baseline` and SHA-256 validation against `grammars.json`).
- 5-Class Security Resilience Showcase, 200-case PRNG property test, and 10,000-iteration mutation fuzz suite.
