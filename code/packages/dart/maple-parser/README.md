# coding_adventures_maple_parser (Dart)

Maple parser driven by the compiled `maple.grammar` grammar. The
Dart lane of the Rust `maple-parser` crate — one language definition
(`code/grammars/maple/maple.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_maple_parser/maple_parser.dart';

final ast = parseMaple('x := 5;\n');
print(ast.ruleName);
// program
```

## API

- `parseMaple(source)` parses into an `ASTNode` rooted at
  `program`.
- `tryParseMaple(source)` is the nullable twin: `null` instead of
  throwing, with idiomatic Dart nullability. Like the reference's
  `Result`-returning `try_parse_maple`, it is fully fallible —
  lexical errors become `null` too.
- `createMapleParser(source)` returns the underlying `GrammarParser`
  with the Maple grammar and the recursion-depth guard
  (`maxRuleDepth` 150) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures and
  depth-cap trips throw `GrammarParseError`.

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking or a `Result<String>`, and the nullable twin returns
  null instead of `Err(String)`: Dart lanes throw or return null
  (the lexer/parser precedent) rather than stringifying errors.
- All six recursion-shape boundaries were re-measured on this
  engine and pinned by test; every one matches the reference
  exactly (parens 11/12, list 10/11, `not` 135/136, unary-minus
  135/136, power 67/68, nested `if` 67/68 — same grammar cascade,
  same depth counting at cap 150).
