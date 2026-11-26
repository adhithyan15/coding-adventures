# coding_adventures_oct_parser (Dart)

Oct parser driven by the compiled `oct.grammar` grammar. The Dart
lane of the Rust `oct-parser` crate — one language definition
(`code/grammars/oct/oct.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_oct_parser/oct_parser.dart';

final ast = parseOct('fn main() { let x: u8 = 5; }');
print(ast.ruleName);
// program
```

## API

- `parseOct(source)` parses into an `ASTNode` rooted at
  `program`.
- `tryParseOct(source)` is the nullable twin: `null` instead of
  throwing, with idiomatic Dart nullability.
- `createOctParser(source)` / `createOctParserFromTokens(tokens)`
  return the underlying `GrammarParser` with the Oct grammar and
  the recursion-depth guard (`maxRuleDepth` 200) enabled. The
  token factory exists for preprocessor dialects (MacroOct) that
  already hold a pure Oct token stream.
- Lex failures throw the engine's `LexerError`; syntax failures and
  depth-cap trips throw `GrammarParseError`.

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of a
  `Result`, and the nullable twin returns null instead of
  `Err(GrammarParseError)`: Dart lanes throw or return null (the
  lexer/parser precedent) rather than modelling `Result`.
- The 20/21 nesting boundary was re-measured on this engine and
  pinned by test; it matches the reference exactly (same grammar
  cascade, same depth counting at cap 200).
