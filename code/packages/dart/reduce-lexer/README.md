# coding_adventures_reduce_lexer (Dart)

REDUCE lexer driven by the compiled `reduce.tokens` grammar. The Dart
lane of the Rust `reduce-lexer` crate — one language definition
(`code/grammars/reduce/reduce.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_reduce_lexer/reduce_lexer.dart';

final tokens = tokenizeReduce('df(x, y)\$');
print([(for (final t in tokens) t.type)]);
// [NAME, LPAREN, NAME, COMMA, NAME, RPAREN, DOLLAR, EOF]
```

## API

- `tokenizeReduce(source)` tokenizes in one shot, returning the
  tokens with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeReduce(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_tokenize_reduce` with idiomatic Dart nullability).
- `createReduceLexer(source)` returns the underlying `GrammarLexer`
  with the REDUCE grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `Reduce tokenization failed: …`, and the `Result` twin returns
  null instead of `Err(String)`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/reduce_lexer_test.dart`): all 17 reference tests
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
