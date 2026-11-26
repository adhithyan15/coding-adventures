# coding_adventures_nib_parser (Dart)

Nib parser driven by the compiled `nib.grammar` grammar. The Dart
lane of the Rust `nib-parser` crate — one language definition
(`code/grammars/nib/nib.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_nib_parser/nib_parser.dart';

final ast = parseNib('fn main() { let x: u4 = 1; }');
print(ast.ruleName);
// program
```

## API

- `parseNib(source)` parses into an `ASTNode` rooted at
  `program`.
- `tryParseNib(source)` is the nullable twin: `null` instead of
  throwing, with idiomatic Dart nullability.
- `parseNibTokens(tokens)` / `tryParseNibTokens(tokens)` parse a
  pre-tokenized stream with the same grammar and depth guard, for
  preprocessing dialects that already hold tokens.
- `createNibParser(source)` / `createNibParserFromTokens(tokens)`
  return the underlying `GrammarParser` with the Nib grammar and
  the recursion-depth guard (`maxRuleDepth` 200) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures and
  depth-cap trips throw `GrammarParseError`.

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of a
  `Result`, and the nullable twins return null instead of
  `Err(GrammarParseError)`: Dart lanes throw or return null (the
  lexer/parser precedent) rather than modelling `Result`.
- The 16/17 nesting boundary was re-measured on this engine and
  pinned by test; it matches the reference exactly (same grammar
  cascade, same depth counting at cap 200).
