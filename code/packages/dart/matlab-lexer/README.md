# coding_adventures_matlab_lexer (Dart)

MATLAB lexer driven by the compiled `matlab.tokens` grammar. The
Dart lane of the Rust `matlab-lexer` crate — one language definition
(`code/grammars/matlab/matlab.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_matlab_lexer/matlab_lexer.dart';

final tokens = tokenizeMatlab("A' * B'\n");
print([for (final t in tokens) t.type]);
// [NAME, TRANSPOSE, STAR, NAME, TRANSPOSE, NEWLINE, EOF]
```

## API

- `tokenizeMatlab(source)` tokenizes in one shot, returning the
  tokens with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeMatlab(source)` is the nullable twin: `null` instead
  of throwing (mirrors the reference's `Result`-returning
  `try_tokenize_matlab` with idiomatic Dart nullability).
- `createMatlabLexer(source)` returns the underlying `GrammarLexer`
  with the MATLAB grammar and all four hooks registered.
- `stripBlockComments(source)` is the first pre-tokenize hook,
  exported for direct use like the reference.
- Lex failures throw the engine's `LexerError` (message, line,
  column, stable error id).

## Hooks

The defining problem is the `'` transpose-vs-char-array ambiguity,
resolved exactly like the reference:

- Pre-tokenize block-comment stripping blanks alone-on-line `%{` /
  `%}` blocks to newlines (line numbers stay aligned).
- Pre-tokenize quote protection leaves `'` after a value in place
  (a transpose) and replaces every char-array literal with a `` `N` ``
  placeholder recorded in a literal table, while splicing `...`
  continuations and passing `%` comments and `"..."` strings
  through untouched.
- Post-tokenize placeholder restoration re-labels each `CHARARRAY`
  as `STRING` with its decoded content — unconditionally, like the
  reference (an unresolvable placeholder still reads as `STRING`
  for the parser to reject on shape).
- Post-tokenize paren-newline dropping removes `NEWLINE` inside
  open `( )` only (newlines in `[ ]`/`{ }` separate matrix rows
  and are kept), saturating at zero on unbalanced `)`.

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of
  panicking with `MATLAB tokenization failed: …`, and the `Result`
  twin returns null instead of `Err(String)`: Dart lanes propagate
  engine errors (the `toml-lexer` precedent) rather than wrapping
  them.
- Value-terminator detection uses ASCII letters/digits where the
  reference uses Unicode `is_alphanumeric` (Dart has no equivalent
  predicate). Every value token in this grammar is ASCII-only, so a
  non-ASCII character is a lex error in both lanes either way —
  protection never consumes it, and both lanes fail closed on it.

## Validation

- **Unit** (`test/matlab_lexer_test.dart`): all 20 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, an independent saturating
  paren-depth hook mirror over quote-free soup, operator atomicity,
  block-comment stripping, continuation splicing, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, both APIs always
  agree, and an independent saturating depth replay proves no
  NEWLINE survives at paren depth.
- **Coverage**: 100% lines and functions.
