# coding_adventures_rng (Dart)

Pseudorandom number generator library implementing LCG, Xorshift64, and PCG32.

See `code/specs/rng.md` for the full specification.

## Usage

```dart
import 'package:coding_adventures_rng/coding_adventures_rng.dart';

final rng = Pcg32(42);
final value = rng.nextU32();
final f = rng.nextFloat();
final roll = rng.nextIntInRange(1, 6);
```

## Development

```bash
bash BUILD
```
