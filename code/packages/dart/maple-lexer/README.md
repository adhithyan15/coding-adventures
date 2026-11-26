# coding_adventures_maple_lexer (Dart)

Maple lexer driven by the compiled `maple.tokens` grammar. The Dart
lane of the Rust `maple-lexer` crate — one language definition
(`code/grammars/maple/maple.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_maple_lexer/maple_lexer.dart';

final tokens = tokenizeMaple('f := (x, y) -> x + y;');
print([(for (final t in tokens) t.type)]);
// [NAME, ASSIGN, LPAREN, NAME, COMMA, NAME, RPAREN, ARROW, NAME,
//  PLUS, NAME, SEMI, EOF]
```

## API

- `tokenizeMaple(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeMaple(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_tokenize_maple` with idiomatic Dart nullability).
- `createMapleLexer(source)` returns the underlying `GrammarLexer`
  with the Maple grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `Maple tokenization failed: …`, and the `Result` twin returns
  null instead of `Err(String)`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/maple_lexer_test.dart`): all 28 reference tests
  plus error positions, the nullable API shape, and empty input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, operator atomicity, number
  wholeness, keyword exactness, terminator distinctness, API
  agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, and both APIs
  always agree.
- **Coverage**: 100% lines and functions.
