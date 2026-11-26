# coding_adventures_codabar (Dart)

Pure Codabar barcode encoder and layout. The Dart lane of the Rust
`codabar` crate (spec `barcode-symbologies-v1.md`, Codabar section).

```dart
import 'package:coding_adventures_codabar/codabar.dart';

final normalized = normalizeCodabar('40156', start: 'B', stop: 'D');
print(normalized); // 'B40156D'
final scene = layoutCodabar('40156', start: 'B', stop: 'D');
print(scene.metadata['stop']); // 'D'
```

## API

- `normalizeCodabar(data, {start, stop})` — validates and normalizes
  to a guarded string. Unguarded bodies gain guards (each defaulting
  to `A`); already-guarded input must agree with explicit options.
- `encodeCodabar` — per-character binary patterns with source indexes
  and start/data/stop roles.
- `expandCodabarRuns` — barcode runs with inter-character gaps.
- `layoutCodabar`/`drawCodabar` — paint-scene projection with quiet
  zones, inferred symbols, and `symbology`/`start`/`stop`/
  `humanReadableText` metadata.
- Errors: [InvalidCodabarInputError] with stable `errorId`
  (`input-too-long`/`invalid-character`/`invalid-guard`/
  `guard-conflict`); ill-formed host strings (lone surrogates) throw
  [ArgumentError] before validation, outside the v1 domain.

Validation precedence follows the v1 contract: the 4096-scalar cap,
then guard form, then guard options and conflicts, then the body
alphabet scan left to right. Where the reference disagrees (no cap,
full-Unicode folding, silently ignored disagreeing options on
guarded input, non-guard options spliced instead of rejected), the
spec and corpus govern — the port follows them.

The reference sets `human_readable_text` on the paint options, which
with glyph rendering disabled manifests only as scene metadata. The
portable lane entry rejects the text option, so the port writes the
`humanReadableText` metadata key directly: identical observable scenes
without font authority. Text-enabled projection reaches the lane's
specified `human-readable-text-unsupported` error.

## Validation

- **Unit** (`test/codabar_test.dart`): mirrors both reference tests,
  plus guard form, options, folding, the body alphabet, run geometry,
  scene metadata, option merging, and payload-blind errors. 18 tests.
- **Conformance** (`test/codabar_conformance_test.dart`): all 19
  Codabar corpus cases, including the hashed limit case. Registered
  in `consumers.json`.
- **Property** (`test/properties_test.dart`): framing defaults,
  guarded passthrough, run geometry, error totality, contract order,
  and code-pure messages — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile inputs and guard options against an
  independent oracle; only exact errors escape. Ad-hoc
  20,000-iteration runs over two more seeds: zero crashes.
- **Coverage**: 99%+ lines; the only misses are the unreachable null
  guard shared with the sibling lanes and the implicit enum
  constructor.
