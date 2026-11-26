# coding_adventures_macsyma_lexer (Dart)

MACSYMA lexer driven by the compiled `macsyma.tokens` grammar. The
Dart lane of the Rust `macsyma-lexer` crate — one language definition
(`code/grammars/macsyma/macsyma.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_macsyma_lexer/macsyma_lexer.dart';

final tokens = tokenizeMacsyma('f(x) := x ** 2;');
print([(for (final t in tokens) t.type)]);
// [NAME, LPAREN, NAME, RPAREN, COLONEQ, NAME, STAREQ, NUMBER, SEMI, EOF]
```

## API

- `tokenizeMacsyma(source)` tokenizes in one shot, returning the
  tokens with grammar-name types verbatim plus a final `EOF`.
- `createMacsymaLexer(source)` returns the underlying `GrammarLexer`
  with the MACSYMA grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `MACSYMA tokenization failed: …`: Dart lanes propagate engine
  errors (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/macsyma_lexer_test.dart`): all 5 reference tests
  plus power/arrow singles, error positions, empty input, and
  factory-direct use.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, operator atomicity, number
  wholeness, keyword exactness, percent-name wholeness, terminator
  distinctness.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes.
- **Coverage**: 100% lines and functions.
