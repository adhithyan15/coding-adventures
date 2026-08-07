# coding_adventures_bitset

A compact, growable bitset implemented in pure Dart. Boolean values are packed
into 64-bit `BigInt` words, giving deterministic behavior on the Dart VM and
JavaScript targets without native bindings.

## Features

- zero-filled construction with capacity rounded to 64-bit words
- construction from `int`, arbitrary-width `BigInt`, or binary strings
- auto-growing `set` and `toggle`; non-growing `test` and `clear`
- immutable AND, OR, XOR, NOT, and AND-NOT bulk operations
- ascending iteration over set-bit indices
- popcount, `hasAny`/all/none queries, equality, hashing, and conversions

## Usage

```dart
import 'package:coding_adventures_bitset/coding_adventures_bitset.dart';

final bits = Bitset(100)
  ..set(0)
  ..set(42)
  ..set(99);

print(bits.popcount);       // 3
print(bits.test(42));       // true
print(bits.setBits.toList()); // [0, 42, 99]

final mask = Bitset.fromInteger(42);
final complement = ~mask;
final roundTrip = Bitset.fromBinaryString(mask.toBinaryString());
```

## Development

```text
dart pub get
dart format --output=none --set-exit-if-changed .
dart analyze
dart test
```
