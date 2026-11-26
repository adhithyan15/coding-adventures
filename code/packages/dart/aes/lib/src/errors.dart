/// Errors thrown by AES operations.
library;

/// Thrown when a key is not 16, 24, or 32 bytes. The message mirrors
/// the reference text exactly, including the offending length.
final class AesInvalidKeyLength implements Exception {
  final int length;
  const AesInvalidKeyLength(this.length);

  @override
  bool operator ==(Object other) =>
      other is AesInvalidKeyLength && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'AES key must be 16, 24, or 32 bytes; got $length';
}
