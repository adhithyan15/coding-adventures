# coding_adventures_fuzz (Dart)

Coverage-guided fuzz engine: the Dart lane of
[VER06c](../../../specs/VER06c-dart-fuzz-engine.md).

The engine mutates raw byte buffers, feeds them to a target, and keeps the
inputs that cover new branches — evolving a corpus toward crashes. Every
random choice derives from a seeded `SplitMix64` stream, so a campaign
replays deterministically.

## API

```dart
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';

// Mutators (all copy-on-write, all total over every input)
bitFlip(data, bitOffset, [count]);       byteFlip(data, byteOffset, [count]);
byteArithmetic(data, off, delta, [w], [endian]);
boundaryInsert(data, off, width, val, [endian]);
blockDelete(data, off, len);             blockInsert(data, off, bytes);
blockDuplicate(data, src, len, dst);     splice(target, donor, tSplit, dSplit);
mutateSequence(data, seed, steps);       // PRNG-driven compound

// Coverage feedback
CoverageBitmap().recordEdge(e); hasEdge(e); admitTrace(t); count(); merge(o);
(bool, int) corpusAdmit(existing, trace);
int calculateEnergy(base, seedLen, newEdges, totalEdges);

// Crashes
String crashSignature(kind, frames);     bool isDuplicateCrash(a, b);
Uint8List minimizeCrashingInput(input, sig, testFn);
String formatTapFailure(num, target, fid, sig);  // opaque, no payload bytes

// Campaigns
FuzzReport fuzzTarget(corpus, FuzzOptions(...), (input, cov) => outcome);
```

* 8-byte integer access is assembled byte-wise over `BigInt`, exact on both
  the VM and JavaScript (no int boxing past 2^53).
* Failure ids are the closed VER06 vocabulary: `fuzz-crash`, `fuzz-timeout`,
  `fuzz-oom`, `fuzz-leak`, `fuzz-panic`.
* Zero runtime dependencies. Pure computation: no clock, no I/O.

## Tests

```bash
dart pub get
dart test
```

The suite replays all 25 cases of the shared fuzz-v1 corpus
(`code/specs/fixtures/fuzz-v1/cases.json`, read relative to the package
directory with `dart:io`, so the tests run on the VM) — exact mutator
outputs, sequence determinism, admission verdicts, dedup verdicts, opaque
TAP lines, and rejection — plus bitmap, energy, minimization, campaign,
and edge-case groups. Coverage gate: 95% branch and function.
