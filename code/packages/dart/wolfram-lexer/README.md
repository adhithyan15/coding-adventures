# coding_adventures_wolfram_lexer (Dart)

Wolfram lexer driven by the compiled `wolfram.tokens` grammar. The
Dart lane of the Rust `wolfram-lexer` crate — one language definition
(`code/grammars/wolfram/wolfram.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_wolfram_lexer/wolfram_lexer.dart';

final tokens = tokenizeWolfram('Sin[x]\n');
print([for (final t in tokens) t.type]);
// [NAME, LBRACKET, NAME, RBRACKET, NEWLINE, EOF]
```

## API

- `tokenizeWolfram(source)` tokenizes in one shot, returning the
  tokens with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeWolfram(source)` is the nullable twin: `null` instead
  of throwing (mirrors the reference's `Result`-returning
  `try_tokenize_wolfram` with idiomatic Dart nullability).
- `createWolframLexer(source)` returns the underlying `GrammarLexer`
  with the Wolfram grammar and the bracket-interior newline hook
  registered.
- `dropBracketedNewlines(tokens)` is the post-tokenize hook: it drops
  `NEWLINE` tokens inside open `(`, `[`, `{`, or `[[` (`[[` counts
  double, closed by two single `]`).
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `Wolfram tokenization failed: …`, and the `Result` twin
  returns null instead of `Err(String)`: Dart lanes propagate engine
  errors (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/wolfram_lexer_test.dart`): all 21 reference tests
  plus error positions, the nullable API shape, and empty input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, an independent hook mirror over
  bracket soup, operator atomicity, part-expression shape, comment
  invisibility, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, both APIs always
  agree, and an independent depth replay proves no NEWLINE survives
  at bracket depth.
- **Coverage**: 100% lines and functions.
