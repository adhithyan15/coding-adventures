// Internal helpers (not exported from the package barrel).
//
// Threat model: a caller may hand us a `List` subclass whose `length` and `[]`
// lie, change between reads, or are enormous. Every public entry point that
// accepts a caller list therefore:
//
//   1. reads `length` exactly once,
//   2. rejects an over-cap or negative length BEFORE any allocation or shift,
//   3. copies each element exactly once into a fresh list,
//   4. validates and computes only from that private copy.
//
// Error messages are fixed text plus ints we computed ourselves; caller objects
// are never interpolated (that would run caller `toString`).

import 'gates.dart';

/// Rust-parity resource caps.
const int maxInputs = 65536;
const int maxSelectBits = 16;
const int maxCounterWidth = 64;

/// Copies [source] (at most [max] elements) reading each index exactly once.
List<T> copyBounded<T>(
  List<T> source,
  String name, {
  int max = maxInputs,
  int min = 0,
  String? shape,
}) {
  final length = source.length; // the only read of caller `length`
  if (length < 0 || length > max) {
    throw ArgumentError.value(length, '$name.length', 'must be <= $max');
  }
  if (length < min) {
    throw ArgumentError.value(
      length,
      '$name.length',
      shape ?? 'must be >= $min',
    );
  }
  final out = <T>[];
  for (var i = 0; i < length; i++) {
    out.add(source[i]); // each caller index read exactly once
  }
  return List<T>.unmodifiable(out);
}

/// Snapshots and validates a caller bit list; the result holds canonical 0/1.
List<Bit> snapshotBits(
  List<Bit> source,
  String name, {
  int max = maxInputs,
  int min = 0,
  String? shape,
}) {
  final raw = copyBounded<Object?>(
    source,
    name,
    max: max,
    min: min,
    shape: shape,
  );
  final out = List<Bit>.filled(raw.length, 0);
  for (var i = 0; i < raw.length; i++) {
    final v = raw[i];
    if (v is! int || (v != 0 && v != 1)) {
      throw InvalidBitError(v, '$name[$i]');
    }
    out[i] = v == 1 ? 1 : 0;
  }
  return out;
}
