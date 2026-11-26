# coding_adventures_nib_lexer (Dart)

Nib lexer driven by the compiled `nib.tokens` grammar. The Dart lane
of the Rust `nib-lexer` crate — one language definition
(`code/grammars/nib/nib.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_nib_lexer/nib_lexer.dart';

final tokens = tokenizeNib('fn main() { let x: u4 = 7; }');
print([(for (final t in tokens) t.type)]);
// [fn, NAME, LPAREN, RPAREN, LBRACE, let, NAME, COLON, NAME, EQ,
//  INT_LIT, SEMICOLON, RBRACE, EOF]
```

## API

- `tokenizeNib(source)` tokenizes in one shot, then promotes every
  `KEYWORD` token's type to its literal value (`KEYWORD("fn")`
  becomes `fn("fn")`), exactly like the reference's post-pass.
- `createNibLexer(source)` returns the underlying `GrammarLexer`
  with the Nib grammar (pre-promotion tokens).
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `Nib tokenization failed: …`: Dart lanes propagate engine
  errors (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/nib_lexer_test.dart`): all 4 reference tests plus
  table-wide keyword promotion, error positions, empty input, and
  factory-direct use.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, keyword promotion, shift
  atomicity, integer/name wholeness, declaration shape.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, and no bare
  `KEYWORD` ever survives the post-pass.
- **Coverage**: 100% lines and functions.
