# coding_adventures_vhdl_parser

A pure Dart grammar-driven parser for VHDL (IEEE 1076) that parses design
files into recursive abstract syntax trees (`ASTNode`) using pre-compiled
edition grammars.

## Features

- **Five Editions**: 1987, 1993, 2002, 2008 (default) and 2019, each embedded
  from `code/grammars/vhdl/vhdl<year>.grammar` (SHA-256 pinned by the shared
  `language-frontends-v1` conformance manifest). Accepts `2008` and `vhdl2008`.
- **Lexer Integration**: Tokenizes with `coding_adventures_vhdl_lexer`, so
  keywords and identifiers arrive case-folded.
- **Resource & Call Stack Defense**: Restricts token counts
  (`maxTokenCount = 65536`) and one combined nesting depth
  (`maxNestingDepth = 64`) covering parentheses, block-opening keywords and
  prefix-operator chains before recursive parsing, with a typed backstop for any
  stack overflow that slips through.
- **Standing Rule #20**: Every error id and message lives in
  `lib/src/messages.dart`; exceptions are sealed with closed constructors and
  never echo caller source.

## 5-Class VHDL Parser Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: Edition Domain Isolation** | Unknown edition strings selecting the wrong grammar | Closed alias table; anything else throws `InvalidVhdlParserVersionException` without echoing the input. |
| **2: CWE-400 Token Volume DoS** | Memory exhaustion from massive token streams | `maxTokenCount = 65536` ceiling enforced in one linear pass before grammar parsing. |
| **3: CWE-674 Recursion Depth Exhaustion** | VHDL nests with no delimiters at all: `if a then if b then ...`, `not not not x`. Counting only parentheses (as the brace/paren frontends do) would be bypassable, like the Lattice `FUNCTION` token case. | One combined `maxNestingDepth = 64`: `(` and opener keywords (`if`, `case`, `loop`, `process`, `block`, `generate`) increment, `)` and `end <opener>` decrement (clamped at 0), `if ... generate` counts once, and a run of prefix operators (`not`, `abs`, `+`, `-`) adds its length. Probed on the Dart VM: parentheses overflow the stack between 256 and 384 levels, so 64 keeps a wide margin. A `StackOverflowError` is mapped to `VhdlParseException.nestingTooDeep()`. |
| **4: Syntax Error Confinement** | Unexpected tokens corrupting parser state | Re-throws `GrammarParseError` as typed `VhdlParseException.syntaxError` with 1-based line/column offsets. |
| **5: CWE-209 Payload-Blind Confidentiality** | Secret identifiers echoed in parse failure diagnostics | Static messages with position indicators only; caller source text is never reflected. |

## Quick Start

```dart
import 'package:coding_adventures_vhdl_parser/coding_adventures_vhdl_parser.dart';

void main() {
  final ast = parseVhdl('entity e is end entity e;', version: '2008');
  print(ast.ruleName); // 'design_file'
}
```

## Development & Testing

```bash
sh BUILD   # Format, analyze, conformance test, and coverage (>= 95%)
```
