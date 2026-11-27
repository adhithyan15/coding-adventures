# coding_adventures_axiom_parser (Dart)

Axiom parser driven by the compiled `axiom.grammar` grammar. The
Dart lane of the Rust `axiom-parser` crate — one language definition
(`code/grammars/axiom/axiom.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_axiom_parser/axiom_parser.dart';

final ast = parseAxiom('factorial 7');
print(ast.ruleName);
// program
```

`program` is exactly one top-level expression — the tokens give
top-level inputs no separator, so trailing input after one full
expression is a syntax error, not a second statement.

## API

- `parseAxiom(source)` parses into an `ASTNode` rooted at
  `program`.
- `tryParseAxiom(source)` is the nullable twin: `null` instead of
  throwing, with idiomatic Dart nullability. Like the reference's
  `Result`-returning `try_parse_axiom`, it is fully fallible —
  lexical errors become `null` too.
- `createAxiomParser(source)` returns the underlying `GrammarParser`
  with the Axiom grammar and the recursion-depth guard
  (`maxRuleDepth` 140) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures and
  depth-cap trips throw `GrammarParseError`.

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking or a `Result<String>`, and the nullable twin returns
  null instead of `Err(String)`: Dart lanes throw or return null
  (the lexer/parser precedent) rather than stringifying errors.
- All four recursion-shape boundaries were re-measured on this
  engine and pinned by test; every one matches the reference
  exactly (parens 13/14, nested calls 13/14, unary chain 130/131,
  power 65/66 — same grammar cascade, same depth counting at cap
  140).
