# coding_adventures_excel_parser

A pure Dart grammar-driven parser for Excel formulas that parses formula text
into recursive abstract syntax trees (`ASTNode`) using the pre-compiled
`excel.grammar` rules.

## Features

- **Grammar-Driven Architecture**: Uses the embedded `ParserGrammar` compiled from
  `code/grammars/excel/excel.grammar` (SHA-256 pinned by the shared
  `language-frontends-v1` conformance manifest).
- **Formula Surface**: Operators with full precedence, percent and unary
  operators, A1/range/union/intersection references, whole-row and whole-column
  ranges, sheet/workbook-qualified references, array constants, function calls,
  and structured table references.
- **Reference Normalization**: `NAME`/`NUMBER` tokens adjacent to a `COLON` become
  `COLUMN_REF`/`ROW_REF` (`A:B`, `1:10`), mirroring
  `normalizeExcelReferenceTokens` in the TypeScript lane. The parser's `tokens`
  are the normalized stream.
- **Resource & Call Stack Defense**: Restricts input token counts
  (`maxTokenCount = 65536`) and combined delimiter nesting across `(`/`)`,
  `{`/`}` and `[`/`]` (`maxNestingDepth = 64`, Excel's own limit) before
  recursive parsing.
- **Standing Rule #20**: Every error id and message lives in
  `lib/src/messages.dart`; exceptions are sealed with closed constructors and
  never echo caller formula text.

## 5-Class Excel Parser Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: CWE-400 Token Volume DoS** | Memory exhaustion from massive token streams | `maxTokenCount = 65536` ceiling enforced in a single linear pass before normalization or grammar parsing. |
| **2: CWE-674 Recursion Depth Exhaustion** | Deeply nested `((((...))))`, `{{{{...}}}}` or bracket chains blowing the call stack (the formula grammar descends ~20 rules per level) | `maxNestingDepth = 64` shared by all three delimiter kinds, verified in a pre-parse token pass and rejected with `ExcelParseException.nestingTooDeep()`. |
| **3: Reference Integrity** | `A:B` / `1:10` misread as names or numbers | Deterministic colon-adjacency reclassification, verified against shared neutral fixtures and unit tests. |
| **4: Syntax Error Confinement** | Unexpected tokens corrupting parser state | Re-throws `GrammarParseError` as typed `ExcelParseException.syntaxError` with 1-based line/column offsets. |
| **5: CWE-209 Payload-Blind Confidentiality** | Secret cell values echoed in parse failure diagnostics | Static messages with position indicators only; caller source text is never reflected. |

## Quick Start

```dart
import 'package:coding_adventures_excel_parser/coding_adventures_excel_parser.dart';

void main() {
  final ast = parseExcel('=SUM(A:B, 1:10)');
  print(ast.ruleName); // 'formula'
}
```

## Development & Testing

```bash
sh BUILD   # Format, analyze, conformance test, and coverage (>= 95%)
```
