# coding_adventures_sql_lexer

A pure Dart grammar-driven lexical analyzer for ANSI SQL that tokenizes SQL source
text into tokens using pre-compiled grammar definitions.

## Features

- **Grammar-Driven Architecture**: Uses pre-compiled token grammars from `sql.tokens`.
- **Case-Insensitive Keyword Normalization**: Normalizes SQL keywords (`SELECT`, `FROM`, etc.)
  to uppercase while preserving the casing of user identifiers (`NAME`).
- **Operator Aliasing**: Unifies inequality operators (`!=` and `<>` both map to `NOT_EQUALS`).
- **Quoted Identifiers**: Cleanly processes backtick-quoted (`` `table` ``) and double-quoted
  (`"col"`) identifiers.
- **Comment Skipping**: Silently ignores line comments (`-- ...`) and multi-line block comments (`/* ... */`).
- **Standing Rule #20**: Zero raw string literals in operational code, sealed exception
  hierarchies, and payload-blind error diagnostics.

## 5-Class SQL Lexer Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: CWE-400 Source Length DoS** | Memory exhaustion from unbounded SQL query strings | `maxSourceLength = 1048576` (1 MB) ceiling enforced atomically before tokenization. |
| **2: Surrogate Smuggling & Encoding Attacks** | Corrupted UTF-16 code units or lone surrogates causing crashes or bypasses | Complete pre-validation rejects lone and reversed UTF-16 surrogates (`0xD800..0xDFFF`) before tokenization. |
| **3: Case-Insensitive Keyword Normalization** | Case variation evasion in SQL keyword matching (e.g. `SeLeCt`) | All `KEYWORD` tokens are normalized to uppercase, while `NAME` tokens preserve exact casing. |
| **4: Operator & Identifier Aliasing Integrity** | Inconsistent operator handling (`!=` vs `<>`) or raw quotes in identifiers | `<>` and `!=` both produce `NOT_EQUALS`; quoted identifiers are normalized to clean `NAME` values. |
| **5: CWE-209 Payload-Blind Confidentiality** | Sensitive database data or queries leaked through exception messages | Closed exception hierarchy (`InvalidSqlVersionException`, `InvalidSqlSourceException`, `SqlLexicalException`) emits only static messages and line/column offsets, never caller SQL text. |

## Quick Start

```dart
import 'package:coding_adventures_sql_lexer/sql_lexer.dart';

void main() {
  final tokens = tokenizeSql('SELECT id, name FROM users WHERE age >= 18');
  for (final token in tokens) {
    print('${token.type}: ${token.value} (${token.line}:${token.column})');
  }
}
```

## Development & Testing

```bash
sh BUILD   # Format, analyze, conformance test, and coverage (>= 95%)
```
