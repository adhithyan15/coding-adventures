# coding_adventures_jsdoc_lexer (Dart)

JSDoc lexer driven by the compiled `jsdoc.tokens` grammar. The Dart
lane of the Rust `jsdoc-lexer` crate — one language definition
(`code/grammars/jsdoc/jsdoc.tokens`) powers every implementation.

Tokenizes the INTERIOR of a `/** ... */` block comment. The
enclosing `/**` and `*/` markers and the per-line `* ` continuation
prefixes are expected to be stripped by an upstream
comment-extractor stage; leftover `* ` line-starts are skipped as a
safety net.

```dart
import 'package:coding_adventures_jsdoc_lexer/jsdoc_lexer.dart';

final tokens = tokenizeJsdoc('@param {...string} args\n');
print([for (final t in tokens) t.type]);
// [AT_TAG, LBRACE, ELLIPSIS, NAME, RBRACE, NAME, NEWLINE, EOF]
```

## API

- `tokenizeJsdoc(source)` tokenizes in one shot, returning the
  tokens with grammar-name types verbatim plus a final `EOF`.
- `createJsdocLexer(source)` returns the underlying `GrammarLexer`
  with the JSDoc grammar — the documented error path, for callers
  that handle errors explicitly. Like the reference, this lane
  offers no `Result`/nullable twin.
- Lex failures throw the engine's `LexerError` (message, line,
  column, stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of
  panicking with `JSDoc tokenization failed: …`: Dart lanes
  propagate engine errors (the `toml-lexer` precedent) rather than
  wrapping them.

## Validation

- **Unit** (`test/jsdoc_lexer_test.dart`): all 9 reference tests
  plus error positions and the factory's explicit error path.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, tag atomicity, ellipsis
  atomicity, whitespace insensitivity, leftover-star invisibility,
  factory agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes and the factory
  path always agrees.
- **Coverage**: 100% lines and functions.
