# Dart hash-functions

Pure Dart implementations of the DT17 byte hashes, UTF-8 helpers, strategy
wrappers, and deterministic caller-sample analysis helpers.

```dart
import 'package:coding_adventures_hash_functions/coding_adventures_hash_functions.dart';

final bytes = <int>[0x61, 0x62, 0x63];
final smallHash = fnv1a32(bytes); // 440920331
final largeHash = fnv1a64(bytes); // unsigned 64-bit BigInt
final polynomial = polynomialRolling(bytes);

final key = List<int>.generate(16, (index) => index);
final sipHash = SipHash24(key);
final keyedHash = sipHash.hash(bytes);
```

Byte functions accept `List<int>` values from 0 through 255. The 32-bit
functions return nonnegative `int` values. The 64-bit functions return
unsigned `BigInt` values, so results remain exact when compiled to JavaScript.
Parameterized polynomial hashing requires `BigInt` base and modulus values
and evaluates the exact multiply-add before reducing it. String helpers encode
valid Unicode scalar sequences as UTF-8 and reject unpaired surrogates.

The `Fnv1a32`, `Fnv1a64`, `Djb2`, `Murmur3_32`, `PolynomialRolling`, and
`SipHash24` wrappers expose `hash` and `outputBits`. `SipHash24` requires a
caller-supplied 16-byte key and snapshots it at construction. Analysis helpers
use explicit nonempty samples and a `BigInt Function(List<int>)` callback.
They use no random source, clock, or other host capability.

Run `BUILD` or `BUILD_windows` from this directory. The front generates a
test-only snapshot from `code/specs/fixtures/hash-functions-v1/cases.json`,
executes the same public fixture consumer on Dart VM and Node, measures actual
library line and branch coverage separately, and runs source mutation
controls. Snapshot generation and test fixture reads remain outside the
production library.
