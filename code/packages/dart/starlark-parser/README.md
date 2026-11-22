# coding_adventures_starlark_parser

Pure Dart grammar-driven recursive descent parser for Starlark producing concrete syntax trees.

## Features

- **Precompiled Parser Grammar**: Driven by `starlark.grammar` (SHA-256 `415005e733a195126779f749defccce57fe6b606fa75dd4e566187ca53345127`, entry rule `'file'`).
- **Bounded Resource Ceilings**: Strict defenses against memory exhaustion and stack overflow attacks (CWE-400 / CWE-674).
- **Standing Rule #20 Compliance**: 100% centralized, localizable diagnostic error catalog with sealed exception hierarchy and payload-blind error reporting.
- **Zero Platform Capabilities**: Inert in-memory parser with zero external authority.

## 5-Class Security Resilience Showcase

| Class | Vulnerability Vector | Defense Mechanism | Test Verification |
|---|---|---|---|
| **Class 1: Memory Exhaustion / Token Bomb** | Unbounded token sequence injection | `maxTokenCount = 65536` ceiling rejects token bomb streams before parsing begins | `test('Class 1: rejects token streams exceeding 65,536 tokens')` |
| **Class 2: Recursive Descent Stack Overflow** | Deeply nested brackets, block scopes, or prefix ops (CWE-674) | Single combined nesting depth counter (`maxNestingDepth = 64`) across delimiters, indents, and consecutive prefix ops plus `StackOverflowError` backstop | `test('Class 2: rejects nesting depth exceeding 64')` |
| **Class 3: Indentation Mode AST Integrity** | Malformed block indentation / missing suite | Grammar-level indentation validation enforces suites begin with `NEWLINE INDENT` and end with `DEDENT` | `test('Class 3: validates block suite indentation')` |
| **Class 4: Boundary & State Machine Invariants** | Empty source, trailing statement separators, version bounds | Precompiled grammar parses empty source or trailing newlines into well-formed root syntax tree | `test('Class 4: parses boundary constructs and validates version bounds')` |
| **Class 5: Diagnostic Payload Blindness** | Sensitive source text / credential leakage (CWE-209) | Sealed `StarlarkParserException` hierarchy with closed constructors draws error messages solely from `StarlarkParserMessages` | `test('Class 5: enforces payload-blind diagnostics')` |

## Usage

```dart
import 'package:coding_adventures_starlark_parser/starlark_parser.dart';

void main() {
  const code = 'x = 1\n';
  final ast = parseStarlark(code);
  print('Root rule: ${ast.ruleName}');
}
```
