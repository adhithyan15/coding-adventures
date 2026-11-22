# coding_adventures_uuid (Dart)

Universally Unique Identifiers (RFC 4122 / RFC 9562): the Dart lane of
[uuid.md](../../../specs/uuid.md).

A UUID identifies information without a central authority. This package
generates all five versions, parses every common string form, and compares and
formats values — with no dependency beyond the in-lane MD5 and SHA-1 packages.

## API

```dart
import 'package:coding_adventures_uuid/coding_adventures_uuid.dart';

// Generation
Uuid v1()                                // time-based, random node
Uuid v3(Uuid namespace, String name)     // name-based, MD5 (deterministic)
Uuid v4()                                // random
Uuid v5(Uuid namespace, String name)     // name-based, SHA-1 (deterministic)
Uuid v7()                                // time-ordered random (RFC 9562)

// Parsing and validation
Uuid Uuid.parse(String text)             // standard, compact, braced, URN
bool Uuid.isValid(String text)

// Inspection
int version; String variant;             // e.g. 4, "rfc4122"
bool isNil; bool isMax;
Uint8List bytes;                         // copy of the 16 raw bytes
BigInt toBigInt();                       // 128-bit big-endian value

// Formatting
String toString();                       // 8-4-4-4-12 lowercase
String toHex();                          // 32 lowercase digits
String toUrn();                          // urn:uuid:...

// Constants
Uuid.namespaceDns, Uuid.namespaceUrl, Uuid.namespaceOid, Uuid.namespaceX500;
Uuid.nilUuid, Uuid.maxUuid;
```

* `Uuid` is a value type: bytes are copied on the way in and out, equality and
  `Comparable` ordering work on the raw bytes, and instances are map keys.
* `parse` is case-insensitive, trims whitespace, and accepts hyphenated,
  compact, `{braced}`, and `urn:uuid:` forms. It throws `UuidError` on invalid
  input; `isValid` is the boolean twin.
* v3 and v5 names are UTF-8 encoded before hashing, so Unicode names hash the
  same in every lane.
* v1 uses a random node ID (never a MAC address) with the multicast bit set,
  and a freshly random clock sequence per call, matching the reference lane.
* Dart reports microsecond time, so v1 derives its 100-nanosecond count as
  `microsecondsSinceEpoch * 10`.

## What this does and does not guarantee

Generation draws OS entropy through `Random.secure()` and wall-clock time
through `DateTime.now()`; the package manifest declares `rand:read` and
`time:read` for exactly those uses. Name-based versions are pure computation.

Like the reference lane, v7 guarantees only that the 48-bit timestamp portion
never goes backward — same-millisecond UUIDs order by their random suffix, so
full byte monotonicity needs an application sequence counter (RFC 9562
section 6.2), which this port omits for simplicity.

## Tests

```bash
dart pub get
dart test
```

The suite replays all 378 rows of the shared uuid-v1 corpus
(`code/specs/fixtures/uuid-v1/cases.json`, read relative to the package
directory with `dart:io`, so the tests run on the VM), mirrors the reference
lane's unit tests, pins the RFC v3/v5 vectors plus an independently-oracled
Unicode vector, sweeps seeded hostile-shaped inputs deterministically, and runs
negative controls (tampered vectors, avalanche checks, and hostile parse
batteries where `parse` and `isValid` must agree). Coverage gate: 95% branch
and function.
