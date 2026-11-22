# TypeScript Lexer (Dart)

`coding_adventures_typescript_lexer` is a pure Dart, zero-dependency, grammar-driven lexical analyzer for TypeScript supporting 7 versioned editions (`typescript`, `ts1.0`, `ts2.0`, `ts3.0`, `ts4.0`, `ts5.0`, and `ts5.8`).

## Features
- **Embedded Grammars**: 7 precompiled token grammars embedded as native Dart code. Zero runtime filesystem dependencies.
- **Standing Rule #20 Compliance**: 100% centralized localizable diagnostics isolated in `lib/src/messages.dart` with sealed, closed exception types and zero payload reflection.
- **CWE-400 Defense**: 1 MB (`1,048,576` code units) source size limit and UTF-16 surrogate validation enforced before version resolution.

## 5-Class Security Resilience Showcase
1. **Class 1 (CWE-400 Source Length DoS Defense)**: Rejects oversized inputs exceeding 1 MB without memory exhaustion.
2. **Class 2 (CWE-400 Lone / Reversed UTF-16 Surrogates)**: Rejects malformed UTF-16 code units safely.
3. **Class 3 (CWE-209 Payload Confidentiality)**: Exceptions never echo caller TypeScript code or version strings.
4. **Class 4 (Multi-Edition Version Resolution)**: Supports canonical editions (`ts1.0`..`ts5.8` and `typescript`) and common aliases, rejecting unrecognized editions safely.
5. **Class 5 (Lexical Coverage Across Editions)**: Deterministically lexes identifiers, keywords, strings, operators, and comments across all supported editions.
