# coding_adventures_ean_13 (Dart)

Pure EAN-13 barcode encoder and layout. The Dart lane of the Rust
`ean-13` crate (spec `barcode-symbologies-v1.md`, EAN-13 section).

```dart
import 'package:coding_adventures_ean_13/ean_13.dart';

final normalized = normalizeEan13('400638133393'); // '4006381333931'
final scene = layoutEan13('400638133393');
print(scene.metadata['leftParity']); // 'LGLLGG'
```

## API

- `normalizeEan13` — validates and normalizes to 13 digits (computes
  the check digit for 12-digit input, verifies it for 13-digit input).
- `computeEan13CheckDigit` — the GS1 check digit for a 12-digit
  payload (weights 3/1 from the right, starting with 3).
- `leftParityPattern` — the left-half L/G parity mix selected by the
  leading digit (which emits no bars of its own).
- `encodeEan13` — twelve per-digit L/G/R patterns with source indexes
  and roles (indexes 1-12).
- `expandEan13Runs` — 95 modules of barcode runs with guards.
- `layoutEan13`/`drawEan13` — paint-scene projection with quiet zones,
  explicit symbols, and `symbology`/`humanReadableText`/`leadingDigit`/
  `leftParity` metadata.
- Errors: [InvalidEan13InputError] with stable `errorId`
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
specified `human-readable-text-unsupported` error. The reference also
builds a zero-module leading-digit descriptor and filters it before
layout; the lane rejects zero-module descriptors, so the port never
creates it — identical symbols either way.

## Validation

- **Unit** (`test/ean_13_test.dart`): mirrors the reference test,
  plus the full precedence matrix, surrogate rejection, all ten
  parity mixes, run geometry, scene metadata, option merging, and
  payload-blind errors. 15 tests.
- **Conformance** (`test/ean_13_conformance_test.dart`): all 20 EAN-13
  corpus cases (normalized text, module strings, run lengths,
  checksums, parity, error ids). Registered in `consumers.json`.
- **Property** (`test/properties_test.dart`): check-digit oracle
  agreement, normalize fixed points, parity selection, 95-module
  geometry, error totality, precedence, and code-pure messages —
  fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile decodings (Latin-1, malformed UTF-8, lone
  surrogates, over-limit repeats) against an independent oracle;
  only exact errors escape. Ad-hoc 20,000-iteration runs over two
  more seeds: zero crashes.
- **Coverage**: 99%+ lines; the only misses are the unreachable null
  guard shared with the ITF/UPC-A lanes and the implicit enum
  constructor.
