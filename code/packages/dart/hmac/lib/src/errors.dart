/// Errors thrown by HMAC variants on invalid keys or overlong input.
library;

/// Base type for HMAC failures. Messages mirror the reference texts
/// exactly and carry no input bytes.
sealed class HmacError implements Exception {
  const HmacError();
}

/// The key was empty. HMAC requires a non-empty key; an empty key
/// almost always indicates a misconfiguration (e.g. an unset
/// environment variable).
final class HmacEmptyKey extends HmacError {
  const HmacEmptyKey();

  @override
  bool operator ==(Object other) => other is HmacEmptyKey;
  @override
  int get hashCode => 0;
  @override
  String toString() => 'HMAC key must not be empty';
}

/// A fixed-output HMAC input would exceed its FIPS 180-4 encoded
/// message-length field.
final class HmacInputTooLong extends HmacError {
  const HmacInputTooLong();

  @override
  bool operator ==(Object other) => other is HmacInputTooLong;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'HMAC input is too long';
}
