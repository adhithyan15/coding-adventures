/// Errors thrown when Argon2i inputs fail validation.
library;

/// Base type for Argon2i validation errors. Messages mirror the
/// reference texts exactly; numeric payloads mirror its fields.
sealed class Argon2Error implements Exception {
  const Argon2Error();
}

/// Password length must fit in 32 bits.
final class PasswordTooLong extends Argon2Error {
  final int length;
  const PasswordTooLong(this.length);

  @override
  bool operator ==(Object other) =>
      other is PasswordTooLong && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'password length must fit in 32 bits, got $length';
}

/// Salt must be at least 8 bytes.
final class SaltTooShort extends Argon2Error {
  final int length;
  const SaltTooShort(this.length);

  @override
  bool operator ==(Object other) =>
      other is SaltTooShort && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'salt must be at least 8 bytes, got $length';
}

/// Salt length must fit in 32 bits.
final class SaltTooLong extends Argon2Error {
  final int length;
  const SaltTooLong(this.length);

  @override
  bool operator ==(Object other) =>
      other is SaltTooLong && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'salt length must fit in 32 bits, got $length';
}

/// Key length must fit in 32 bits.
final class KeyTooLong extends Argon2Error {
  final int length;
  const KeyTooLong(this.length);

  @override
  bool operator ==(Object other) =>
      other is KeyTooLong && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'key length must fit in 32 bits, got $length';
}

/// Associated-data length must fit in 32 bits.
final class AssociatedDataTooLong extends Argon2Error {
  final int length;
  const AssociatedDataTooLong(this.length);

  @override
  bool operator ==(Object other) =>
      other is AssociatedDataTooLong && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() =>
      'associated data length must fit in 32 bits, got $length';
}

/// Tag length must be at least 4 bytes.
final class TagLengthTooSmall extends Argon2Error {
  final int length;
  const TagLengthTooSmall(this.length);

  @override
  bool operator ==(Object other) =>
      other is TagLengthTooSmall && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'tagLength must be >= 4, got $length';
}

/// Tag length must fit in 32 bits.
final class TagLengthTooLarge extends Argon2Error {
  final int length;
  const TagLengthTooLarge(this.length);

  @override
  bool operator ==(Object other) =>
      other is TagLengthTooLarge && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'tagLength must fit in 32 bits, got $length';
}

/// Parallelism must be in [1, 2^24 - 1].
final class InvalidParallelism extends Argon2Error {
  final int parallelism;
  const InvalidParallelism(this.parallelism);

  @override
  bool operator ==(Object other) =>
      other is InvalidParallelism && parallelism == other.parallelism;
  @override
  int get hashCode => parallelism.hashCode;
  @override
  String toString() => 'parallelism must be in [1, 2^24-1], got $parallelism';
}

/// Memory cost must cover at least 8 blocks per lane.
final class MemoryTooSmall extends Argon2Error {
  final int got;
  final int min;
  const MemoryTooSmall({required this.got, required this.min});

  @override
  bool operator ==(Object other) =>
      other is MemoryTooSmall && got == other.got && min == other.min;
  @override
  int get hashCode => Object.hash(got, min);
  @override
  String toString() => 'memoryCost must be >= 8*parallelism ($min), got $got';
}

/// Time cost must be at least 1 pass.
final class TimeCostZero extends Argon2Error {
  const TimeCostZero();

  @override
  bool operator ==(Object other) => other is TimeCostZero;
  @override
  int get hashCode => 0;
  @override
  String toString() => 'timeCost must be >= 1';
}

/// Only Argon2 v1.3 (0x13) is supported.
final class UnsupportedVersion extends Argon2Error {
  final int version;
  const UnsupportedVersion(this.version);

  @override
  bool operator ==(Object other) =>
      other is UnsupportedVersion && version == other.version;
  @override
  int get hashCode => version.hashCode;
  @override
  String toString() =>
      'only Argon2 v1.3 (0x13) is supported; got 0x${version.toRadixString(16).padLeft(2, '0')}';
}

/// A variable-length hash call failed (wraps the BLAKE2b failure).
final class Argon2Blake2bError extends Argon2Error {
  final String message;
  const Argon2Blake2bError(this.message);

  @override
  bool operator ==(Object other) =>
      other is Argon2Blake2bError && message == other.message;
  @override
  int get hashCode => message.hashCode;
  @override
  String toString() => 'blake2b error: $message';
}
