# coding_adventures_rng (Dart)

Three classic pseudorandom number generators implemented in Dart 3.

See `code/specs/rng.md` for the full specification.

## Algorithms

| Class | Algorithm | Period | Quality |
|---|---|---|---|
| `Lcg` | Linear Congruential Generator (Knuth 1948) | 2^64 | Moderate — fast, full period, some statistical correlations |
| `Xorshift64` | Xorshift (Marsaglia 2003) | 2^64 − 1 | Good — no multiplication, no correlations in lower bits |
| `Pcg32` | Permuted Congruential Generator (O'Neill 2014) | 2^64 | Excellent — passes TestU01 BigCrush and PractRand |

All three use identical Knuth constants:
- Multiplier: `6364136223846793005`
- Increment: `1442695040888963407`

## Dart integer note

Dart integers are arbitrary-precision on the VM and do not wrap on overflow.
Every state update is explicitly masked with `& 0xFFFFFFFFFFFFFFFF` (64-bit)
or `& 0xFFFFFFFF` (32-bit) to replicate the wrap-around behaviour of hardware
registers.

## API

Every generator exposes five methods:

```dart
int    nextU32()                      // uniform int in [0, 2^32)
int    nextU64()                      // uniform int in [0, 2^64)
double nextFloat()                    // uniform double in [0.0, 1.0)
int    nextIntInRange(int min, int max)  // uniform int in [min, max]
```

## Usage

```dart
import 'package:coding_adventures_rng/coding_adventures_rng.dart';

// LCG — simplest, fastest
final lcg = Lcg(42);
final roll = lcg.nextU32();
final dice = lcg.nextIntInRange(1, 6);

// Xorshift64 — good quality, no multiplication
final xor = Xorshift64(42);
final prob = xor.nextFloat();

// PCG32 — best quality, still fast
final pcg = Pcg32(42);
final big = pcg.nextU64();
```

## Reference values (seed = 1)

| Generator | Output 1 | Output 2 | Output 3 |
|---|---|---|---|
| `Lcg` | 1817669548 | 2187888307 | 2784682393 |
| `Xorshift64` | 1082269761 | 201397313 | 1854285353 |
| `Pcg32` | 1412771199 | 1791099446 | 124312908 |

## Development

```bash
cd code/packages/dart/rng
dart pub get
dart test
```

Or use the repo build tool:

```bash
bash BUILD
```
