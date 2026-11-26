# coding_adventures_macrooct_lexer (Dart)

MacroOct lexer — Oct's tokens plus preprocessor directives —
driven by the compiled `macrooct.tokens` grammar. The Dart lane of
the Rust `macrooct-lexer` crate: one language definition
(`code/grammars/macrooct/macrooct.tokens`) powers every
implementation.

```dart
import 'package:coding_adventures_macrooct_lexer/macrooct_lexer.dart';

final tokens = tokenizeMacrooct('@if 1\nfn main() {}\n@end\n');
print([for (final t in tokens) t.type]);
// [AT_IF, INT_LIT, fn, NAME, LPAREN, RPAREN, LBRACE, RBRACE,
//  AT_END, EOF]
```

## API

- `tokenizeMacrooct(source)` tokenizes in one shot, then promotes
  every `KEYWORD` token's type to its literal value, exactly like
  the reference's post-pass.
- `tryTokenizeMacrooct(source)` is the nullable twin: `null`
  instead of throwing (mirrors the reference's `Result`-returning
  `try_tokenize_macrooct` with idiomatic Dart nullability). Use it
  for anything reachable from untrusted input, like an `@include`d
  file's text.
- `createMacrooctLexer(source)` returns the raw underlying
  `GrammarLexer` — like the reference factory, it does NOT apply
  the keyword-promotion post-pass.
- Lex failures throw the engine's `LexerError` (message, line,
  column, stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of
  panicking with `MacroOct tokenization failed: …`, and the
  `Result` twin returns null instead of `Err(String)`: Dart lanes
  propagate engine errors (the `toml-lexer` precedent) rather than
  wrapping them.

## Validation

- **Unit** (`test/macrooct_lexer_test.dart`): all 11 reference tests
  (including the PREP01 drift-guard and compiled-grammar
  conformance tests, via the in-lane `grammar_tools` parser) plus
  the doc example, table-wide keyword promotion, the raw factory's
  unpromoted output, the nullable twin, error positions, and empty
  input.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, promotion completeness,
  directive atomicity, agreement with the `oct` lane on the shared
  (`@`-less, quoteless) surface, comment invisibility, API
  agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, both APIs always
  agree, promotion never leaks, and the `oct` lane produces the
  identical stream on shared-surface input.
- **Coverage**: 100% lines and functions.
