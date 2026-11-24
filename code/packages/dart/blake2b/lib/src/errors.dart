/// Errors thrown when BLAKE2b options fail validation.
library;

/// Base type for BLAKE2b option errors. Messages mirror the reference
/// texts exactly.
sealed class Blake2bError implements Exception {
  const Blake2bError();
}

/// `digestSize` must be in 1..64. Carries the offending value.
final class InvalidDigestSize extends Blake2bError {
  /// The rejected digest size.
  final int size;

  /// Creates the error.
  const InvalidDigestSize(this.size);

  @override
  bool operator ==(Object other) =>
      other is InvalidDigestSize && size == other.size;

  @override
  int get hashCode => size.hashCode;

  @override
  String toString() => 'digest_size must be in [1, 64], got $size';
}

/// `key` length must be in 0..64. Carries the offending length.
final class KeyTooLong extends Blake2bError {
  /// The rejected key length.
  final int length;

  /// Creates the error.
  const KeyTooLong(this.length);

  @override
  bool operator ==(Object other) =>
      other is KeyTooLong && length == other.length;

  @override
  int get hashCode => length.hashCode;

  @override
  String toString() => 'key length must be in [0, 64], got $length';
}

/// `salt` must be empty or exactly 16 bytes. Carries the bad length.
final class InvalidSaltLength extends Blake2bError {
  /// The rejected salt length.
  final int length;

  /// Creates the error.
  const InvalidSaltLength(this.length);

  @override
  bool operator ==(Object other) =>
      other is InvalidSaltLength && length == other.length;

  @override
  int get hashCode => length.hashCode;

  @override
  String toString() => 'salt must be exactly 16 bytes (or empty), got $length';
}

/// `personal` must be empty or exactly 16 bytes. Carries the bad length.
final class InvalidPersonalLength extends Blake2bError {
  /// The rejected personalization length.
  final int length;

  /// Creates the error.
  const InvalidPersonalLength(this.length);

  @override
  bool operator ==(Object other) =>
      other is InvalidPersonalLength && length == other.length;

  @override
  int get hashCode => length.hashCode;

  @override
  String toString() =>
      'personal must be exactly 16 bytes (or empty), got $length';
}
