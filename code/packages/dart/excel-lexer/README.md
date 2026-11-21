# coding_adventures_excel_lexer

A pure Dart grammar-driven lexical analyzer for Excel formulas that produces
flat token streams using the pre-compiled `excel.tokens` grammar.

## Features

- **Grammar-Driven Architecture**: Uses the embedded `TokenGrammar` compiled from
  `code/grammars/excel/excel.tokens` (SHA-256 pinned by the shared
  `language-frontends-v1` conformance manifest).
- **Formula Surface**: A1 cell references, sheet/workbook-qualified prefixes,
  array constants, error constants, structured table references, string and
  numeric literals, and the full operator set.
- **Context-Sensitive Names**: A `NAME` directly followed (ignoring ASCII
  spaces) by `(` becomes `FUNCTION_NAME`; followed by `[` becomes `TABLE_NAME`.
  This reproduces the TypeScript `excelOnToken` hook as a post-pass because the
  Dart lexer has no on-token hook.
- **Resource & Encoding Bounds**: Enforces `maxSourceLength = 1048576` (1 MB) and
  validates UTF-16 surrogates atomically before tokenization.
- **Standing Rule #20**: Every error id and message lives in
  `lib/src/messages.dart`; exceptions are sealed with closed constructors and
  never echo caller formula text.

## 5-Class Excel Lexer Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: CWE-400 Source Length DoS** | Memory exhaustion via oversized formulas | `maxSourceLength = 1048576` ceiling enforced before version resolution or tokenization. |
| **2: Surrogate Smuggling** | Lone or reversed UTF-16 surrogates causing decoding confusion | Source scanned for invalid surrogates (`0xD800`..`0xDFFF`) before processing. |
| **3: Name Disambiguation** | Function/table/name confusion changing parse meaning | Deterministic contiguity rule: only physically adjacent `SPACE` tokens may precede `(` or `[`; tabs and newlines break adjacency exactly like the character peek in the TypeScript lane. |
| **4: Literal & Reference Integrity** | Sign swallowing (`1+2` lexed as one number), unterminated strings | Split numeric token variants; typed `ExcelLexicalException` on unrecognized input. |
| **5: CWE-209 Payload-Blind Confidentiality** | Sensitive cell contents leaked into diagnostics | Static messages with 1-based positions only; caller source is never reflected. |

## Quick Start

```dart
import 'package:coding_adventures_excel_lexer/coding_adventures_excel_lexer.dart';

void main() {
  final tokens = tokenizeExcel('=SUM(A1:B2)');
  print(tokens.map((t) => t.type).toList());
  // [EQUALS, FUNCTION_NAME, LPAREN, CELL, COLON, CELL, RPAREN, EOF]
}
```

## Development & Testing

```bash
sh BUILD   # Format, analyze, conformance test, and coverage (>= 95%)
```
