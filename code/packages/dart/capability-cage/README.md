# coding_adventures_capability_cage (Dart)

Capability manifests, taxonomy, and target matching. The Dart lane of the
Rust `capability-cage` crate (slice 1 of the port: taxonomy, manifest
loading and validation; audit logging, backends, and secure wrappers
follow in later slices).

```dart
import 'package:coding_adventures_capability_cage/coding_adventures_capability_cage.dart';

final manifest = Manifest.loadFromString('''
{"version": 1, "capabilities": [
  {"category": "fs", "action": "read", "target": "package:/fixtures/*"}
]}''');

manifest.check(Category.fs, Action.read, 'package:/fixtures/a.json'); // ok
manifest.check(Category.fs, Action.read, '/tmp/secret.txt'); // throws
```

## API

- [Category]/[Action] — the 11×17 taxonomy with wire forms, parsing,
  and [isValidCombination] (25 valid pairs, checked exhaustively against
  the shared `taxonomy.json` fixture).
- [Capability] — immutable declaration with required justification and
  optional read/write-separation annotations; validates pair and target.
- [Manifest] — `loadFromString`/`loadFromFile` (`required_capabilities.json`
  shape, integer version 1), `has`/`check` triple coverage with glob
  targets, `capabilitySurfaceSummary`, and read/write-separation
  validation (`checked` constructor, `validateReadWriteSeparation`).
- [matchTarget] — glob matcher: `*` (one component), `**` (any depth),
  in-segment stars, `host:port` with per-side stars, backslash
  normalization.
- Errors mirror the reference texts exactly: [CapabilityViolationError],
  [InvalidCombination] (`UnsupportedPair`/`EmptyTarget`/
  `InvalidTargetFormat`), [ManifestError] (`Parse`/`Schema`/
  `InvalidCombination`/`Io`/`RwsViolation`).

Two deliberate notes: the checklist omits the reference's provably
redundant suffix-overlap guard in segment matching (documented with the
proof); display strings quote names plainly where the reference uses
Rust debug escapes (same shape for ordinary names).

## Validation

- **Unit** (`test/capability_cage_test.dart`): mirrors the reference
  suites case-for-case (taxonomy incl. fixture, capability, errors,
  glob, manifest), plus every schema-rejection branch, file loading
  (round-trip, missing, non-UTF-8, self-manifest), value semantics, and
  glob edges. 81 tests.
- **Property** (`test/properties_test.dart`): glob laws, `has` model,
  summary consistency, JSON round-trips, check duality, RWS conversion
  — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration campaign over
  hostile JSON (only [ManifestError] escapes; successes re-verify) and
  hostile glob pairs (totality + self-match). Ad-hoc 60,000-iteration
  runs over three more seeds: zero crashes.
- **Coverage**: 100% lines and branches, 97.9% functions (implicit enum
  constructors only).
