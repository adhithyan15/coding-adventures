/// Errors thrown by scrypt parameter validation and derivation.
///
/// The reference error enum carries no messages, so these texts are
/// lane-defined in the pbkdf2 lane's style; the variant set and
/// triggering conditions mirror the reference exactly.
library;

/// Base type for scrypt failures.
sealed class ScryptError implements Exception {
  const ScryptError();
}

/// N < 2 or N is not a power of two. The `integerify` index wraps
/// with `j % N`, which needs a power of two for a uniform
/// distribution over the V table.
final class ScryptInvalidN extends ScryptError {
  const ScryptInvalidN();

  @override
  bool operator ==(Object other) => other is ScryptInvalidN;
  @override
  int get hashCode => 0;
  @override
  String toString() => 'scrypt: N must be a power of two >= 2';
}

/// N exceeds the 2^20 cap against accidental out-of-memory use.
final class ScryptNTooLarge extends ScryptError {
  const ScryptNTooLarge();

  @override
  bool operator ==(Object other) => other is ScryptNTooLarge;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'scrypt: N must not exceed 2^20';
}

/// r < 1. Each ROMix block is 128*r bytes; r=0 is nonsensical.
final class ScryptInvalidR extends ScryptError {
  const ScryptInvalidR();

  @override
  bool operator ==(Object other) => other is ScryptInvalidR;
  @override
  int get hashCode => 2;
  @override
  String toString() => 'scrypt: r must be positive';
}

/// p < 1. Zero parallel blocks would produce zero output.
final class ScryptInvalidP extends ScryptError {
  const ScryptInvalidP();

  @override
  bool operator ==(Object other) => other is ScryptInvalidP;
  @override
  int get hashCode => 3;
  @override
  String toString() => 'scrypt: p must be positive';
}

/// dkLen < 1. A zero-length key is meaningless.
final class ScryptInvalidKeyLength extends ScryptError {
  const ScryptInvalidKeyLength();

  @override
  bool operator ==(Object other) => other is ScryptInvalidKeyLength;
  @override
  int get hashCode => 4;
  @override
  String toString() => 'scrypt: dk_len must be positive';
}

/// dkLen exceeds the 2^20 allocation cap.
final class ScryptKeyLengthTooLarge extends ScryptError {
  const ScryptKeyLengthTooLarge();

  @override
  bool operator ==(Object other) => other is ScryptKeyLengthTooLarge;
  @override
  int get hashCode => 5;
  @override
  String toString() => 'scrypt: dk_len must not exceed 2^20';
}

/// p*r exceeds 2^30 (RFC 7914 §2), or the p*128*r working buffer
/// would exceed 2^30 bytes (1 GiB).
final class ScryptPRTooLarge extends ScryptError {
  const ScryptPRTooLarge();

  @override
  bool operator ==(Object other) => other is ScryptPRTooLarge;
  @override
  int get hashCode => 6;
  @override
  String toString() => 'scrypt: p * r must not exceed 2^30';
}

/// An internal PBKDF2 computation failed (wraps any in-lane
/// PBKDF2 error, e.g. when the working buffer exceeds PBKDF2's own
/// 2^20 output cap).
final class ScryptHmacError extends ScryptError {
  const ScryptHmacError();

  @override
  bool operator ==(Object other) => other is ScryptHmacError;
  @override
  int get hashCode => 7;
  @override
  String toString() => 'scrypt: internal PBKDF2 computation failed';
}
