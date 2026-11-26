# coding_adventures_prolog_lexer (Dart)

ISO Prolog lexer driven by the compiled `iso.tokens` grammar. The
Dart lane of the Rust `prolog-lexer` crate — one language definition
(`code/grammars/prolog/iso.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_prolog_lexer/prolog_lexer.dart';

final tokens = tokenizeIsoProlog('father(homer, bart).');
print([(for (final t in tokens) t.type)]);
// [ATOM, LPAREN, ATOM, COMMA, ATOM, RPAREN, DOT, EOF]
```

## API

- `tokenizeIsoProlog(source)` tokenizes in one shot, returning the
  tokens with grammar-name types verbatim plus a final `EOF`.
- `createIsoPrologLexer(source)` returns the underlying `GrammarLexer`
  with the ISO Prolog grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `ISO Prolog tokenization failed: …`: Dart lanes propagate
  engine errors (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/prolog_lexer_test.dart`): all 17 reference tests
  plus quoted/symbolic atom values (proven by differential probe),
  NAF/prob-sep priority, error positions, and factory-direct use.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, atom/variable split, integer
  wholeness, arrow atomicity, comment invisibility, fact shape.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over the in-lane `fuzz` package; only `LexerError` escapes.
- **Coverage**: 100% lines and functions.
