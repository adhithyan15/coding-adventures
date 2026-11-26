/// Errors thrown by the fixed-memory ChaCha20 and AEAD boundaries.
library;

/// Base type for fixed ChaCha20 failures. Categories retain no key,
/// nonce, counter, input, output, or length values.
sealed class FixedChaCha20Error implements Exception {
  const FixedChaCha20Error();
}

/// Caller-owned output length differs from the input length.
final class ChaCha20OutputLengthMismatch extends FixedChaCha20Error {
  const ChaCha20OutputLengthMismatch();

  @override
  bool operator ==(Object other) => other is ChaCha20OutputLengthMismatch;
  @override
  int get hashCode => 0;
  @override
  String toString() => 'ChaCha20 output length must equal input length';
}

/// The requested blocks would exceed ChaCha20's 32-bit counter
/// space.
final class ChaCha20CounterExhausted extends FixedChaCha20Error {
  const ChaCha20CounterExhausted();

  @override
  bool operator ==(Object other) => other is ChaCha20CounterExhausted;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'ChaCha20 request exceeds the 32-bit block counter';
}

/// Base type for fixed AEAD failures. Categories retain no key,
/// nonce, plaintext, ciphertext, tag, AAD, output, or
/// observed-length values.
sealed class FixedAeadError implements Exception {
  const FixedAeadError();
}

/// Caller-owned output length differs from the input length.
final class AeadOutputLengthMismatch extends FixedAeadError {
  const AeadOutputLengthMismatch();

  @override
  bool operator ==(Object other) => other is AeadOutputLengthMismatch;
  @override
  int get hashCode => 2;
  @override
  String toString() => 'AEAD output length must equal input length';
}

/// An input length cannot be represented by RFC 8439's 64-bit
/// encoding.
final class AeadLengthOverflow extends FixedAeadError {
  const AeadLengthOverflow();

  @override
  bool operator ==(Object other) => other is AeadLengthOverflow;
  @override
  int get hashCode => 3;
  @override
  String toString() => 'AEAD input length exceeds the RFC 8439 encoding';
}

/// The payload would exceed the ChaCha20 counter space.
final class AeadCounterExhausted extends FixedAeadError {
  const AeadCounterExhausted();

  @override
  bool operator ==(Object other) => other is AeadCounterExhausted;
  @override
  int get hashCode => 4;
  @override
  String toString() => 'AEAD request exceeds the ChaCha20 block counter';
}

/// The supplied authentication tag did not match. No plaintext is
/// released.
final class AeadAuthenticationFailed extends FixedAeadError {
  const AeadAuthenticationFailed();

  @override
  bool operator ==(Object other) => other is AeadAuthenticationFailed;
  @override
  int get hashCode => 5;
  @override
  String toString() => 'AEAD authentication failed';
}
