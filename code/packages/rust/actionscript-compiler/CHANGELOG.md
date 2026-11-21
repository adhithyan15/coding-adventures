# Changelog

All notable changes to `actionscript-compiler` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Refactored `AsParser::parser_parse_statement_inner` and `parser_parse_primary` into small `#[inline(never)]` helpers so deep `{}` / `()` / `[]` / `!` nesting returns `Err(AsCompilerError::RecursionDepthExceeded)` cleanly on a 1 MiB Windows debug stack (CWE-674).
- Blocked `__proto__`, `prototype`, and `constructor` property reads, writes, and invocations in `AsVmRunner` (`flash_set_property`, `flash_get_property`, `flash_invoke_method`) across AS1/AS2/AS3 (CWE-1321).
- Replaced `.lock().unwrap()` with `.unwrap_or_else(|e| e.into_inner())` in `AsVmRunner`.
- Added `tests/hostile_inputs.rs` (`#![forbid(unsafe_code)]`) covering 1 MiB stack nesting, CWE-1321, step limits, malformed literals/UTF-8, and a 1,000-case token-soup fuzz campaign.

## [0.1.0] - 2026-11-16

### Added
- Initial implementation of `SWF03: actionscript-compiler`.
- Support for ActionScript 1.0, 2.0, and 3.0 dialects (`AsDialect::{As1, As2, As3}`).
- Bounded lexer (`AsLexer`) with `MAX_TOKENS = 100_000` cap.
- Bounded recursive descent parser (`AsParser`) with `MAX_AST_DEPTH = 128` recursion limit and `MAX_AST_NODES = 50_000` cap.
- Complete AST representation (`Program`, `Stmt`, `Expr`, `ClassDef`, `InterfaceDef`, `PackageDef`).
- AST to `interpreter-ir` lowering (`AsLowering`) with `MAX_INSTRUCTIONS = 200_000` cap.
- Flash runtime environment and VM runner (`AsVmRunner`) integrating with `vm-core::VMCore` and Flash builtins (`flash_stop`, `flash_play`, `flash_goto_and_play`, `flash_goto_and_stop`, `flash_tell_target`, `flash_trace`, `flash_is`, `flash_as`).
- Comprehensive conformance test suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability conformance gate (`tests/runtime_capability.rs`).
- Security assurance artifacts: `actionscript-compiler.warrant`, `required_capabilities.json`, `assurance-profile.json`, `effect_certificate.json`, and 26-CWE `assurance_report.json`.
