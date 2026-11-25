# coding_adventures_lisp_lexer (Dart)

Tokenizes Lisp source code into a stream of tokens. The Dart lane of
the Rust `lisp-lexer` crate.

```dart
import 'package:coding_adventures_lisp_lexer/lisp_lexer.dart';

final tokens = tokenize('(+ 1 2)');
// LParen("("), Symbol("+"), Number("1"), Number("2"), RParen(")"), Eof("")
```

## API

- `tokenize` scans atoms (numbers incl. negatives, symbols, quoted
  strings), delimiters (`(`, `)`, `'`, `.`), and appends exactly one
  `eof`. Whitespace and `;` comments are skipped.
- `Token` carries a `TokenType` and its source text; `TokenType` has
  the reference's `displayName`s (`NUMBER`, `SYMBOL`, …).
- Failures throw `LexerError` (message + rune-offset position) with
  the reference's exact texts.

Positions count Unicode scalar values, matching the reference's
`char` walk on all scalar input. (The reference documents byte
offsets but counts `chars` — the code governs.)

## Validation

- **Unit** (`test/lisp_lexer_test.dart`): all 35 reference tests plus
  exact error positions/texts, the dot-digit quirk, the full symbol
  alphabet, display names, and value semantics.
- **Property** (`test/properties_test.dart`): fail-closed vocabulary,
  the EOF invariant, the space-join fixed point, token shapes, value
  provenance, and purity — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  (astral, lone surrogates, quotes, escapes); only `LexerError`
  escapes.
- **Coverage**: 100% lines, branches, and functions.
