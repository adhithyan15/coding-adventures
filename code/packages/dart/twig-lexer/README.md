# coding_adventures_twig_lexer (Dart)

Twig lexer driven by the compiled `twig.tokens` grammar. The Dart
lane of the Rust `twig-lexer` crate — one language definition
(`code/grammars/twig/twig.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_twig_lexer/twig_lexer.dart';

final tokens = tokenizeTwig('(define x 42)');
print([(for (final t in tokens) t.type)]);
// [LPAREN, KEYWORD, NAME, INTEGER, RPAREN, EOF]
```

## API

- `tokenizeTwig(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `createTwigLexer(source)` returns the underlying `GrammarLexer`
  with the Twig grammar.
- `twigTokenGrammarSpec()` exposes the compiled grammar (keyword
  lists, patterns) for downstream tools — a single source of truth.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id). The reference returns `Result<_, LexerError>`
  (never panics on input, an anti-DoS design), which is exactly
  Dart's throw.

## Deliberate divergences from the reference

- None behavioral: the reference's `Result`-returning shape maps
  one-to-one onto Dart's throwing call.

## Validation

- **Unit** (`test/twig_lexer_test.dart`): all 22 reference tests plus
  the grammar spec accessor, keyword values, error positions, and
  empty input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, keyword promotion, integer
  wholeness, boolean atomicity, comment invisibility, form shape.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes.
- **Coverage**: 100% lines and functions.
