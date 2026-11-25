/// Errors thrown by PBKDF2 operations.
library;

/// Base type for PBKDF2 failures. Messages mirror the reference
/// texts exactly.
sealed class Pbkdf2Error implements Exception {
  const Pbkdf2Error();
}

/// The password is empty and empty passwords were not allowed. An
/// empty password provides no entropy.
final class Pbkdf2EmptyPassword extends Pbkdf2Error {
  const Pbkdf2EmptyPassword();

  @override
  bool operator ==(Object other) => other is Pbkdf2EmptyPassword;
  @override
  int get hashCode => 0;
  @override
  String toString() => 'pbkdf2: password must not be empty';
}

/// The iteration count is zero (or negative, which the reference's
/// unsigned count cannot express). Must be ≥ 1.
final class Pbkdf2InvalidIterations extends Pbkdf2Error {
  const Pbkdf2InvalidIterations();

  @override
  bool operator ==(Object other) => other is Pbkdf2InvalidIterations;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'pbkdf2: iterations must be positive';
}

/// The key length is zero (or negative, likewise inexpressible in
/// the reference). Must be positive.
final class Pbkdf2InvalidKeyLength extends Pbkdf2Error {
  const Pbkdf2InvalidKeyLength();

  @override
  bool operator ==(Object other) => other is Pbkdf2InvalidKeyLength;
  @override
  int get hashCode => 2;
  @override
  String toString() => 'pbkdf2: key_length must be positive';
}

/// The key length exceeds the practical 2^20 (1 MiB) bound. RFC 8018
/// §5.2 imposes (2^32−1)×hLen as the hard cap; the tighter bound
/// prevents memory-exhaustion DoS and integer overflow in block
/// counting.
final class Pbkdf2KeyLengthTooLarge extends Pbkdf2Error {
  const Pbkdf2KeyLengthTooLarge();

  @override
  bool operator ==(Object other) => other is Pbkdf2KeyLengthTooLarge;
  @override
  int get hashCode => 3;
  @override
  String toString() => 'pbkdf2: key_length must not exceed 2^20 (1 MiB)';
}

/// The underlying PRF failed. Unreachable when the password is
/// non-empty (validated before the PRF runs); preserved rather than
/// panicking in case the HMAC lane ever changes.
final class Pbkdf2PrfError extends Pbkdf2Error {
  const Pbkdf2PrfError();

  @override
  bool operator ==(Object other) => other is Pbkdf2PrfError;
  @override
  int get hashCode => 4;
  @override
  String toString() => 'pbkdf2: pseudorandom function returned an error';
}
