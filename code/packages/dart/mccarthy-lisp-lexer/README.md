# coding_adventures_mccarthy_lisp_lexer (Dart)

McCarthy 1960 Lisp lexer driven by the compiled
`mccarthy_lisp.tokens` grammar. The Dart lane of the Rust
`mccarthy-lisp-lexer` crate — one language definition
(`code/grammars/mccarthy_lisp/mccarthy_lisp.tokens`) powers every
implementation.

```dart
import 'package:coding_adventures_mccarthy_lisp_lexer/mccarthy_lisp_lexer.dart';

final tokens = tokenizeMcCarthy("(CAR '(A B C))");
print(tokens.map((t) => '${t.type}:${t.value}').join(' '));
// LPAREN:( SYMBOL:CAR QUOTE:' LPAREN:( SYMBOL:A SYMBOL:B SYMBOL:C RPAREN:) RPAREN:) EOF:
```

## API

- `tokenizeMcCarthy(source)` lexes in one shot; the final token is
  always `EOF`. Whitespace and `;` line comments are skipped.
- `createMcCarthyLexer(source)` returns the underlying
  `GrammarLexer`.
- `mccarthyTokenGrammarSpec()` returns the compiled grammar for
  tooling introspection (the reference caches it in a `OnceLock`;
  a Dart top-level `final` is already initialized exactly once).

## Deliberate divergences from the reference

- Tokenization failures throw the structured engine `LexerError`
  instead of returning `Err(LexerError)`: Dart lanes propagate
  errors rather than wrapping them.

## Validation

- **Unit** (`test/mccarthy_lisp_lexer_test.dart`): all 11 reference
  tests plus 3 lane extras (factory agreement, mixed
  integer/dotted shapes, positions).
- **Property** (`test/properties_test.dart`): 7 seeded properties
  over the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, positions, the all-uppercase rule (lowercase
  outside comments always fails), valid-program lexing, factory
  agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, every success
  ends with `EOF` and re-lexes deterministically.
- **Coverage**: 100% lines and functions.
