# coding_adventures_code_128 (Dart)

Pure Code 128 Code Set B barcode encoder and layout. The Dart lane of
the Rust `code128` crate (spec `barcode-symbologies-v1.md`, Code
128-B section).

```dart
import 'package:coding_adventures_code_128/code_128.dart';

final normalized = normalizeCode128B('HELLO'); // 'HELLO'
final scene = layoutCode128('HELLO');
print(scene.metadata['checksum']); // '40'
```

## API

- `normalizeCode128B` — validates the printable-ASCII domain (empty
  input is valid) and returns the input unchanged.
- `computeCode128Checksum` — the weighted checksum over data values
  (104 plus value × 1-based position, modulo 103).
- `encodeCode128B` — framed symbols (start + data + checksum + stop)
  with values, patterns, and roles.
- `expandCode128Runs` — barcode runs concatenated gaplessly (every
  pattern starts with a bar and, except stop, ends with a space).
- `layoutCode128`/`drawCode128` — paint-scene projection with quiet
  zones, explicit symbols, and `symbology`/`codeSet`/`checksum`/
  `humanReadableText` metadata.
- Errors: [InvalidCode128InputError] with stable `errorId`
  (`input-too-long`/`invalid-character`); ill-formed host strings
  (lone surrogates) throw [ArgumentError] before validation, outside
  the v1 domain.

Validation precedence follows the v1 contract: the 4096-scalar cap,
then the alphabet scan. The reference has no cap; the spec and corpus
require it, so the port enforces it.

The reference sets `human_readable_text` on the paint options, which
with glyph rendering disabled manifests only as scene metadata. The
portable lane entry rejects the text option, so the port writes the
`humanReadableText` metadata key directly: identical observable scenes
without font authority. Text-enabled projection reaches the lane's
specified `human-readable-text-unsupported` error.

## Validation

- **Unit** (`test/code_128_test.dart`): mirrors the reference test,
  plus checksum weights, domain edges, surrogate rejection, framing,
  gapless geometry, scene metadata, option merging, and payload-blind
  errors. 14 tests.
- **Conformance** (`test/code_128_conformance_test.dart`): all 19
  Code 128-B corpus cases, including symbol values and the hashed
  limit case. Registered in `consumers.json`.
- **Property** (`test/properties_test.dart`): check-digit oracle,
  framing, gapless geometry, error totality, cap precedence, and
  code-pure messages — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile decodings against an independent oracle;
  only exact errors escape. Ad-hoc 20,000-iteration runs over two
  more seeds: zero crashes.
- **Coverage**: 99%+ lines; the only misses are the unreachable null
  guard shared with the sibling lanes and the implicit enum
  constructor.
