import 'messages.dart';

/// Closed enumeration of valid EAN-13 validation error codes in Dart.
enum Ean13ErrorCode {
  inputTooLong(ean13ErrorInputTooLong),
  invalidLength(ean13ErrorInvalidLength),
  invalidCharacter(ean13ErrorInvalidCharacter),
  invalidCheckDigit(ean13ErrorInvalidCheckDigit);

  final String errorId;
  const Ean13ErrorCode(this.errorId);
}

/// Sealed base error type for EAN-13 operations, preventing unauthorized
/// subclassing.
sealed class Ean13Error extends ArgumentError {
  Ean13Error(super.message);
}

/// Thrown when EAN-13 input fails validation against the symbology
/// specification.
final class InvalidEan13InputError extends Ean13Error {
  final Ean13ErrorCode errorCode;

  InvalidEan13InputError(this.errorCode) : super(ean13MessageFor(errorCode));

  String get errorId => errorCode.errorId;
}
