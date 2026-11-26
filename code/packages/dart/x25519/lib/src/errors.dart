/// Errors thrown by X25519 on low-order inputs.
library;

/// Thrown when the Montgomery ladder produces the all-zeros output,
/// indicating a low-order point input. The message mirrors the
/// reference text exactly.
final class X25519LowOrderPoint implements Exception {
  const X25519LowOrderPoint();

  @override
  bool operator ==(Object other) => other is X25519LowOrderPoint;
  @override
  int get hashCode => 0;
  @override
  String toString() => 'X25519 produced the all-zeros output (low-order point)';
}
