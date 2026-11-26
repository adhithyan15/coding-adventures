# coding_adventures_cobol_parser (Dart)

COBOL-60 parser driven by the compiled `cobol.grammar` grammar. The
Dart lane of the Rust `cobol-parser` crate — one language definition
(`code/grammars/cobol/cobol.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_cobol_parser/cobol_parser.dart';

final ast = parseCobol([
  '000000 IDENTIFICATION DIVISION.',
  '000000 PROGRAM-ID. HELLO.',
  '000000 PROCEDURE DIVISION.',
  '000000 MAIN.',
  '000000     STOP RUN.',
].join('\n'));
print(ast.ruleName);
// program
```

Source is 80-column card format: the lexer strips the 6-column
sequence area plus indicator column before tokenizing, so every code
line rides on a card.

## API

- `parseCobol(source)` parses into an `ASTNode` rooted at
  `program`.
- `tryParseCobol(source)` is the nullable twin: `null` instead of
  throwing, with idiomatic Dart nullability. Like the reference's
  `Result`-returning `try_parse_cobol`, it is fully fallible —
  lexical errors become `null` too.
- `createCobolParser(source)` returns the underlying `GrammarParser`
  with the COBOL grammar and the recursion-depth guard
  (`maxRuleDepth` 128, the shared default) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures and
  depth-cap trips throw `GrammarParseError`.

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking or a `Result<String>`, and the nullable twin returns
  null instead of `Err(String)`: Dart lanes throw or return null
  (the lexer/parser precedent) rather than stringifying errors.
- The 57/58 IF-nesting boundary was measured on this engine and
  pinned by test as a regression tripwire. The reference pins no
  exact boundary (it only asserts 4096 nested IFs fail), so this
  guards the Dart engine's own counting, not a cross-lane claim.
