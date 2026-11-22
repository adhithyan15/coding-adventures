# coding_adventures_pbt (Dart)

Property-based testing engine: the Dart lane of
[VER05b](../../../specs/VER05b-dart-pbt-engine.md).

Instead of hand-writing examples, describe the shape of valid inputs with a
`Gen<T>` and assert a property every output must satisfy. The engine draws
deterministic candidates from a seeded `SplitMix64` stream, and when one
falsifies the property it shrinks it to the smallest witness that still fails.

## API

```dart
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';

// Generators
Gen.boolean(); Gen.u8(); Gen.u64();              // primitives
Gen.integer(BigInt.zero, BigInt.from(100));      // bounded BigInt
Gen.bytes(0, 64);                                // Uint8List
Gen.list(0, 10, Gen.u8());                       // lists
Gen.constant(v); Gen.oneOf([a, b]);              // choice
Gen.tuple2(a, b); Gen.tuple3(a, b, c);           // records
gen.map(f); gen.filter(p); gen.flatMap(f);       // combinators

// Running properties
assertProperty(gen, (v) => ..., PbtOptions(numTests: 200, seed: BigInt.from(7)));
Verdict v = checkProperty(seed, options, (rng) => gen.sample(rng), prop, shrink);

// Shrinking
(BigInt, int) shrinkInt(initial, target, min, max, prop, [maxSteps]);
(List<BigInt>, int) shrinkList(initial, elemTarget, prop, [maxSteps]);
(Uint8List, int) shrinkBytes(initial, prop, [maxSteps]);
```

* All 64-bit state is `BigInt`, exact on both the VM and JavaScript: the same
  seed yields the same stream on every platform, matching the sibling lanes.
* Failure ids are the closed VER05 vocabulary: `property-counterexample`,
  `property-exhausted`, `property-timeout`, `property-panic`.
* On CI (`isCI: true` or `--define=CI=true`) counterexamples are redacted to
  an FNV-1a repro digest; off CI the message carries both witnesses.
* Zero runtime dependencies. Pure computation: no clock, no I/O, no
  environment reads (CI detection uses a compile-time define).

## Tests

```bash
dart pub get
dart test
```

The suite replays all 25 cases of the shared pbt-v1 corpus
(`code/specs/fixtures/pbt-v1/cases.json`, read relative to the package
directory with `dart:io`, so the tests run on the VM) — exact generator
outputs per seed, exact shrink minimals *and step counts*, replay
consistency, filter exhaustion, failure-id validation, and corpus rejection —
plus generator-algebra, shrinking-edge, and error-handling groups. Coverage
gate: 95% branch and function.
