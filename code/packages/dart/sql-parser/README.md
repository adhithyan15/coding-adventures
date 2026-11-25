# coding_adventures_sql_parser

A pure Dart grammar-driven parser for ANSI SQL that parses SQL text into recursive
abstract syntax trees (`ASTNode`) using pre-compiled grammar definitions.

## Features

- **Grammar-Driven Architecture**: Uses pre-compiled parser grammar rules from `sql.grammar`.
- **Full DDL & DML Support**: Parses `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `CREATE TABLE`, `DROP TABLE`, `ALTER TABLE`, and subqueries.
- **Operator Precedence**: Enforces standard mathematical and logical operator precedence through recursive grammar rules.
- **Resource & Stack Defense**: Restricts input token counts (`maxTokenCount = 65536`) and parenthetical nesting depth (`maxNestingDepth = 256`) before recursive parsing.
- **Standing Rule #20**: Zero raw string literals in operational code, sealed exception hierarchies, and payload-blind diagnostics.

## 5-Class SQL Parser Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: CWE-400 Token Volume DoS** | Memory exhaustion from massive token streams | `maxTokenCount = 65536` ceiling enforced before grammar parsing. |
| **2: CWE-674 Recursion Depth Exhaustion** | Deeply nested parenthetical expressions (`((((...))))`) blowing the call stack | `maxNestingDepth = 256` verified in a pre-parse token pass, rejecting excessive nesting with `SqlParseException.nestingTooDeep()`. |
| **3: Grammar Rule Integrity** | Incorrect precedence or malformed AST structures | Evaluated against canonical PEG grammar rules; verified with recursive tree projection against shared neutral fixtures. |
| **4: Syntax Error Confinement** | Unrecognized syntax or unexpected tokens corrupting parser state | Re-throws `GrammarParseError` as typed `SqlParseException.syntaxError` with 1-based line/column offsets. |
| **5: CWE-209 Payload-Blind Confidentiality** | Secret database values or queries echoed in parse failure diagnostics | All exception types emit static messages with position indicators only, strictly redacting caller SQL payloads. |

## Quick Start

```dart
import 'package:coding_adventures_sql_parser/sql_parser.dart';

void main() {
  final ast = parseSql('SELECT id, name FROM users WHERE age >= 18');
  print(ast.ruleName); // 'program'
}
```

## Development & Testing

```bash
sh BUILD   # Format, analyze, conformance test, and coverage (>= 95%)
```
