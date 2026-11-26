# coding_adventures_read_write_separation (Dart)

Manifest-level read/write separation classifier. The Dart lane of the Rust
`read-write-separation` crate: capabilities are classified by flavor and
trust, manifests are summarized and validated.

```dart
import 'package:coding_adventures_read_write_separation/coding_adventures_read_write_separation.dart';

final caps = [
  Capability(category: 'net', action: 'connect', target: 'imap.example:993')
      .withFlavor(CapabilityFlavor.ingestion),
  Capability(category: 'fs', action: 'write', target: '/tmp/outbox'),
];

validateManifest(caps); // throws RwsViolation: untrusted input + actuation
```

## API

- `Capability` — immutable `category`/`action`/`target` triple with optional
  `flavor`, `trust`, `justification` annotations (`withFlavor`, `withTrust`,
  `withJustification` copy with one field set) and `identifier()`.
- `CapabilityFlavor` (`ingestion`, `actuation`, `internal`) and
  `CapabilityTrust` (`trusted`, `untrusted`) — wire-form enums.
- `classifyCapability(cap)` — resolves explicit annotations, else the
  category/action(/target) default tables (loopback listeners and
  package-internal reads are trusted; `proc` and outbound writes default
  to actuation).
- `summarizeManifest(caps)` — 13-counter shape/risk summary with `isEmpty`,
  `hasRwsRisk`, and `hasSameResourceOverlap`.
- `validateManifest(caps)` — throws `RwsViolation` (carrying the offending
  `untrustedInputs`, `actuations`, and `message`) when untrusted inputs meet
  external actuations or when a same-category read/write pair overlaps
  (exact target match or `prefix*` glob in either direction). The overlap
  message takes precedence when both triggers fire.

One mapping note: the reference returns `Result<(), RwsViolation>`; the
Dart lane throws the violation, which is the idiomatic shape for a
validator. Default tables, overlap and glob rules, dedup behavior, and both
message texts mirror the reference exactly.

## Validation

- **Unit** (`test/read_write_separation_test.dart`): mirrors the reference
  suite case-for-case (13 tests), plus the default flavor/trust tables,
  loopback and package-internal edges, input classification, overlap edges
  (reversed globs, bare `*`, cross-category), message precedence,
  violation-list dedup, risk-flag combinations, and value semantics.
  35 tests.
- **Property** (`test/properties_test.dart`): verdict stability under
  duplication and reversal, summary-counter consistency, classification
  laws, overlap symmetry — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration campaign asserting
  summary invariants, verdict determinism (repeat/reverse/duplicate), and
  totality. Ad-hoc 60,000-iteration runs over three more seeds: zero
  crashes.
- **Coverage**: 100% lines and branches, 95.5% functions (the two misses
  are implicit enum constructors, uncoverable by construction).
