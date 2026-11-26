# coding_adventures_c_lexer (Dart)

C lexer driven by the compiled `c.tokens` grammar (the integer-core
subset, SIR27). The Dart lane of the Rust `c-lexer` crate — one
language definition (`code/grammars/c/c.tokens`) powers every
implementation.

```dart
import 'package:coding_adventures_c_lexer/c_lexer.dart';

final tokens = tokenizeC('int32_t x = 5;');
print([(for (final t in tokens) t.type)]);
// [KEYWORD, NAME, EQ, INT_LIT, SEMICOLON, EOF]
```

## API

- `tokenizeC(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeC(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_tokenize_c` with idiomatic Dart nullability).
- `createCLexer(source)` returns the underlying `GrammarLexer` with
  the C grammar.
- `isCKeyword(word)` reports whether the compiled grammar reserves
  the word (keywords plus reserved words), from a set built once.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `C tokenization failed: …`, and the `Result` twin returns
  null instead of `Err(String)`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/c_lexer_test.dart`): all 9 reference tests plus
  literal values with escapes, keyword predicate edges, the missing
  `ARROW` (pinned as `MINUS` + `GT`), error positions, and empty
  input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, keyword agreement, operator
  atomicity, integer wholeness, comment invisibility, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over the in-lane `fuzz` package; only `LexerError` escapes, and
  both APIs always agree.
- **Coverage**: 100% lines and functions.
