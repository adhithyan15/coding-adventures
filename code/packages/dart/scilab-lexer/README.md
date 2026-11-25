# coding_adventures_scilab_lexer (Dart)

Scilab lexer driven by the compiled `scilab.tokens` grammar. The
Dart lane of the Rust `scilab-lexer` crate — one language definition
(`code/grammars/scilab/scilab.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_scilab_lexer/scilab_lexer.dart';

final tokens = tokenizeScilab("x = 'hi';\n");
print([for (final t in tokens) t.type]);
// [NAME, EQ, STRING, SEMICOLON, NEWLINE, EOF]
```

## API

- `tokenizeScilab(source)` tokenizes in one shot, returning the
  tokens with grammar-name types verbatim plus a final `EOF`.
- `tryTokenizeScilab(source)` is the nullable twin: `null` instead
  of throwing (mirrors the reference's `Result`-returning
  `try_tokenize_scilab` with idiomatic Dart nullability).
- `createScilabLexer(source)` returns the underlying `GrammarLexer`
  with the Scilab grammar and all four hooks registered.
- Lex failures throw the engine's `LexerError` (message, line,
  column, stable error id).

## Hooks

The defining problem is the `'` transpose-vs-string ambiguity,
resolved exactly like the reference:

- Pre-tokenize quote protection walks the source once, leaving `'`
  after a value in place (a transpose) and replacing every string
  literal with a `` `N` `` placeholder recorded in a literal table.
  `//` and `/* */` comments and `"..."` strings pass through
  untouched, invisible to the value-context tracking.
- Post-tokenize placeholder restoration re-labels each
  `STRING_PLACEHOLDER` as `STRING` with its decoded content — but
  only when the recorded (line, column) matches, so a verbatim
  `` `N` `` typed in ordinary source never hijacks an unrelated
  literal's content.
- Post-tokenize double-quote escape collapse turns the engine's
  `DQ_STRING` into `STRING`, collapsing the `""` escape.
- Post-tokenize paren-newline dropping removes `NEWLINE` inside
  open `( )` only (newlines in `[ ]`/`{ }` separate matrix rows
  and are kept), saturating at zero on unbalanced `)`.

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of
  panicking with `Scilab tokenization failed: …`, and the `Result`
  twin returns null instead of `Err(String)`: Dart lanes propagate
  engine errors (the `toml-lexer` precedent) rather than wrapping
  them.
- Value-terminator detection uses ASCII letters/digits where the
  reference uses Unicode `is_alphanumeric` (Dart has no equivalent
  predicate). Every value token in this grammar is ASCII-only, so a
  non-ASCII character is a lex error in both lanes either way —
  protection never consumes it, and both lanes fail closed on it.

## Validation

- **Unit** (`test/scilab_lexer_test.dart`): all 35 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, an independent paren-depth hook
  mirror over quote-free soup, operator atomicity, percent-constant
  atomicity, comment invisibility, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, both APIs always
  agree, and an independent saturating depth replay proves no
  NEWLINE survives at paren depth.
- **Coverage**: 100% lines and functions.
