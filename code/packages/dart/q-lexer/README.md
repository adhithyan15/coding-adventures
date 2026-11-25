# coding_adventures_q_lexer (Dart)

Q lexer driven by the compiled `q.tokens` grammar. The Dart lane of
the Rust `q-lexer` crate — one language definition
(`code/grammars/q/q.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_q_lexer/q_lexer.dart';

final tokens = tokenizeQ('x:2 -1\n');
print([(for (final t in tokens) (t.type, t.value))]);
// [(NAME, x), (COLON, :), (NUMBER, 2), (NUMBER, -1), (NEWLINE, \n), (EOF, )]
```

## API

- `tokenizeQ(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeQ(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_tokenize_q` with idiomatic Dart nullability).
- `createQLexer(source)` returns the underlying `GrammarLexer` with
  the Q grammar and both hooks installed.
- `stripSlashComments(source)` is the pre-tokenize hook: `/` opens a
  comment after whitespace or at line start; comment characters
  become spaces one-for-one (positions preserved), the newline kept.
- `foldNegativeNumberLiterals(tokens)` is the post-tokenize hook: a
  `MINUS` glued to a `NUMBER`, where a new strand element may start,
  folds into one signed `NUMBER`.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `Q tokenization failed: …`, and the `Result` twin returns
  null instead of `Err(String)`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/q_lexer_test.dart`): all 5 in-module reference
  tests and all 47 integration tests, plus error positions, the
  nullable API shape, and empty input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, comment blanking (rune-exact,
  newline-preserving), the six-row fold truth table, number
  wholeness, brace shape, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, and both APIs
  always agree.
- **Coverage**: 100% lines and functions.
