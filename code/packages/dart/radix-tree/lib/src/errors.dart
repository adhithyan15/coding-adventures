import 'messages.dart';

/// Closed enumeration of radix tree error codes.
enum RadixTreeErrorCode {
  invalidKey(errorIdInvalidKey),
  keyNotFound(errorIdKeyNotFound);

  const RadixTreeErrorCode(this.errorId);

  /// The stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all radix tree operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class RadixTreeException implements Exception {
  const RadixTreeException(this.errorCode, this.message);

  /// The category code for this error.
  final RadixTreeErrorCode errorCode;

  /// The static, payload-blind explanation of the error.
  final String message;

  /// Stable machine-readable identifier matching [errorCode.errorId].
  String get errorId => errorCode.errorId;

  @override
  String toString() => message;
}

/// Thrown when a key violates scalar bounds, contains surrogates, or exceeds length ceiling.
final class InvalidRadixKeyException extends RadixTreeException {
  const InvalidRadixKeyException._(String message)
      : super(RadixTreeErrorCode.invalidKey, message);

  /// Key contains a lone surrogate, reversed surrogates, or out-of-range scalar.
  const InvalidRadixKeyException.invalidScalar() : this._(messageInvalidScalar);

  /// Key exceeds the maximum allowed scalar length ceiling.
  const InvalidRadixKeyException.keyTooLong() : this._(messageKeyTooLong);
}

/// Thrown when `operator []` cannot find the requested key.
final class RadixKeyNotFoundException extends RadixTreeException {
  const RadixKeyNotFoundException()
      : super(RadixTreeErrorCode.keyNotFound, messageKeyNotFound);
}
