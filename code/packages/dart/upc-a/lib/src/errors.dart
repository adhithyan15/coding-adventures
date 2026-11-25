import 'messages.dart';

/// Closed enumeration of valid UPC-A validation error codes in Dart.
enum UpcAErrorCode {
  inputTooLong(upcAErrorInputTooLong),
  invalidLength(upcAErrorInvalidLength),
  invalidCharacter(upcAErrorInvalidCharacter),
  invalidCheckDigit(upcAErrorInvalidCheckDigit);

  final String errorId;
  const UpcAErrorCode(this.errorId);
}

/// Sealed base error type for UPC-A operations, preventing unauthorized
/// subclassing.
sealed class UpcAError extends ArgumentError {
  UpcAError(super.message);
}

/// Thrown when UPC-A input fails validation against the symbology
/// specification.
final class InvalidUpcAInputError extends UpcAError {
  final UpcAErrorCode errorCode;

  InvalidUpcAInputError(this.errorCode) : super(upcAMessageFor(errorCode));

  String get errorId => errorCode.errorId;
}
