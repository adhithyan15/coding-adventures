# coding_adventures_code_39 (Dart)

Pure Code 39 barcode encoder and layout. The Dart lane of the Rust
`code39` crate (spec `barcode-symbologies-v1.md`, Code 39 section).

```dart
import 'package:coding_adventures_code_39/code_39.dart';

final normalized = normalizeCode39('hello-1'); // 'HELLO-1'
final scene = layoutCode39('hello-1');
print(scene.metadata['encodedText']); // 'HELLO-1'
```

## API

- `normalizeCode39` — folds ASCII lowercase, rejects `*` and
  out-of-alphabet characters left to right (empty input is valid).
- `encodeCode39Char`/`encodeCode39` — per-character narrow/wide
  patterns, wrapped in start/stop delimiters.
- `expandCode39Runs` — barcode runs with inter-character gaps.
- `layoutCode39`/`drawCode39` — paint-scene projection with quiet
  zones, inferred symbols, and `symbology`/`encodedText`/
  `humanReadableText` metadata.
- Errors: [InvalidCode39InputError] with stable `errorId`
  (`input-too-long`/`invalid-character`/`reserved-character`);
  ill-formed host strings (lone surrogates) throw [ArgumentError]
  before validation, outside the v1 domain.

Validation precedence follows the v1 contract: the 4096-scalar cap
first, then one left-to-right scan reporting `reserved-character`
for `*` at its position before the generic `invalid-character`. Only
ASCII `a`-`z` folds — the reference folds with full Unicode
`to_uppercase` (so `ß` becomes `SS`), but the spec and corpus
require ASCII-only, and the port follows them.

The reference sets `human_readable_text` on the paint options, which
with glyph rendering disabled manifests only as scene metadata. The
portable lane entry rejects the text option, so the port writes the
`humanReadableText` metadata key directly: identical observable scenes
without font authority. Text-enabled projection reaches the lane's
specified `human-readable-text-unsupported` error.

## Validation

- **Unit** (`test/code_39_test.dart`): mirrors the portable reference
  tests, plus folding, scan order, the full 43-character alphabet,
  run geometry, scene metadata, option merging, and payload-blind
  errors. 17 tests.
- **Conformance** (`test/code_39_conformance_test.dart`): all 11 Code
  39 corpus cases, including the 4096-input hashed limit case.
  Registered in `consumers.json`.
- **Property** (`test/properties_test.dart`): fold laws, framing,
  run geometry, error totality, leftmost-offender scan order, and
  code-pure messages — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile decodings against an independent oracle;
  only exact errors escape. Ad-hoc 20,000-iteration runs over two
  more seeds: zero crashes.
- **Coverage**: 99%+ lines; the only misses are the unreachable null
  guard shared with the sibling lanes and the implicit enum
  constructor.
