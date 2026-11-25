# coding_adventures_oct_lexer (Dart)

Oct lexer driven by the compiled `oct.tokens` grammar. The Dart lane
of the Rust `oct-lexer` crate — one language definition
(`code/grammars/oct/oct.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_oct_lexer/oct_lexer.dart';

final tokens = tokenizeOct('fn main() { let x: u8 = 5; }');
print([(for (final t in tokens) t.type)]);
// [fn, NAME, LPAREN, RPAREN, LBRACE, let, NAME, COLON, NAME, EQ,
//  INT_LIT, SEMICOLON, RBRACE, EOF]
```

## API

- `tokenizeOct(source)` tokenizes in one shot, then promotes every
  `KEYWORD` token's type to its literal value — the same convention
  as Nib's lexer, so quoted-keyword grammar rules match by type name.
- `createOctLexer(source)` returns the underlying `GrammarLexer`
  with the Oct grammar (pre-promotion tokens).
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `Oct tokenization failed: …`: Dart lanes propagate engine
  errors (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/oct_lexer_test.dart`): all 5 reference tests plus
  an exact declaration stream, table-wide keyword promotion, error
  positions, empty input, and factory-direct use.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, keyword promotion, integer/name
  wholeness, declaration shape, promotion totality.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, and no bare
  `KEYWORD` ever survives the post-pass.
- **Coverage**: 100% lines and functions.
