# coding_adventures_macsyma_parser (Dart)

MACSYMA parser driven by the compiled `macsyma.grammar` grammar. The
Dart lane of the Rust `macsyma-parser` crate — one language definition
(`code/grammars/macsyma/macsyma.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_macsyma_parser/macsyma_parser.dart';

final ast = parseMacsyma('x : 3\$\n');
print(ast.ruleName);
// program
```

## API

- `parseMacsyma(source)` parses into an `ASTNode` rooted at
  `program`.
- `tryParseMacsyma(source)` is the nullable twin: `null` instead of
  throwing, with idiomatic Dart nullability.
- `createMacsymaParser(source)` returns the underlying `GrammarParser`
  with the MACSYMA grammar and the recursion-depth guard
  (`maxRuleDepth` 200) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures and
  depth-cap trips throw `GrammarParseError`.

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking, and the nullable twin returns null instead of a
  `Result`: Dart lanes propagate engine errors (the
  lexer/parser precedent) rather than aborting the isolate.
- The 14/15 nesting boundary was re-measured on this engine and
  pinned by test; it matches the reference exactly (same grammar
  cascade, same depth counting at cap 200).
