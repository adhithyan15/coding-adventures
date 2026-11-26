# coding_adventures_axiom_lexer (Dart)

Axiom lexer driven by the compiled `axiom.tokens` grammar. The Dart
lane of the Rust `axiom-lexer` crate — one language definition
(`code/grammars/axiom/axiom.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_axiom_lexer/axiom_lexer.dart';

final tokens = tokenizeAxiom('a := 3');
print([(for (final t in tokens) t.type)]);
// [NAME, ASSIGN, NUMBER, EOF]
```

## API

- `tokenizeAxiom(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeAxiom(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_tokenize_axiom` with idiomatic Dart nullability).
- `createAxiomLexer(source)` returns the underlying `GrammarLexer`
  with the Axiom grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `Axiom tokenization failed: …`, and the `Result` twin returns
  null instead of `Err(String)`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/axiom_lexer_test.dart`): all 6 in-module reference
  tests and all 56 integration tests (including the 50K-nesting,
  100K-token, and 500KB-comment scale tests), plus error positions,
  the nullable API shape, and empty input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, operator atomicity, number
  wholeness, keyword exactness, comment invisibility, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, and both APIs
  always agree.
- **Coverage**: 100% lines and functions.
