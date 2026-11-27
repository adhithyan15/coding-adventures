# coding_adventures_reduce_parser (Dart)

Reduce parser driven by the compiled `reduce.grammar` grammar. The
Dart lane of the Rust `reduce-parser` crate — one language definition
(`code/grammars/reduce/reduce.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_reduce_parser/reduce_parser.dart';

final ast = parseReduce('x := 5;\n');
print(ast.ruleName);
// program
```

## API

- `parseReduce(source)` parses into an `ASTNode` rooted at
  `program`.
- `tryParseReduce(source)` is the nullable twin: `null` instead of
  throwing, with idiomatic Dart nullability. Like the reference's
  `Result`-returning `try_parse_reduce`, it is fully fallible —
  lexical errors become `null` too.
- `createReduceParser(source)` returns the underlying `GrammarParser`
  with the Reduce grammar and the recursion-depth guard
  (`maxRuleDepth` 128) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures and
  depth-cap trips throw `GrammarParseError`.

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking or a `Result<String>`, and the nullable twin returns
  null instead of `Err(String)`: Dart lanes throw or return null
  (the lexer/parser precedent) rather than stringifying errors.
- All five recursion-shape boundaries were re-measured on this
  engine and pinned by test; every one matches the reference
  exactly (parens 8/9, `:=` 56/57, `if`/`else` 56/57, cons 112/113,
  power 56/57 — same grammar cascade, same depth counting at cap
  128).
