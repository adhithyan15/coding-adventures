/// Errors thrown by AES modes of operation.
library;

/// Base type for compatibility-mode failures. Messages mirror the
/// reference texts exactly. Key-length failures propagate from the
/// AES lane as [AesInvalidKeyLength], matching the reference's `?`.
sealed class AesModesError implements Exception {
  const AesModesError();
}

/// Padded input is empty or not a multiple of 16 bytes.
final class InvalidPaddedLength extends AesModesError {
  const InvalidPaddedLength();

  @override
  bool operator ==(Object other) => other is InvalidPaddedLength;
  @override
  int get hashCode => 0;
  @override
  String toString() =>
      'Invalid padded data: length must be a positive multiple of 16';
}

/// PKCS#7 pad value out of range, or pad bytes inconsistent.
final class InvalidPadding extends AesModesError {
  const InvalidPadding();

  @override
  bool operator ==(Object other) => other is InvalidPadding;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'Invalid PKCS#7 padding';
}

/// ECB ciphertext is empty or not a multiple of 16 bytes.
final class EcbInvalidCiphertext extends AesModesError {
  const EcbInvalidCiphertext();

  @override
  bool operator ==(Object other) => other is EcbInvalidCiphertext;
  @override
  int get hashCode => 2;
  @override
  String toString() =>
      'ECB ciphertext must be a non-empty multiple of 16 bytes';
}

/// CBC IV is not 16 bytes.
final class CbcInvalidIv extends AesModesError {
  final int length;
  const CbcInvalidIv(this.length);

  @override
  bool operator ==(Object other) =>
      other is CbcInvalidIv && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'CBC IV must be 16 bytes, got $length';
}

/// CBC ciphertext is empty or not a multiple of 16 bytes.
final class CbcInvalidCiphertext extends AesModesError {
  const CbcInvalidCiphertext();

  @override
  bool operator ==(Object other) => other is CbcInvalidCiphertext;
  @override
  int get hashCode => 3;
  @override
  String toString() =>
      'CBC ciphertext must be a non-empty multiple of 16 bytes';
}

/// CTR nonce is not 12 bytes.
final class CtrInvalidNonce extends AesModesError {
  final int length;
  const CtrInvalidNonce(this.length);

  @override
  bool operator ==(Object other) =>
      other is CtrInvalidNonce && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'CTR nonce must be 12 bytes, got $length';
}

/// GCM IV is not 12 bytes.
final class GcmInvalidIv extends AesModesError {
  final int length;
  const GcmInvalidIv(this.length);

  @override
  bool operator ==(Object other) =>
      other is GcmInvalidIv && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'GCM IV must be 12 bytes, got $length';
}

/// GCM authentication tag did not match. No plaintext is released.
final class GcmAuthenticationFailed extends AesModesError {
  const GcmAuthenticationFailed();

  @override
  bool operator ==(Object other) => other is GcmAuthenticationFailed;
  @override
  int get hashCode => 4;
  @override
  String toString() => 'GCM authentication failed: tag mismatch';
}

/// A GHASH input cannot be represented by GCM's 64-bit bit-length
/// field.
final class GhashInputTooLong implements Exception {
  const GhashInputTooLong();

  @override
  bool operator ==(Object other) => other is GhashInputTooLong;
  @override
  int get hashCode => 5;
  @override
  String toString() => 'GHASH input exceeds the supported length';
}

/// Base type for fixed-memory AES-128-GCM failures. Categories
/// retain no key, nonce, plaintext, ciphertext, tag, AAD, output,
/// or observed-length values.
sealed class FixedAes128GcmError implements Exception {
  const FixedAes128GcmError();
}

/// Caller-owned output length differs from the input length.
final class GcmOutputLengthMismatch extends FixedAes128GcmError {
  const GcmOutputLengthMismatch();

  @override
  bool operator ==(Object other) => other is GcmOutputLengthMismatch;
  @override
  int get hashCode => 6;
  @override
  String toString() => 'AES-128-GCM output length must equal input length';
}

/// An input length cannot be represented by GCM's 64-bit
/// bit-length field.
final class GcmLengthOverflow extends FixedAes128GcmError {
  const GcmLengthOverflow();

  @override
  bool operator ==(Object other) => other is GcmLengthOverflow;
  @override
  int get hashCode => 7;
  @override
  String toString() => 'AES-128-GCM input length exceeds the GCM encoding';
}

/// The payload would exceed GCM's 32-bit counter domain.
final class GcmCounterExhausted extends FixedAes128GcmError {
  const GcmCounterExhausted();

  @override
  bool operator ==(Object other) => other is GcmCounterExhausted;
  @override
  int get hashCode => 8;
  @override
  String toString() => 'AES-128-GCM request exceeds the 32-bit block counter';
}

/// The supplied authentication tag did not match. No plaintext is
/// released, and the output buffer is left unchanged.
final class GcmFixedAuthenticationFailed extends FixedAes128GcmError {
  const GcmFixedAuthenticationFailed();

  @override
  bool operator ==(Object other) => other is GcmFixedAuthenticationFailed;
  @override
  int get hashCode => 9;
  @override
  String toString() => 'AES-128-GCM authentication failed';
}
