# coding_adventures_verilog_lexer

Pure Dart grammar-driven lexical analyzer and preprocessor for Verilog HDL (IEEE 1364-1995, IEEE 1364-2001, and IEEE 1364-2005).

## Overview

`coding_adventures_verilog_lexer` provides tokenization and macro preprocessing for Verilog Hardware Description Language (HDL) source code.

### Key Capabilities

- **Multi-Edition Grammar Support**: Native precompiled grammar support for IEEE 1364-1995, 2001, and 2005.
- **Line-Preserving Preprocessor**: In-memory directive processing (`define, `undef, `ifdef, `ifndef, `else, `endif, `timescale, `include) preserving line numbers for downstream error reporting.
- **Zero Ambient Authority**: In-memory macro expansion without filesystem access; `include directives are securely stubbed as comments.
- **Standing Rule #20 Compliance**: 100% centralized localizable diagnostic catalog in `messages.dart`, sealed exceptions with closed constructors, and strict payload-blindness.

---

## 5-Class Security Resilience Showcase

| Class | Vulnerability Vector | Defense Mechanism | Invariant Guaranteed |
| :--- | :--- | :--- | :--- |
| **Class 1: Bounded Memory & Input Envelope** | Denial of Service via unbounded source files or recursive macro expansion bombs (CWE-400) | Enforces `maxSourceLength = 1,048,576` code units (1 MB) and `maxConditionalDepth = 64` both pre- and post-expansion | Prevents heap exhaustion and exponential macro expansion |
| **Class 2: Lexical, Character & Encoding Integrity** | UTF-16 surrogate desynchronization or lone surrogate injection | Pre-scan validates paired surrogates (`0xD800`..`0xDFFF`) before preprocessing or version routing | Guarantees well-formed UTF-16 representation |
| **Class 3: Ambient Authority Isolation** | Arbitrary file inclusion or directory traversal via `` `include `` | Pure in-memory preprocessor transforms `` `include `` into inline comments with zero filesystem access | Eliminates path traversal and local file inclusion (LFI) |
| **Class 4: Closed Diagnostic Catalog & Payload Blindness** | Sensitive source code or macro payload leakage via error messages | Sealed `VerilogLexerException` with closed constructors and static messages from `messages.dart` | Eliminates payload echoing in error messages |
| **Class 5: Preprocessor Directive State Integrity** | Condition stack corruption or unbalanced conditional directives | Strict condition stack validation tracking parent active status and balanced parenthesis matching | Guarantees deterministic conditional compilation |

---

## Usage

```dart
import 'package:coding_adventures_verilog_lexer/verilog_lexer.dart';

void main() {
  const source = '''
`define WIDTH 8
module counter(input wire [`WIDTH-1:0] in);
endmodule
''';

  final tokens = tokenizeVerilog(source);
  for (final token in tokens) {
    print('${token.type}: ${token.value}');
  }
}
```

---

## Verification & Testing

Validate formatting, static analysis, conformance, and test coverage:

```bash
sh BUILD
```
