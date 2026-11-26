# coding_adventures_perl_lexer (Dart)

Perl lexer driven by the compiled `perl.tokens` grammar (unversioned
LANG81 subset) and the LANG84 per-release tables. The Dart lane of
the Rust `perl-lexer` crate — one language definition per Perl
release (`code/grammars/perl/`) powers every implementation.

```dart
import 'package:coding_adventures_perl_lexer/perl_lexer.dart';

final tokens = tokenizePerl('print(7 - 2 * 3);\n');
print(tokens.map((t) => '${t.type}:${t.value}').join(' '));
// NAME:print LPAREN:( INT:7 MINUS:- INT:2 STAR:* INT:3 RPAREN:) SEMICOLON:; EOF:
```

## API

- `tokenizePerl(source)` lexes the pilot subset with the
  unversioned grammar; inputs over `maxPerlSourceBytes` (65536
  UTF-8 bytes) are rejected before tokenization.
- `unversionedPerlTokenGrammar()` returns the unversioned table.
- `perlReleaseTokenGrammar(release)` returns exactly one
  inventory release's table, or `null` for pending/unknown IDs
  (never a neighbour or the unversioned pair, LANG84 C4.3).
- `perlGrammarClass(release)` returns the deduplicated class
  (`g_<12 hex>`) holding a release's table, or `null`.
- `supportedPerlReleases`: the 118 paired releases in inventory
  order.

## Deliberate divergences from the reference

- Over-cap sources throw `ArgumentError` with the reference's
  exact message instead of `Err(String)` (a bad size is a
  programming error at the call site).
- Tokenization failures propagate the structured engine
  `LexerError` instead of the reference's string wrap.
- The regeneration check (compiled tables vs fresh `.tokens`
  parses) runs in the emitter at generation time rather than in
  the test suite: lane tests never read outside the package. The
  suite pins the same inventory and class relationships.

## Validation

- **Unit** (`test/perl_lexer_test.dart`): all 5 reference tests
  plus 5 lane extras (exact cap message, byte-vs-code-unit cap,
  unversioned pilot shape, positions).
- **Property** (`test/properties_test.dart`): 9 seeded properties
  over the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, positions, the exact 65536-byte cap boundary,
  valid-snippet lexing, release coverage, unknown-release nulls,
  lookup determinism.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`, every input run under the unversioned
  grammar and all 5 class tables; only `LexerError` escapes, every
  success ends with `EOF` and re-lexes deterministically.
- **Coverage**: 100% lines and functions.
