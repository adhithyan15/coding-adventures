/// Pseudorandom number generator library: LCG, Xorshift64, and PCG32.
///
/// Import this file to use any of the three generators:
///
/// ```dart
/// import 'package:coding_adventures_rng/coding_adventures_rng.dart';
///
/// final rng = Pcg32(42);
/// print(rng.nextU32());
/// print(rng.nextFloat());
/// print(rng.nextIntInRange(1, 6));
/// ```
library coding_adventures_rng;

export 'src/rng.dart';
