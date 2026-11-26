# coding_adventures_macronib_lexer (Dart)

MacroNib lexer — Nib's tokens plus assembler-style directives —
driven by the compiled `macronib.tokens` grammar. The Dart lane of
the Rust `macronib-lexer` crate: one language definition
(`code/grammars/macronib/macronib.tokens`) powers every
implementation.

```dart
import 'package:coding_adventures_macronib_lexer/macronib_lexer.dart';

final tokens = tokenizeMacronib('fn main() { .set x 7; }\n');
print([for (final t in tokens) t.type]);
// [fn, NAME, LPAREN, RPAREN, LBRACE, DOT_SET, NAME, INT_LIT,
//  SEMICOLON, RBRACE, EOF]
```

## API

- `tokenizeMacronib(source)` tokenizes in one shot, then promotes
  every `KEYWORD` token's type to its literal value, exactly like
  the reference's post-pass.
- `tryTokenizeMacronib(source)` is the nullable twin: `null`
  instead of throwing (mirrors the reference's `Result`-returning
  `try_tokenize_macronib` with idiomatic Dart nullability).
- `createMacronibLexer(source)` returns the raw underlying
  `GrammarLexer` — like the reference factory, it does NOT apply
  the keyword-promotion post-pass.
- Lex failures throw the engine's `LexerError` (message, line,
  column, stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of
  panicking with `MacroNib tokenization failed: …`, and the
  `Result` twin returns null instead of `Err(String)`: Dart lanes
  propagate engine errors (the `toml-lexer` precedent) rather than
  wrapping them.

## Validation

- **Unit** (`test/macronib_lexer_test.dart`): all 4 reference tests
  (including the two `.tokens` grammar-conformance tests, via the
  in-lane `grammar_tools` parser) plus table-wide keyword
  promotion, the raw factory's unpromoted output, the nullable
  twin, error positions, and empty input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, promotion completeness,
  directive atomicity, agreement with the `nib` lane on the shared
  (dotless-quoteless) surface, comment invisibility, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, both APIs always
  agree, promotion never leaks, and the `nib` lane produces the
  identical stream on shared-surface input.
- **Coverage**: 100% lines and functions.
