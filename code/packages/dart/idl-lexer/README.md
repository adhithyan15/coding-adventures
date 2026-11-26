# coding_adventures_idl_lexer (Dart)

IDL lexer driven by the compiled `idl.tokens` grammar. The Dart lane
of the Rust `idl-lexer` crate — one language definition
(`code/grammars/idl/idl.tokens`) powers every implementation. No
callback, no hooks: the grammar is entirely declarative (notably, `/`
is always plain division; the `/KEYWORD` shorthand is the parser's
problem, per the reference's documented finding).

```dart
import 'package:coding_adventures_idl_lexer/idl_lexer.dart';

final tokens = tokenizeIdl("PRINT, 'hello'");
print([(for (final t in tokens) (t.type, t.value))]);
// [(NAME, PRINT), (COMMA, ,), (STRING, hello), (EOF, )]
```

## API

- `tokenizeIdl(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeIdl(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_tokenize_idl` with idiomatic Dart nullability).
- `createIdlLexer(source)` returns the underlying `GrammarLexer` with
  the IDL grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id). Keywords fold to uppercase values (`if` →
  `KEYWORD("IF")`) while `NAME`s keep their exact case.

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `IDL tokenization failed: …`, and the `Result` twin returns
  null instead of `Err(String)`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/idl_lexer_test.dart`): all 5 in-module reference
  tests and all 53 integration tests (two exact duplicates share one
  mirror each), plus error positions, the nullable API shape, keyword
  boundaries, and empty input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, keyword folding over the
  grammar's own keyword table, name case preservation, string
  wholeness, comment invisibility, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over the in-lane `fuzz` package; only `LexerError` escapes, and the
  two APIs always agree.
- **Coverage**: 100% lines and functions.
