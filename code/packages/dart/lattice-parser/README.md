# coding_adventures_lattice_parser

A pure Dart grammar-driven parser for Lattice stylesheets that parses Lattice text
into recursive abstract syntax trees (`ASTNode`) using pre-compiled grammar definitions.

## Features

- **Grammar-Driven Architecture**: Uses pre-compiled parser grammar rules from `lattice.grammar`.
- **CSS & Lattice Support**: Parses variables, mixins, control directives (`@if`, `@for`, `@each`, `@while`), functions, `@use`, selectors, and blocks.
- **Resource & Call Stack Defense**: Restricts input token counts (`maxTokenCount = 65536`) and delimiter nesting depth across `{`/`}` and `(`/`)` (`maxNestingDepth = 256`) before recursive parsing.
- **Standing Rule #20**: Zero raw string literals in operational code, sealed exception hierarchies, and payload-blind diagnostics.

## 5-Class Lattice Parser Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: CWE-400 Token Volume DoS** | Memory exhaustion from massive token streams | `maxTokenCount = 65536` ceiling enforced before grammar parsing. |
| **2: CWE-674 Recursion Depth Exhaustion** | Deeply nested blocks (`{{{{...}}}}`) or expressions (`((((...))))`) blowing the call stack | `maxNestingDepth = 256` verified in a pre-parse token pass across braces and parentheses, rejecting excessive nesting with `LatticeParseException.nestingTooDeep()`. |
| **3: Grammar Rule Integrity** | Incorrect selector or expression precedence | Evaluated against canonical grammar rules; verified with recursive tree projection against shared neutral fixtures. |
| **4: Syntax Error Confinement** | Unrecognized syntax or unexpected tokens corrupting parser state | Re-throws `GrammarParseError` as typed `LatticeParseException.syntaxError` with 1-based line/column offsets. |
| **5: CWE-209 Payload-Blind Confidentiality** | Secret stylesheet values or selectors echoed in parse failure diagnostics | All exception types emit static messages with position indicators only, strictly redacting caller source text. |

## Quick Start

```dart
import 'package:coding_adventures_lattice_parser/coding_adventures_lattice_parser.dart';

void main() {
  final ast = parseLattice(r'$color: red; h1 { color: $color; }');
  print(ast.ruleName); // 'stylesheet'
}
```

## Development & Testing

```bash
sh BUILD   # Format, analyze, conformance test, and coverage (>= 95%)
```
