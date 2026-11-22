/// Coverage-guided fuzz engine: the Dart lane of VER06/VER06c.
///
/// Deterministic mutator algebra, 65,536-bit coverage feedback, power
/// scheduling, FNV crash dedup, minimization, closed failure vocabulary, and
/// privacy-preserving TAP reporting. Zero runtime dependencies.
library coding_adventures_fuzz;

export 'src/fuzz.dart';
