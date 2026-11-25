# coding_adventures_asciidoc_parser (Dart)

AsciiDoc to Document AST parser ([spec](../../../specs/TE03-asciidoc-parser.md)).
The Dart lane of the Rust `asciidoc-parser` crate.

```dart
import 'package:coding_adventures_asciidoc_parser/asciidoc_parser.dart';

final doc = parse('= Hello\n\nWorld *with* strong.\n');
print(doc.children[0].type); // heading
```

## API

- `parse(text)` runs both phases and returns the root `DocumentNode`:
  the block state machine (headings, paragraphs, lists, fences, quotes,
  comments, attribute lines), then the inline scanner (spans, macros,
  cross-references, URLs, breaks) over each block's raw content.
- Node types are re-exported from `coding_adventures_document_ast`,
  mirroring the reference's `pub use document_ast::*`.
- The parser is total and lenient: unclosed blocks run to EOF, unclosed
  markup stays literal text, and `parse` never throws.

## Deliberate divergences from the reference

- None behavioral: every index, slice, and advance matches the Rust
  lane bit for bit (the inline scanner works over UTF-8 bytes, exactly
  like the reference). Two packaging notes: the reference's `VERSION`
  const has no mirror (the version lives in pubspec.yaml on every Dart
  lane, by convention), and the reference's defensive invalid-UTF-8
  fallback is omitted as unreachable by construction (Dart strings are
  always valid Unicode).
- Known Python/Ruby-lane divergences from Rust (this lane follows Rust;
  logged as follow-ups): their closed passthrough appends `\n` (Rust:
  none, pinned by test), they drop empty unclosed blocks at EOF (Rust
  emits them, the spec's literal reading), their unclosed passthrough
  appends `\n` (Rust: plain join), they clamp `=======` to level 6
  (Rust: paragraph text), and they skip `///` comments (Rust: paragraph
  text). No lane implements the spec's same-token fence-closing
  sentence — all close on any long-enough run.

## Validation

- **Unit** (`test/asciidoc_parser_test.dart`): all 34 behavioral
  reference tests plus the corners the reference leaves — fence
  leniency, every EOF shape, pending-language lifetime, level-skipping
  and type-change list behavior, constrained-boundary failures, macro
  fallbacks, URL terminator exactness, break stacking, CRLF
  preservation, and Unicode.
- **Property** (`test/properties_test.dart`): 8 seeded properties over
  the in-lane `pbt` package — totality plus a whole-tree AST invariant
  sweep, determinism, blank-empty, heading singularity, code
  round-trip, marker wrapping, break splitting, dispatch agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over the in-lane `fuzz` package across both phases; any throw or
  invariant violation is a crash.
- **Coverage**: 100% lines. The single uncovered function record is the
  `_State` enum's implicit constructor, a coverage artifact with no
  callable body.
