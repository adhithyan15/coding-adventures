# TypeScript Parser (Dart)

`coding_adventures_typescript_parser` is a pure Dart, zero-dependency, grammar-driven syntax analyzer for TypeScript supporting 7 versioned editions (`typescript`, `ts1.0`, `ts2.0`, `ts3.0`, `ts4.0`, `ts5.0`, and `ts5.8`).

## Features
- **Embedded Grammars**: 7 precompiled parser grammars embedded as native Dart code. Zero runtime filesystem dependencies.
- **Standing Rule #20 Compliance**: 100% centralized localizable diagnostics isolated in `lib/src/messages.dart` with sealed, closed exception types and zero payload reflection.
- **CWE-400 Defense**: 65,536 token count ceiling prevents memory exhaustion.
- **CWE-674 Defense**: Unified 64-level nesting depth counter covers parentheses, braces, brackets, and consecutive prefix-operator chains (`!`, `~`, `+`, `-`, `++`, `--`, `typeof`, `void`, `delete`, `await`), bounded safely within the Dart VM stack limit (< 25% stack depth). A `StackOverflowError` catch backstop ensures graceful typed rejection.

## 5-Class Security Resilience Showcase
1. **Class 1 (CWE-400 Token Count Ceiling Defense)**: Rejects oversized token streams (> 65,536 tokens).
2. **Class 2 (CWE-674 Delimiter Nesting Depth Defense)**: Rejects deeply nested `(`, `{`, `[` exceeding 64 levels.
3. **Class 3 (CWE-674 Chained Prefix Operators & Mixed Nesting)**: Counts runs of prefix operators (`!`, `~`, `typeof`, etc.) towards nesting depth.
4. **Class 4 (CWE-209 Payload-Blind Diagnostics & Confidentiality)**: Syntax and version exceptions never echo caller TypeScript code or version strings.
5. **Class 5 (Grammar & AST Conformance Across Editions)**: Validates AST shape across all supported editions, matching neutral fixtures.
