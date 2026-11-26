# coding_adventures_upc_a (Dart)

Pure UPC-A barcode encoder and layout. The Dart lane of the Rust
`upc-a` crate (spec `barcode-symbologies-v1.md`, UPC-A section).

```dart
import 'package:coding_adventures_upc_a/upc_a.dart';

final normalized = normalizeUpcA('03600029145'); // '036000291452'
final scene = layoutUpcA('03600029145');
print(scene.metadata?['humanReadableText']); // '036000291452'
```

## API

- `normalizeUpcA` — validates and normalizes to 12 digits (computes
  the check digit for 11-digit input, verifies it for 12-digit input).
- `computeUpcACheckDigit` — the GS1 check digit for an 11-digit payload.
- `encodeUpcA` — per-digit L/R patterns with source indexes and roles.
- `expandUpcARuns` — 95 modules of barcode runs with guards.
- `layoutUpcA`/`drawUpcA` — paint-scene projection with quiet zones,
  explicit symbols, and `symbology`/`humanReadableText` metadata.
- Errors: [InvalidUpcAInputError] with stable `errorId`
  (`input-too-long`/`invalid-length`/`invalid-character`/
  `invalid-check-digit`); ill-formed host strings (lone surrogates)
  throw [ArgumentError] before validation, outside the v1 domain.

Validation precedence follows the v1 contract: too-long, then length,
then alphabet left-to-right, then checksum. Where the reference
disagrees (it scans characters before length and has no 4096 cap), the
spec and corpus govern — the port follows them, like the ITF lane.

The reference sets `human_readable_text` on the paint options, which
with glyph rendering disabled manifests only as scene metadata. The
portable lane entry rejects the text option, so the port writes the
`humanReadableText` metadata key directly: identical observable scenes
without font authority. Text-enabled projection reaches the lane's
specified `human-readable-text-unsupported` error.

## Validation

- **Unit** (`test/upc_a_test.dart`): mirrors both reference tests,
  plus the full precedence matrix, surrogate rejection, both digit
  tables, run geometry, scene metadata, option merging, and
  payload-blind errors. 15 tests.
- **Conformance** (`test/upc_a_conformance_test.dart`): all 12 UPC-A
  corpus cases (normalized text, module strings, run lengths,
  checksums, error ids). Registered in `consumers.json`.
- **Property** (`test/properties_test.dart`): check-digit oracle
  agreement, normalize fixed points, 95-module alternating geometry,
  encode agreement, error totality over hostile inputs, precedence,
  and code-pure messages — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile decodings (Latin-1, malformed UTF-8, lone
  surrogates, over-limit repeats) against an independent oracle;
  only exact errors escape. Ad-hoc 20,000-iteration runs over two
  more seeds: zero crashes.
- **Coverage**: 99.2% lines, 94.4% functions (the 2 misses are the
  unreachable null guard shared with the ITF lane and the implicit
  enum constructor).
