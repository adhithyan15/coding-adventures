/// Provenance log for tracked entities: origins, contributions,
/// tombstones, entries, and the [CVLog] that accumulates them.
///
/// Slice 1 covers the in-memory log (mint, mutate, query, checked
/// limits). Serialization, graph validation, checked export, and the
/// journal land in slice 2.
library;

export 'src/limits.dart';
export 'src/log.dart';
export 'src/types.dart';
