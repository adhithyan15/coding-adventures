/// Errors thrown by HKDF operations.
library;

/// Base type for HKDF failures. Messages mirror the reference texts
/// exactly; numeric payloads mirror its fields.
sealed class HkdfError implements Exception {
  const HkdfError();
}

/// The requested output length exceeds 255 × HashLen: Expand's
/// single-byte counter (0x01..0xFF) can produce at most 255 blocks.
final class HkdfOutputTooLong extends HkdfError {
  final int requested;
  final int maximum;
  const HkdfOutputTooLong({required this.requested, required this.maximum});

  @override
  bool operator ==(Object other) =>
      other is HkdfOutputTooLong &&
      requested == other.requested &&
      maximum == other.maximum;
  @override
  int get hashCode => Object.hash(requested, maximum);
  @override
  String toString() =>
      'HKDF output length $requested exceeds maximum $maximum (255 * HashLen)';
}

/// The requested output length is zero.
final class HkdfOutputTooShort extends HkdfError {
  const HkdfOutputTooShort();

  @override
  bool operator ==(Object other) => other is HkdfOutputTooShort;
  @override
  int get hashCode => 0;
  @override
  String toString() => 'HKDF output length must be positive';
}

/// A fixed-output Extract or Expand input exceeds the selected
/// hash's encoded message-length bound.
final class HkdfInputTooLong extends HkdfError {
  const HkdfInputTooLong();

  @override
  bool operator ==(Object other) => other is HkdfInputTooLong;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'HKDF input exceeds the selected hash length bound';
}

/// The fixed-output PRF rejected its key. Public Extract inputs
/// cannot cause this (an omitted salt becomes HashLen zero bytes);
/// the category preserves the underlying invariant without panicking.
final class HkdfPrfRejectedKey extends HkdfError {
  const HkdfPrfRejectedKey();

  @override
  bool operator ==(Object other) => other is HkdfPrfRejectedKey;
  @override
  int get hashCode => 2;
  @override
  String toString() => 'HKDF pseudorandom function rejected its key';
}

/// Thrown when [HkdfPrk.importExact] receives anything but exactly
/// the selected hash's output length. The message deliberately omits
/// the caller's length and bytes.
final class HkdfPrkImportError implements Exception {
  const HkdfPrkImportError();

  @override
  bool operator ==(Object other) => other is HkdfPrkImportError;
  @override
  int get hashCode => 3;
  @override
  String toString() =>
      'HKDF pseudorandom key must exactly match the selected hash length';
}
