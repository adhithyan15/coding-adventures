# coding_adventures_xml_lexer (Dart)

XML lexer driven by the compiled `xml.tokens` grammar with a
callback-driven group stack. The Dart lane of the Rust `xml-lexer`
crate — one language definition
(`code/grammars/xml/xml.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_xml_lexer/xml_lexer.dart';

final tokens = tokenizeXml('<p>Hello &amp; welcome</p>');
print([(for (final t in tokens) t.type)]);
// [OPEN_TAG_START, TAG_NAME, TAG_CLOSE, TEXT, ENTITY_REF, TEXT,
//  CLOSE_TAG_START, TAG_NAME, TAG_CLOSE, EOF]
```

## API

- `tokenizeXml(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `createXmlLexer(source)` returns the underlying `GrammarLexer` with
  the XML grammar and callback wired up.
- `xmlOnToken(token, ctx)` is the group-stack callback: tag openers
  push `tag`, closers pop; comment/CDATA/PI starts push their group
  and freeze skips (their whitespace is content); ends pop and
  re-enable; `PI_TARGET` swaps `pi` for `pi_body`.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `XML tokenization failed: …`: Dart lanes propagate engine
  errors (the `toml-lexer` precedent) rather than wrapping them,
  preserving line, column, and error id for callers.

## This PR also fixes the shared definition

Porting caught a portability violation in `xml.tokens`: `CDATA_BRACK`
used a lone `]` regex, which Dart's `RegExp` (Unicode mode) rejects at
compile time while Rust/Go/PCRE accept it as a literal. The definition
now spells it `[\]]` (identical match language in every engine) and the
portability note bans lone `]` alongside lookaround and alternation.
All four generated artifacts were recompiled with their sanctioned
runners (Rust, Go, TypeScript, Dart) and every affected suite passes:
Rust 38/38, Go lane + engine differentials, TS 35/35, Dart 54/54.

Regenerating the Go file exposed a second bug: the Go emitter iterated
the groups map in random order, so its output was never
byte-reproducible. It now sorts group names (with a regression test
asserting 25 identical emissions plus sorted order).

## Validation

- **Unit** (`test/xml_lexer_test.dart`): all 38 reference tests plus
  the corners the reference leaves — lexer errors (position, id),
  unclosed constructs at EOF, `NEWLINE` inside skip-disabled groups,
  quote-vs-structure precedence, positions, and markup-looking text.
- **Property** (`test/properties_test.dart`): 8 seeded properties over
  the in-lane `pbt` package — closed failure vocabulary, EOF shape,
  determinism, monotonic positions, delimiter balance, entity
  exactness, attribute quotes, body reassembly.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over the in-lane `fuzz` package; only `LexerError` escapes, and
  every success re-parses identically.
- **Coverage**: 100% lines and functions.
