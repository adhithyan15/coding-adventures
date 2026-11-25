# coding_adventures_capability_cage (Dart)

Capability manifests, taxonomy, and target matching. The Dart lane of the
Rust `capability-cage` crate (all three slices: taxonomy, manifest
loading and validation, audit envelopes, backends, and the `fs`/`env`
secure wrappers).

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
- Audit envelopes: [OperationDecision]/[OperationRecord] (`allowed`/
  `denied`/`withMessage` (copy), `capabilityKey`), generic
  [Operation] (`allowed`/`denied`/`map`/`recordTo`), and [AuditSink]
  with [NoopAuditSink] and [VecAuditSink] (`records`/`takeRecords`).
- Secure wrappers: `readFile`/`writeFile`/`createFile`/`deleteFile`/
  `listDir` (`fs`) and `readVar`/`writeVar` (`env`). Each checks the
  manifest first (denial throws [CapabilityViolationError] directly —
  Dart has no `io::Result` channel to preserve, so the reference's
  `io::Error` wrapping is not mirrored), then delegates to
  [currentBackend].
- [Backend] seam (async by design — the reference is sync, but async
  is idiomatic `dart:io` and host-channel routing is inherently async):
  [OpenBackend] (`dart:io`), [DenyAllBackend] (exact reference refusal
  messages via [BackendError]), scripted [TestBackend] with
  [TestBackendCall] records, and Zone-scoped [runWithBackend]/
  [runWithBackendAsync] with [currentBackend] (nesting replaces the
  reference's mutex-serialized override guard; Dart has no
  destructors).

Two deliberate notes: the checklist omits the reference's provably
redundant suffix-overlap guard in segment matching (documented with the
proof); display strings quote names plainly where the reference uses
Rust debug escapes (same shape for ordinary names).

Three slice-2 platform mappings: NUL-bearing paths are rejected
fail-closed (`dart:io` silently truncates at NUL while `std::fs`
errors — a truncation/confusion gap if unchecked); `writeEnv` on the
live backend always throws [UnsupportedError] (`dart:io` exposes the
process environment read-only); `listDir` keeps every basename (Dart
path strings are always valid, so the reference's non-Unicode skip
has no equivalent). The package manifest now declares fs
read/write/create/delete/list plus env read; it is intentionally not
RWS-clean (separation is enforced per caller manifest at each wrapped
call, not on the mechanism's own surface).

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
- **Slice 2 unit** (`test/audit_test.dart`, `test/backend_test.dart`):
  mirrors the 4 audit + 6 backend reference tests case-for-case, plus
  zone nesting/restoration/exception-safety, async propagation, a live
  temp-dir round trip, NUL fail-closed on all five fs methods, env
  write unsupported, first-match scripting, snapshot independence,
  and string/equality/hash helpers. 32 tests.
- **Slice 2 property** (new group in `test/properties_test.dart`):
  envelope mapping, denied shape, sink order/drain, zone nesting
  under injected throws, call-record equality laws, message-copy
  semantics — fixed seeds 281–286.
- **Slice 2 async campaign** (`runAsyncCampaign` in
  `test/backend_test.dart`): 2×500 seeded hostile op-scripts over the
  async backend API against an independent model (outputs, call log,
  audit keys); only exact [BackendError] escapes, only from
  [DenyAllBackend]. (The in-lane `fuzz` engine drives sync targets
  only, so the campaign rolls its own deterministic SplitMix64 loop.)
  Ad-hoc 6,000 scripts over two more seeds: clean.
- **Slice 3 unit** (`test/secure_test.dart`): mirrors the 4 + 6
  reference wrapper tests case-for-case, plus wrong-action denial,
  live temp-dir round trips, denial-before-OS-touch, writeVar
  check-then-platform ordering, and the NUL backstop. 15 tests.
- **Slice 3 async campaign** (`runSecureCampaign` in
  `test/secure_test.dart`): 2×500 seeded scripts over random
  manifests × wrappers, asserting verdict-equals-`has`, exact denial
  triples (pinning that each wrapper checks the right
  category/action/target), and backend-log gating (log grows iff
  allowed); only [CapabilityViolationError] escapes.
- **Coverage**: 100% lines, 98.4% functions (the 3 misses are
  implicit enum constructors: `Category`, `Action`,
  `OperationDecision`).
