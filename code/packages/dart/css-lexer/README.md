# coding_adventures_css_lexer (Dart)

CSS lexer driven by the compiled `css.tokens` grammar. The Dart lane
of the Rust `css-lexer` crate — one language definition
(`code/grammars/css/css.tokens`) powers every implementation. No
callback: CSS needs no group stack.

```dart
import 'package:coding_adventures_css_lexer/css_lexer.dart';

final tokens = tokenizeCss('body { color: red; }');
print([(for (final t in tokens) t.type)]);
// [IDENT, LBRACE, IDENT, COLON, IDENT, SEMICOLON, RBRACE, EOF]
```

## API

- `tokenizeCss(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `createCssLexer(source)` returns the underlying `GrammarLexer` with
  the CSS grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `CSS tokenization failed: …`: Dart lanes propagate engine
  errors (the `toml-lexer` precedent) rather than wrapping them.

## Known shared behaviors (both lanes, by construction)

- The grammar's `errors:` section (`BAD_STRING`, `BAD_URL`) is dormant
  schema: neither the Rust nor the Dart engine consults it, so
  malformed input throws / panics instead of yielding error tokens.
- Scientific notation splits: `DIMENSION`'s unit class eats the
  exponent's `e`, so `1e10` lexes as `DIMENSION(1e)` + `NUMBER(10)`
  (proven identical in Rust via differential probe) — contradicting
  both the grammar's own comment and real CSS. Logged for the grammar
  owners.

## This PR also fixes the Dart emitter

Compiling `css.tokens` failed: the emitter renders strings with
`jsonEncode`, which leaves `$` bare, and Dart reads it as
interpolation (`DOLLAR_EQUALS`, `BAD_STRING`, `BAD_URL` broke the
generated file). The emitter now escapes `$` as `\$`, with a
regression test proven to fail pre-fix. The fix reproduces the
established convention byte-for-byte (prior lanes hand-escaped the
same way).

## Validation

- **Unit** (`test/css_lexer_test.dart`): all 10 reference tests plus
  the corners the reference leaves — numeric priority, custom
  properties, unicode ranges, URL/function discrimination, CDO/CDC,
  escapes-none values, operator priority, errors with positions.
- **Property** (`test/properties_test.dart`): 8 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, rule balance, priority atomicity,
  string wholeness, trivia invisibility.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over the in-lane `fuzz` package; only `LexerError` escapes.
- **Coverage**: 100% lines and functions.
