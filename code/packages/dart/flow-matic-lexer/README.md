# coding_adventures_flow_matic_lexer (Dart)

FLOW-MATIC lexer driven by the compiled `flow_matic.tokens` grammar.
The Dart lane of the Rust `flow-matic-lexer` crate — one language
definition (`code/grammars/flow_matic/flow_matic.tokens`) powers every
implementation.

```dart
import 'package:coding_adventures_flow_matic_lexer/flow_matic_lexer.dart';

final tokens = tokenizeFlowMatic('COMPARE PRODUCT-NO (A) .');
print([(for (final t in tokens) t.type)]);
// [KEYWORD, NAME, LPAREN, NAME, RPAREN, PERIOD, EOF]
```

## API

- `tokenizeFlowMatic(source)` tokenizes in one shot, returning the
  tokens with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeFlowMatic(source)` is the nullable twin: `null` instead
  of throwing (mirrors the reference's `Result`-returning
  `try_tokenize_flow_matic` with idiomatic Dart nullability).
- `createFlowMaticLexer(source)` returns the underlying `GrammarLexer`
  with the FLOW-MATIC grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id). Keywords match case-insensitively and fold to
  uppercase values while NAMEs keep their case.

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `FLOW-MATIC tokenization failed: …`, and the `Result` twin
  returns null instead of `Err(String)`: Dart lanes propagate engine
  errors (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/flow_matic_lexer_test.dart`): all 15 reference
  tests plus error positions, the nullable API shape, and empty
  input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, keyword folding over the
  grammar's own keyword table, hyphenated-name wholeness, number
  wholeness, newline invisibility, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, and both APIs
  always agree.
- **Coverage**: 100% lines and functions.
