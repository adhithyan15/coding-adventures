# coding_adventures_r_lexer (Dart)

R lexer driven by the compiled `r.tokens` grammar. The Dart lane of
the Rust `r-lexer` crate — one language definition
(`code/grammars/r/r.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_r_lexer/r_lexer.dart';

final tokens = tokenizeR('data_frame <- 1\n');
print([(for (final t in tokens) t.type)]);
// [NAME, ASSIGN, NUMBER, NEWLINE, EOF]
```

## API

- `tokenizeR(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeR(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_tokenize_r` with idiomatic Dart nullability).
- `createRLexer(source)` returns the underlying `GrammarLexer` with
  the R grammar and the bracket-interior newline hook registered.
- `dropBracketedNewlines(tokens)` is the post-tokenize hook: it drops
  `NEWLINE` tokens inside open `(` or `[` (never `{`), so calls and
  indexes may span lines.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `R tokenization failed: …`, and the `Result` twin returns
  null instead of `Err(String)`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/r_lexer_test.dart`): all 12 reference tests plus
  hook edge cases (brackets, unbalanced closers — counts proven
  identical in Rust via differential probe), error positions, the
  nullable API shape, and empty input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, an independent hook mirror over
  bracket soup, bracket shape, number/underscore-name wholeness, API
  agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, both APIs always
  agree, and an independent depth replay proves no NEWLINE survives
  at paren/bracket depth.
- **Coverage**: 100% lines and functions.
