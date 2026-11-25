# Changelog

All notable changes to `coding_adventures_typescript_parser` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial pure Dart implementation of TypeScript parser.
- Embedded precompiled parser grammars for 7 editions: `typescript` (generic default), `ts1.0`, `ts2.0`, `ts3.0`, `ts4.0`, `ts5.0`, and `ts5.8`.
- Standing Rule #20 compliance: 100% centralized localizable diagnostics isolated in `lib/src/messages.dart` with closed `sealed class TypescriptParserException` hierarchy.
- CWE-400 defense with 65,536 token ceiling.
- CWE-674 defense with combined 64-level nesting depth tracking parentheses, curly braces, brackets, and consecutive prefix-operator chains (`!`, `~`, `+`, `-`, `++`, `--`, `typeof`, `void`, `delete`, `await`), plus safe `StackOverflowError` backstop to typed exception.
- Bounded neutral fixture conformance replay (`typescript-baseline` AST projection and SHA-256 validation against `grammars.json`).
- 5-Class Security Resilience Showcase, 200-case PRNG property test, and 10,000-iteration mutation fuzz suite.
