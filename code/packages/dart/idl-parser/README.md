# coding_adventures_idl_parser (Dart)

IDL parser driven by the compiled `idl.grammar` grammar. The Dart
lane of the Rust `idl-parser` crate — one language definition
(`code/grammars/idl/idl.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_idl_parser/idl_parser.dart';

final ast = parseIdl('x = 5\n');
print(ast.ruleName);
// program
```

## API

- `parseIdl(source)` parses into an `ASTNode` rooted at
  `program`.
- `tryParseIdl(source)` is the nullable twin: `null` instead of
  throwing, with idiomatic Dart nullability. Like the reference's
  `Result`-returning `try_parse_idl`, it is fully fallible —
  lexical errors become `null` too.
- `createIdlParser(source)` returns the underlying `GrammarParser`
  with the IDL grammar and the recursion-depth guard
  (`maxRuleDepth` 148) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures and
  depth-cap trips throw `GrammarParseError`.

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking or a `Result<String>`, and the nullable twin returns
  null instead of `Err(String)`: Dart lanes throw or return null
  (the lexer/parser precedent) rather than stringifying errors.
- All six recursion-shape boundaries were re-measured on this
  engine and pinned by test; every one matches the reference
  exactly (parens 13/14, nested `IF` 26/27, call args 12/13,
  subscripts 11/12, unary chain 134/135, array literals 12/13 —
  same grammar cascade, same depth counting at cap 148).
