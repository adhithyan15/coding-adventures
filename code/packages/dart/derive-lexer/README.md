# coding_adventures_derive_lexer (Dart)

Derive lexer driven by the compiled `derive.tokens` grammar. The
Dart lane of the Rust `derive-lexer` crate — one language definition
(`code/grammars/derive/derive.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_derive_lexer/derive_lexer.dart';

final tokens = tokenizeDerive('F(x) := x^2 + 1\n');
print([for (final t in tokens) t.type]);
// [NAME, LPAREN, NAME, RPAREN, ASSIGN, NAME, POWER, NUMBER, PLUS,
//  NUMBER, NEWLINE, EOF]
```

## API

- `tokenizeDerive(source)` tokenizes in one shot, returning the
  tokens with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeDerive(source)` is the nullable twin: `null` instead
  of throwing (mirrors the reference's `Result`-returning
  `try_tokenize_derive` with idiomatic Dart nullability).
- `createDeriveLexer(source)` returns the underlying `GrammarLexer`
  with the Derive grammar and the bracket-interior newline hook
  registered.
- `dropBracketedNewlines(tokens)` is the post-tokenize hook: it drops
  `NEWLINE` tokens inside open `(` or `[` (one combined depth;
  Derive has no `{ }`), saturating at zero on unbalanced closers.
- Lex failures throw the engine's `LexerError` (message, line,
  column, stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of
  panicking with `Derive tokenization failed: …`, and the `Result`
  twin returns null instead of `Err(String)`: Dart lanes propagate
  engine errors (the `toml-lexer` precedent) rather than wrapping
  them.

## Validation

- **Unit** (`test/derive_lexer_test.dart`): all 13 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, an independent hook mirror over
  bracket soup, operator atomicity, keyword case-sensitivity,
  brace rejection, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, both APIs always
  agree, and an independent saturating depth replay proves no
  NEWLINE survives at bracket depth.
- **Coverage**: 100% lines and functions.
