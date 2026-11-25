# coding_adventures_apl_lexer (Dart)

APL lexer driven by the compiled `apl.tokens` grammar (the
historical-core subset, MA05 §4). The Dart lane of the Rust
`apl-lexer` crate — one language definition
(`code/grammars/apl/apl.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_apl_lexer/apl_lexer.dart';

final tokens = tokenizeApl('A←⍳5');
print([(for (final t in tokens) t.type)]);
// [NAME, ARROW, IOTA, NUMBER, EOF]
```

## API

- `tokenizeApl(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `createAplLexer(source)` returns the underlying `GrammarLexer` with
  the APL grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `APL tokenization failed: …`: Dart lanes propagate engine
  errors (the `toml-lexer` precedent) rather than wrapping them.

## This PR also fixes the Dart `.tokens` parser

Compiling `apl.tokens` failed: the literal scanner skipped `\`-escapes
when hunting the closing quote, so `SCAN = "\"` read as unclosed.
Literals are escape-blind by construction (no lane unescapes bodies —
Rust uses `rest.find('"')`, Go uses `strings.Index`), so the Dart
scanner now takes the first `"` too, yielding the intended
one-backslash pattern. Regression test included; grammar-tools suite
16/16.

## Validation

- **Unit** (`test/apl_lexer_test.dart`): all 10 reference integration
  tests plus single-glyph pins, LF/CRLF newline shapes, high-minus
  forms, error positions, and empty input.
- **Differential** (throwaway probe, removed after): all 41 unit-test
  inputs run through both lanes — token streams byte-identical
  (type, value, EOF) across Rust and Dart.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, glyph atomicity, number/name
  wholeness, comment invisibility, reduce shape.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over the in-lane `fuzz` package; only `LexerError` escapes.
- **Coverage**: 100% lines and functions.
