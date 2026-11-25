# coding_adventures_cobol_lexer (Dart)

COBOL-60 lexer driven by the compiled `cobol.tokens` grammar. The
Dart lane of the Rust `cobol-lexer` crate — one language definition
(`code/grammars/cobol/cobol.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_cobol_lexer/cobol_lexer.dart';

final tokens = tokenizeCobol('000600 01  EMP-NAME     PICTURE X(20).\n');
print([for (final t in tokens) t.type]);
// [NUMBER, NAME, KEYWORD, PIC_STRING, DOT, EOF]
```

## API

- `tokenizeCobol(source)` tokenizes carded (or already free-form)
  source in one shot, returning the tokens with grammar-name types
  verbatim plus a final `EOF`.
- `tryTokenizeCobol(source)` is the nullable twin: `null` instead
  of throwing (mirrors the reference's `Result`-returning
  `try_tokenize_cobol` with idiomatic Dart nullability).
- `createCobolLexer(source)` returns the underlying `GrammarLexer`
  with the COBOL grammar and the column-strip hook registered.
- `stripCobolColumns(source)` is the pre-tokenize hook, exported
  for direct use and testing like the reference: it keeps columns
  8–72, drops `*`/`/` comment lines, and splices `-`
  continuations.
- Lex failures throw the engine's `LexerError` (message, line,
  column, stable error id). Positions refer to the STRIPPED source.

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of
  panicking with `COBOL tokenization failed: …`, and the `Result`
  twin returns null instead of `Err(String)`: Dart lanes propagate
  engine errors (the `toml-lexer` precedent) rather than wrapping
  them.
- Keyword lookup is verbatim (only exact-case reserved words
  promote), exactly like the reference: the grammar's
  `case_insensitive` magic is off in both lanes (a doc line resets
  it), while token patterns match case-insensitively in both.

## Validation

- **Unit** (`test/cobol_lexer_test.dart`): all 16 reference tests
  plus short-line, predecessor-less continuation, CRLF, error
  positions, empty input, and nullable-twin agreement.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, comment invisibility,
  continuation splicing, picture atomicity, keyword case behavior,
  API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes and both APIs
  always agree.
- **Coverage**: 100% lines and functions.
