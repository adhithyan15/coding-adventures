# coding_adventures_starlark_lexer

Pure Dart grammar-driven lexical analyzer for Starlark with Python-compatible significant indentation tracking (`INDENT`, `DEDENT`, `NEWLINE`).

## Features

- **Precompiled Token Grammar**: Driven by `starlark.tokens` (SHA-256 `7b0f726b893d4152bd5b57b492d8bfe2371633bbf52885f5a273fc2d600c89a9`).
- **Python-Style Indentation Mode**: Automatically handles leading space counting, bracket depth suppression (`()`, `[]`, `{}`), blank/comment line skipping, and EOF `NEWLINE`/`DEDENT` flushing.
- **Standing Rule #20 Compliance**: 100% centralized, localizable diagnostic error catalog with sealed exception hierarchy and payload-blind error reporting.
- **Zero Platform Capabilities**: Inert in-memory tokenizer with zero external authority.

## 5-Class Security Resilience Showcase

| Class | Vulnerability Vector | Defense Mechanism | Test Verification |
|---|---|---|---|
| **Class 1: Memory Exhaustion / Allocation Bomb** | Megabyte/Gigabyte payload injection | `maxSourceLength = 1048576` (1 MB) ceiling rejects oversized sources in O(1) time before allocation | `test('Class 1: rejects sources exceeding 1 MB limit')` |
| **Class 2: Unicode & Surrogate Pair Injection** | Isolated or ill-formed UTF-16 surrogates (CWE-20) | Strict O(n) UTF-16 surrogate pair validator rejects lone high/low surrogates with typed, payload-blind exception | `test('Class 2: rejects malformed UTF-16 surrogates')` |
| **Class 3: Indentation Ambiguity & Tab Injection** | Mixed tabs/spaces indentation ambiguity | Leading horizontal tab character in indentation triggers immediate `StarlarkLexicalException` | `test('Class 3: rejects tab in indentation')` |
| **Class 4: Boundary & State Machine Invariants** | Inconsistent dedent levels / bracket depth desync | Indentation stack strictly validates that every dedent returns to an exact previous indentation level | `test('Class 4: rejects inconsistent dedent')` |
| **Class 5: Diagnostic Payload Blindness** | Sensitive source text / credential leakage (CWE-209) | Sealed `StarlarkLexerException` hierarchy with closed constructors draws error messages solely from `StarlarkLexerMessages` | `test('Class 5: enforces payload-blind diagnostics')` |

## Usage

```dart
import 'package:coding_adventures_starlark_lexer/starlark_lexer.dart';

void main() {
  const code = '''
def greet(name):
    return "Hello, " + name
''';

  final tokens = tokenizeStarlark(code);
  for (final token in tokens) {
    print('${token.type}: ${token.value}');
  }
}
```
