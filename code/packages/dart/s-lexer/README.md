# coding_adventures_s_lexer (Dart)

S lexer driven by the compiled `s.tokens` grammar. The Dart lane of
the Rust `s-lexer` crate — one language definition
(`code/grammars/s/s.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_s_lexer/s_lexer.dart';

final tokens = tokenizeS('x <- c(1, 2, 3)\n');
print([(for (final t in tokens) t.type)]);
// [NAME, ASSIGN, NAME, LPAREN, NUMBER, COMMA, NUMBER, COMMA, NUMBER,
//  RPAREN, NEWLINE, EOF]
```

## API

- `tokenizeS(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeS(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_tokenize_s` with idiomatic Dart nullability).
- `createSLexer(source)` returns the underlying `GrammarLexer` with
  the S grammar and the bracket-interior newline hook registered.
- `dropBracketedNewlines(tokens)` is the post-tokenize hook: it drops
  `NEWLINE` tokens inside open `(` or `[` (never `{`).
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `S tokenization failed: …`, and the `Result` twin returns
  null instead of `Err(String)`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/s_lexer_test.dart`): all 24 reference tests plus
  error positions, the nullable API shape, and empty input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, an independent hook mirror over
  bracket soup, underscore splitting, dotted-name wholeness, comment
  invisibility, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, both APIs always
  agree, and an independent depth replay proves no NEWLINE survives
  at paren/bracket depth.
- **Coverage**: 100% lines and functions.
